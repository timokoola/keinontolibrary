//! The end-to-end ingest: join the Kotus inventory with the Voikko corpus and emit the
//! packed [`Artifact`] plus a human-readable report.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use keinontolibrary_core::{Case, Number, Status};
use keinontolibrary_data::{
    slot_index, slot_parts, Artifact, LemmaRecord, Meta, ParadigmRecord, SlotRecord,
};
use rayon::prelude::*;

use crate::kotus::Inventory;
use crate::voikko::{self, CleanForm};

/// Where to read sources and write outputs.
#[derive(Debug, Clone)]
pub struct Config {
    /// Path to the Kotus `nykysuomensanalista2024.txt`.
    pub kotus_path: PathBuf,
    /// Directory of Voikko `*.jsonl` shards.
    pub voikko_dir: PathBuf,
    /// Output path for the packed artifact.
    pub artifact_path: PathBuf,
    /// Output path for the ingest report.
    pub report_path: PathBuf,
    /// Version string stamped into the artifact metadata.
    pub version: String,
}

/// Summary statistics from a run.
#[derive(Debug, Default, Clone)]
pub struct Report {
    /// Kotus noun lemmas kept.
    pub kotus_lemmas: usize,
    /// Kotus rows dropped as compounds/indeclinables (no in-scope `tn`).
    pub kotus_dropped_compounds: usize,
    /// Kotus rows skipped as non-nouns.
    pub kotus_skipped_non_nouns: usize,
    /// Total Voikko forms surviving filters.
    pub voikko_forms_kept: usize,
    /// Voikko forms whose lemma is not in the Kotus noun inventory (ignored).
    pub voikko_forms_not_in_kotus: usize,
    /// Kotus lemmas with at least one attested corpus form.
    pub lemmas_with_forms: usize,
    /// Kotus lemmas with no corpus forms at all (left for the rule fallback).
    pub lemmas_without_forms: usize,
    /// `(lemma, tn)` groups where the Voikko gradation letter disagreed with Kotus.
    pub av_mismatches: usize,
    /// Distinct (lemma, paradigm, slot, variant) forms in the artifact.
    pub total_forms: u64,
    /// Form count per case (indexed by [`Case::index`]).
    pub forms_per_case: [u64; 15],
    /// Number of lemmas that have at least one attested form, per declension class `tn`.
    pub lemmas_per_class: BTreeMap<u8, usize>,
}

impl Report {
    fn render(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "keinontolibrary ingest report");
        let _ = writeln!(s, "=============================");
        let _ = writeln!(s, "Kotus noun lemmas kept:           {}", self.kotus_lemmas);
        let _ = writeln!(
            s,
            "Kotus compounds/indeclinables:    {}",
            self.kotus_dropped_compounds
        );
        let _ = writeln!(
            s,
            "Kotus non-noun rows skipped:      {}",
            self.kotus_skipped_non_nouns
        );
        let _ = writeln!(
            s,
            "Voikko forms kept:                {}",
            self.voikko_forms_kept
        );
        let _ = writeln!(
            s,
            "Voikko forms not in Kotus:        {}",
            self.voikko_forms_not_in_kotus
        );
        let _ = writeln!(
            s,
            "Lemmas with corpus forms:         {}",
            self.lemmas_with_forms
        );
        let _ = writeln!(
            s,
            "Lemmas without corpus forms:      {}",
            self.lemmas_without_forms
        );
        let _ = writeln!(
            s,
            "av mismatches (Kotus vs Voikko):  {}",
            self.av_mismatches
        );
        let _ = writeln!(s, "Total forms in artifact:          {}", self.total_forms);

        let _ = writeln!(s, "\nForms per case:");
        for case in Case::ALL {
            let _ = writeln!(
                s,
                "  {:<12} {}",
                case.name(),
                self.forms_per_case[case.index()]
            );
        }

        let _ = writeln!(s, "\nLemmas with forms per class (tn):");
        for (tn, n) in &self.lemmas_per_class {
            let _ = writeln!(s, "  tn {tn:<3} {n}");
        }
        s
    }
}

/// A `(lemma, tn)` group of attested forms: packed slot index → ordered, deduped variants.
type SlotMap = HashMap<u8, Vec<String>>;
/// Attested forms grouped by `(lemma, tn)`.
type Groups = HashMap<(String, u8), SlotMap>;
/// The Voikko gradation letter seen per `(lemma, tn)` group, for cross-checking Kotus.
type AvSeen = HashMap<(String, u8), Option<char>>;

/// Push a variant preserving first-occurrence order and dropping duplicates.
fn push_unique(variants: &mut Vec<String>, form: String) {
    if !variants.contains(&form) {
        variants.push(form);
    }
}

fn case_index_u8(case: Case) -> u8 {
    u8::try_from(case.index()).expect("case index < 15")
}

