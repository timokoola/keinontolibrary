//! The runtime overlay store: an append-only JSONL file of admin-supplied forms,
//! consulted **before** the packed artifact and the rule fallback.
//!
//! The overlay lets operators add new lemmas or correct individual forms without
//! rebuilding the artifact. It is cheaply clonable (shared via an `Arc`) and uses interior
//! mutability so the same overlay can live inside an [`Engine`] while admin handlers append
//! to it.

use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};

use keinontolibrary_core::{
    normalize, Case, FormStore, Forms, MemoryStore, Number, ParadigmRef, Source,
};
use serde::{Deserialize, Serialize};

/// One overlay record: the forms for a single `(lemma, paradigm, number, case)` slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayEntry {
    /// Lemma (normalized on apply).
    pub lemma: String,
    /// Declension class.
    pub tn: u8,
    /// Homonym number, if disambiguating.
    #[serde(default)]
    pub hn: Option<u8>,
    /// Gradation letter, if any.
    #[serde(default)]
    pub av: Option<char>,
    /// Grammatical number.
    pub number: Number,
    /// Case.
    pub case: Case,
    /// Surface forms, primary first.
    pub variants: Vec<String>,
}

impl OverlayEntry {
    /// Validate an entry before it is persisted/applied: a real declension class, at
    /// least one non-empty variant, and a non-empty lemma.
    ///
    /// # Errors
    /// Returns a message describing the first problem found.
    pub fn validate(&self) -> Result<(), String> {
        if self.lemma.trim().is_empty() {
            return Err("empty lemma".into());
        }
        // 1–49 regular, 50/51 compounds, 101 pronouns (mirrors the ingest's in-scope set).
        if !(matches!(self.tn, 1..=51) || self.tn == 101) {
            return Err(format!("tn {} out of range", self.tn));
        }
        if self.variants.is_empty() {
            return Err("no variants".into());
        }
        if self.variants.iter().any(|v| v.trim().is_empty()) {
            return Err("empty variant".into());
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Inner {
    store: RwLock<MemoryStore>,
    path: PathBuf,
}

/// A shared, persistent overlay store.
#[derive(Debug, Clone)]
pub struct Overlay {
    inner: Arc<Inner>,
}

impl Overlay {
    /// Open (or create) the overlay backed by `path`, replaying any existing entries.
    ///
    /// # Errors
    /// Returns an error if the file exists but cannot be read.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut store = MemoryStore::new();
        if path.exists() {
            let file = std::fs::File::open(&path)?;
            for (i, line) in io::BufReader::new(file).lines().enumerate() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<OverlayEntry>(&line) {
                    Ok(entry) => match entry.validate() {
                        Ok(()) => apply(&mut store, &entry),
                        Err(e) => eprintln!(
                            "overlay {}: skipping invalid entry on line {}: {e}",
                            path.display(),
                            i + 1
                        ),
                    },
                    Err(e) => eprintln!(
                        "overlay {}: skipping malformed JSON on line {}: {e}",
                        path.display(),
                        i + 1
                    ),
                }
            }
        }
        Ok(Self {
            inner: Arc::new(Inner {
                store: RwLock::new(store),
                path,
            }),
        })
    }

    /// An in-memory-only overlay (no persistence), for tests.
    #[must_use]
    pub fn in_memory() -> Self {
        Self {
            inner: Arc::new(Inner {
                store: RwLock::new(MemoryStore::new()),
                path: PathBuf::new(),
            }),
        }
    }

    /// Append an entry to the overlay (persisting it) and apply it in memory.
    ///
    /// # Errors
    /// Returns an error if the entry cannot be persisted.
    pub fn append(&self, entry: &OverlayEntry) -> io::Result<()> {
        entry.validate().map_err(io::Error::other)?;
        if !self.inner.path.as_os_str().is_empty() {
            if let Some(parent) = self.inner.path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            let line = serde_json::to_string(entry).map_err(io::Error::other)?;
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.inner.path)?;
            writeln!(file, "{line}")?;
        }
        // A panic mid-write could poison the lock; recover the inner store rather than
        // cascading the panic to every later request.
        let mut store = self
            .inner
            .store
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        apply(&mut store, entry);
        Ok(())
    }
}

