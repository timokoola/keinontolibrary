//! `keinontolibrary-core` — core types and public API for declining simple Finnish nouns.
//!
//! The primary surface is the free functions [`decline`] and [`paradigm`], which run
//! against a process-global [`Engine`]. A data-backed engine is installed at startup via
//! [`install`] (the `keinontolibrary-data` crate does this); until then the global engine
//! is empty and every query returns [`Error::UnknownWord`].
//!
//! For embedding or testing, construct an [`Engine`] directly with [`Engine::builder`] and
//! call its methods.
//!
//! ```
//! use keinontolibrary_core::{Case, Number, Engine, MemoryStore, ParadigmRef, Forms, Source};
//!
//! let mut store = MemoryStore::new();
//! store.insert("talo", ParadigmRef::new(None, 1), Number::Singular, Case::Inessive,
//!     Forms::present(vec!["talossa".into()], Source::Lookup));
//! let engine = Engine::builder().lookup(Box::new(store)).build();
//!
//! let forms = engine.decline("talo", Number::Singular, Case::Inessive).unwrap();
//! assert_eq!(forms.primary(), Some("talossa"));
//! ```

mod case;
mod engine;
mod error;
mod forms;
mod normalize;
mod paradigm_ref;

pub use case::{Case, Number, ParseError};
pub use engine::{Engine, EngineBuilder, FormStore, Generator, MemoryStore};
pub use error::Error;
pub use forms::{Forms, Paradigm, Source, Status};
pub use normalize::normalize;
pub use paradigm_ref::ParadigmRef;

use std::sync::OnceLock;

static ENGINE: OnceLock<Engine> = OnceLock::new();

/// Install the process-global engine used by the free [`decline`]/[`paradigm`] functions.
///
/// Returns `Err` (handing the engine back) if one was already installed — the global is
/// write-once. Call this once at startup.
///
/// # Errors
/// Returns the passed-in engine if a global engine was already installed.
pub fn install(engine: Engine) -> Result<(), Engine> {
    ENGINE.set(engine)
}

/// The global engine, defaulting to an empty one if none was installed.
fn global() -> &'static Engine {
    ENGINE.get_or_init(Engine::empty)
}

/// Decline `lemma` into a single `(number, case)` slot using the global engine.
///
/// # Errors
/// - [`Error::UnknownWord`] if the lemma is not in the inventory.
/// - [`Error::Ambiguous`] if the lemma has multiple paradigms (use [`decline_with`]).
/// - [`Error::DefectiveForm`] if the slot is defective for this lemma.
pub fn decline(lemma: &str, number: Number, case: Case) -> Result<Forms, Error> {
    global().decline(lemma, number, case)
}

/// Decline `lemma` into one slot, disambiguating homonyms with an explicit paradigm.
///
/// # Errors
/// As [`decline`], minus [`Error::Ambiguous`].
// The by-value `ParadigmRef` is the documented public API surface (see the spec); callers
// hand ownership in even though we only borrow it internally.
#[allow(clippy::needless_pass_by_value)]
pub fn decline_with(
    lemma: &str,
    number: Number,
    case: Case,
    paradigm: ParadigmRef,
) -> Result<Forms, Error> {
    global().decline_with(lemma, number, case, &paradigm)
}

/// Build the whole paradigm (all `number × case` slots) for `lemma`.
///
/// # Errors
/// - [`Error::UnknownWord`] if the lemma is not in the inventory.
/// - [`Error::Ambiguous`] if the lemma has multiple paradigms (use [`paradigm_with`]).
pub fn paradigm(lemma: &str) -> Result<Paradigm, Error> {
    global().paradigm(lemma)
}

/// Build the whole paradigm for an explicit paradigm of `lemma`.
///
/// # Errors
/// - [`Error::UnknownWord`] if neither the lemma nor the requested paradigm is known.
#[allow(clippy::needless_pass_by_value)]
pub fn paradigm_with(lemma: &str, paradigm: ParadigmRef) -> Result<Paradigm, Error> {
    global().paradigm_with(lemma, &paradigm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_defaults_to_empty_and_reports_unknown() {
        // No `install` in this test binary, so the global engine is empty.
        assert!(matches!(
            decline("hevonen", Number::Singular, Case::Inessive),
            Err(Error::UnknownWord(_))
        ));
        assert!(matches!(paradigm("hevonen"), Err(Error::UnknownWord(_))));
    }
}