/// List the `*.jsonl` shards in `dir`, sorted by file name for determinism.
fn list_shards(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading Voikko dir {}", dir.display()))?
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .collect();
    paths.sort();
    Ok(paths)
}

/// Parse all shards in parallel, returning forms concatenated in shard (file-name) order.
fn parse_all(shards: &[PathBuf]) -> Vec<CleanForm> {
    let mut per_shard: Vec<(usize, Vec<CleanForm>)> = shards
        .par_iter()
        .enumerate()
        .map(|(i, path)| {
            let text = std::fs::read_to_string(path).unwrap_or_default();
            (i, voikko::parse_shard(&text))
        })
        .collect();
    per_shard.sort_by_key(|(i, _)| *i);
    per_shard.into_iter().flat_map(|(_, forms)| forms).collect()
}

/// Group surviving forms by `(lemma, tn)`, keeping only lemmas in the Kotus inventory.
///
/// Returns the grouping, the Voikko gradation letter seen per group (for cross-check), and
/// the count of forms whose lemma was not in Kotus.
fn group_forms(inv: &Inventory, forms: Vec<CleanForm>) -> (Groups, AvSeen, usize) {
    let mut groups: Groups = HashMap::new();
    let mut av_seen: AvSeen = HashMap::new();
    let mut not_in_kotus = 0usize;

    for f in forms {
        if !inv.lemmas.contains_key(&f.lemma) {
            not_in_kotus += 1;
            continue;
        }
        let key = (f.lemma.clone(), f.tn);
        av_seen.entry(key.clone()).or_insert(f.av);
        let slot = slot_index(f.number, f.case);
        push_unique(
            groups.entry(key).or_default().entry(slot).or_default(),
            f.form,
        );
    }
    (groups, av_seen, not_in_kotus)
}

/// Build the slot records for one `(lemma, tn)` paradigm, deriving the accusative and
/// flagging the rare instructive singular.
fn build_slots(group: Option<&SlotMap>) -> (Vec<SlotRecord>, u64) {
    let mut records: Vec<SlotRecord> = Vec::new();
    let mut form_count: u64 = 0;
    let Some(group) = group else {
        return (records, 0);
    };

    for (&slot, variants) in group {
        if variants.is_empty() {
            continue;
        }
        let (number, case) = slot_parts(slot);
        // Singular instructive is marginal/lexicalized.
        let status = if case == Case::Instructive && number == Number::Singular {
            Status::Rare
        } else {
            Status::Present
        };
        form_count += variants.len() as u64;
        records.push(SlotRecord {
            slot,
            status,
            variants: variants.clone(),
            coincides_with: None,
        });
    }

    // Derive the accusative: singular = genitive singular, plural = nominative plural.
    let derive =
        |records: &mut Vec<SlotRecord>, form_count: &mut u64, src_n, src_c, coincides: Case| {
            if let Some(src) = group.get(&slot_index(src_n, src_c)) {
                if !src.is_empty() {
                    *form_count += src.len() as u64;
                    records.push(SlotRecord {
                        slot: slot_index(src_n, Case::Accusative),
                        status: Status::Present,
                        variants: src.clone(),
                        coincides_with: Some(case_index_u8(coincides)),
                    });
                }
            }
        };
    derive(
        &mut records,
        &mut form_count,
        Number::Singular,
        Case::Genitive,
        Case::Genitive,
    );
    derive(
        &mut records,
        &mut form_count,
        Number::Plural,
        Case::Nominative,
        Case::Nominative,
    );

    records.sort_by_key(|r| r.slot);
    (records, form_count)
}