/// Apply an entry to an in-memory store (normalizing the lemma).
fn apply(store: &mut MemoryStore, entry: &OverlayEntry) {
    let reference = ParadigmRef::new(entry.hn, entry.tn).with_av(entry.av);
    store.insert(
        normalize(&entry.lemma),
        reference,
        entry.number,
        entry.case,
        Forms::present(entry.variants.clone(), Source::Overlay),
    );
}

impl FormStore for Overlay {
    fn paradigms(&self, lemma: &str) -> Vec<ParadigmRef> {
        self.inner
            .store
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .paradigms(lemma)
    }

    fn forms(
        &self,
        lemma: &str,
        reference: &ParadigmRef,
        number: Number,
        case: Case,
    ) -> Option<Forms> {
        self.inner
            .store
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .forms(lemma, reference, number, case)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keinontolibrary_core::Engine;

    fn entry(lemma: &str, form: &str) -> OverlayEntry {
        OverlayEntry {
            lemma: lemma.into(),
            tn: 1,
            hn: None,
            av: None,
            number: Number::Singular,
            case: Case::Inessive,
            variants: vec![form.into()],
        }
    }

    #[test]
    fn overlay_serves_appended_forms_and_persists() {
        let dir = std::env::temp_dir().join(format!("kl-overlay-{}", std::process::id()));
        let path = dir.join("overlay.jsonl");
        let _ = std::fs::remove_file(&path);

        let overlay = Overlay::open(&path).unwrap();
        overlay.append(&entry("uudissana", "uudissanassa")).unwrap();

        // Visible through the FormStore interface immediately.
        let r = ParadigmRef::new(None, 1);
        let f = overlay
            .forms("uudissana", &r, Number::Singular, Case::Inessive)
            .unwrap();
        assert_eq!(f.variants, vec!["uudissanassa"]);
        assert_eq!(f.source, Source::Overlay);

        // Reload from disk: the entry survived.
        let reopened = Overlay::open(&path).unwrap();
        assert_eq!(
            reopened
                .forms("uudissana", &r, Number::Singular, Case::Inessive)
                .unwrap()
                .variants,
            vec!["uudissanassa"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_rejects_invalid_entries() {
        let overlay = Overlay::in_memory();
        let mut e = entry("x", "xssa");
        e.variants = vec![];
        assert!(overlay.append(&e).is_err());
        let mut e = entry("x", "xssa");
        e.variants = vec![String::new()];
        assert!(overlay.append(&e).is_err());
        let mut e = entry("x", "xssa");
        e.tn = 200;
        assert!(overlay.append(&e).is_err());
        assert!(overlay.append(&entry("", "xssa")).is_err());
    }

    #[test]
    fn open_skips_malformed_and_invalid_lines() {
        let dir = std::env::temp_dir().join(format!("kl-ov-bad-{}", std::process::id()));
        let path = dir.join("overlay.jsonl");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            &path,
            "not json at all\n\
             {\"lemma\":\"\",\"tn\":1,\"number\":\"singular\",\"case\":\"inessive\",\"variants\":[\"x\"]}\n\
             {\"lemma\":\"hyvä\",\"tn\":10,\"number\":\"singular\",\"case\":\"inessive\",\"variants\":[\"hyvässä\"]}\n",
        )
        .unwrap();
        // Bad lines are skipped (logged), the good one survives.
        let overlay = Overlay::open(&path).unwrap();
        let r = ParadigmRef::new(None, 10);
        assert!(overlay
            .forms("hyvä", &r, Number::Singular, Case::Inessive)
            .is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn overlay_makes_new_lemma_declinable_through_engine() {
        let overlay = Overlay::in_memory();
        overlay.append(&entry("uudissana", "uudissanassa")).unwrap();
        let engine = Engine::builder().overlay(Box::new(overlay)).build();
        let f = engine
            .decline("uudissana", Number::Singular, Case::Inessive)
            .unwrap();
        assert_eq!(f.primary(), Some("uudissanassa"));
    }
}