/// Run the full ingest, writing the artifact and report. Returns the report.
pub fn run(config: &Config) -> Result<Report> {
    let kotus_text = std::fs::read_to_string(&config.kotus_path)
        .with_context(|| format!("reading Kotus list {}", config.kotus_path.display()))?;
    let inv = Inventory::parse_str(&kotus_text);

    let shards = list_shards(&config.voikko_dir)?;
    let forms = parse_all(&shards);
    let voikko_forms_kept = forms.len();

    let (groups, av_seen, not_in_kotus) = group_forms(&inv, forms);

    let mut report = Report {
        kotus_lemmas: inv.len(),
        kotus_dropped_compounds: inv.dropped_compounds,
        kotus_skipped_non_nouns: inv.skipped_non_nouns,
        voikko_forms_kept,
        voikko_forms_not_in_kotus: not_in_kotus,
        ..Report::default()
    };

    // Build lemma records in sorted order for a deterministic artifact.
    let mut lemmas: Vec<&String> = inv.lemmas.keys().collect();
    lemmas.sort();

    let mut records = Vec::with_capacity(lemmas.len());
    for lemma in lemmas {
        let paradigms = &inv.lemmas[lemma];
        let mut had_forms = false;
        let mut paradigm_records = Vec::with_capacity(paradigms.len());
        for p in paradigms {
            let key = (lemma.clone(), p.tn);
            // Cross-check gradation letters.
            if let Some(&voikko_av) = av_seen.get(&key) {
                if voikko_av != p.av {
                    report.av_mismatches += 1;
                }
            }
            let (slots, n) = build_slots(groups.get(&key));
            if !slots.is_empty() {
                had_forms = true;
                *report.lemmas_per_class.entry(p.tn).or_default() += 1;
                for slot in &slots {
                    let (_, case) = slot_parts(slot.slot);
                    report.forms_per_case[case.index()] += slot.variants.len() as u64;
                }
            }
            report.total_forms += n;
            paradigm_records.push(ParadigmRecord {
                tn: p.tn,
                av: p.av,
                rare: p.rare,
                slots,
            });
        }
        if had_forms {
            report.lemmas_with_forms += 1;
        } else {
            report.lemmas_without_forms += 1;
        }
        records.push(LemmaRecord {
            lemma: lemma.clone(),
            paradigms: paradigm_records,
        });
    }

    let meta = Meta {
        version: config.version.clone(),
        kotus_source: "Kotus Nykysuomen sanalista 2024 (CC BY 4.0)".into(),
        voikko_source: "Voikko-generated JSONL corpus".into(),
        n_lemmas: u32::try_from(records.len()).unwrap_or(u32::MAX),
        n_forms: report.total_forms,
    };
    let artifact = Artifact {
        meta,
        lemmas: records,
    };

    if let Some(parent) = config.artifact_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    artifact
        .write_to(&config.artifact_path)
        .with_context(|| format!("writing artifact {}", config.artifact_path.display()))?;
    std::fs::write(&config.report_path, report.render())
        .with_context(|| format!("writing report {}", config.report_path.display()))?;

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kotus::KotusParadigm;

    fn inv_with(lemma: &str, tn: u8) -> Inventory {
        let mut inv = Inventory::default();
        inv.lemmas.insert(
            lemma.to_owned(),
            vec![KotusParadigm {
                tn,
                av: None,
                rare: false,
            }],
        );
        inv
    }

    #[test]
    fn groups_and_derives_accusative() {
        let inv = inv_with("talo", 1);
        let forms = vec![
            CleanForm {
                lemma: "talo".into(),
                tn: 1,
                av: None,
                number: Number::Singular,
                case: Case::Genitive,
                form: "talon".into(),
            },
            CleanForm {
                lemma: "talo".into(),
                tn: 1,
                av: None,
                number: Number::Plural,
                case: Case::Nominative,
                form: "talot".into(),
            },
        ];
        let (groups, _, _) = group_forms(&inv, forms);
        let (slots, count) = build_slots(groups.get(&("talo".to_string(), 1)));
        assert_eq!(count, 4); // gen sg, nom pl, + derived acc sg, acc pl

        let acc_sg = slots
            .iter()
            .find(|s| slot_parts(s.slot) == (Number::Singular, Case::Accusative))
            .unwrap();
        assert_eq!(acc_sg.variants, vec!["talon"]);
        assert_eq!(acc_sg.coincides_with, Some(case_index_u8(Case::Genitive)));

        let acc_pl = slots
            .iter()
            .find(|s| slot_parts(s.slot) == (Number::Plural, Case::Accusative))
            .unwrap();
        assert_eq!(acc_pl.variants, vec!["talot"]);
        assert_eq!(acc_pl.coincides_with, Some(case_index_u8(Case::Nominative)));
    }

    #[test]
    fn instructive_singular_marked_rare() {
        let inv = inv_with("käsi", 27);
        let forms = vec![CleanForm {
            lemma: "käsi".into(),
            tn: 27,
            av: None,
            number: Number::Singular,
            case: Case::Instructive,
            form: "käsin".into(),
        }];
        let (groups, _, _) = group_forms(&inv, forms);
        let (slots, _) = build_slots(groups.get(&("käsi".to_string(), 27)));
        let instr = slots
            .iter()
            .find(|s| slot_parts(s.slot).1 == Case::Instructive)
            .unwrap();
        assert_eq!(instr.status, Status::Rare);
    }

    #[test]
    fn forms_not_in_kotus_are_counted() {
        let inv = inv_with("talo", 1);
        let forms = vec![CleanForm {
            lemma: "ankka".into(),
            tn: 9,
            av: None,
            number: Number::Singular,
            case: Case::Genitive,
            form: "ankan".into(),
        }];
        let (_, _, not_in_kotus) = group_forms(&inv, forms);
        assert_eq!(not_in_kotus, 1);
    }
}
