//! PyO3 bindings for keinontolibrary.
//!
//! Exposes a single `Inflector` class over the data-backed declension engine. The Python
//! package (`keinontolibrary/__init__.py`) locates the bundled artifact + overlay and wraps
//! this class with module-level `decline`/`paradigm` convenience functions.

use std::collections::BTreeMap;
use std::str::FromStr;

use keinontolibrary_core::{Case, Error, Number};
use keinontolibrary_data::{build_engine, EngineBundle};
use pyo3::exceptions::{PyValueError, PyKeyError};
use pyo3::prelude::*;

/// A loaded declension engine (artifact + overlay + rule fallback).
#[pyclass(module = "keinontolibrary")]
struct Inflector {
    bundle: EngineBundle,
}

#[pymethods]
impl Inflector {
    /// Open an inflector from an artifact file and an overlay file.
    ///
    /// Prefer the module-level `keinontolibrary.decline` / `.paradigm`, which use the
    /// artifact bundled in the wheel; construct this directly only to point at custom data.
    #[new]
    #[pyo3(signature = (artifact_path, overlay_path = ""))]
    fn new(artifact_path: &str, overlay_path: &str) -> PyResult<Self> {
        let bundle = build_engine(artifact_path, overlay_path)
            .map_err(|e| PyValueError::new_err(format!("failed to load engine: {e}")))?;
        Ok(Self { bundle })
    }

    /// Decline `word` into one `(number, case)` slot, returning the surface form(s).
    ///
    /// `number` is "singular"/"plural"; `case` is an English case name (e.g. "inessive").
    /// Raises `KeyError` for an unknown word and `ValueError` for bad arguments, ambiguity,
    /// or a defective (non-existent) slot.
    fn decline(&self, word: &str, number: &str, case: &str) -> PyResult<Vec<String>> {
        let (number, case) = parse_pair(number, case)?;
        match self.bundle.engine.decline(word, number, case) {
            Ok(forms) => Ok(forms.variants),
            Err(e) => Err(to_pyerr(e)),
        }
    }

    /// Build the whole paradigm for `word`: `{number: {case: [forms...]}}`.
    ///
    /// Raises `KeyError` for an unknown word and `ValueError` for ambiguity.
    fn paradigm(&self, word: &str) -> PyResult<BTreeMap<String, BTreeMap<String, Vec<String>>>> {
        let paradigm = self.bundle.engine.paradigm(word).map_err(to_pyerr)?;
        let mut out: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        for &number in &Number::ALL {
            let mut cases: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for &case in &Case::ALL {
                let forms = paradigm.get(number, case);
                cases.insert(case.to_string(), forms.variants.clone());
            }
            out.insert(number.to_string(), cases);
        }
        Ok(out)
    }
}

/// Parse the number/case string pair, raising `ValueError` on an unknown name.
fn parse_pair(number: &str, case: &str) -> PyResult<(Number, Case)> {
    let number = Number::from_str(number)
        .map_err(|_| PyValueError::new_err(format!("unknown number: {number:?} (use \"singular\" or \"plural\")")))?;
    let case = Case::from_str(case)
        .map_err(|_| PyValueError::new_err(format!("unknown case: {case:?}")))?;
    Ok((number, case))
}

/// Map an engine error onto the closest Python exception.
fn to_pyerr(err: Error) -> PyErr {
    match err {
        Error::UnknownWord(w) => PyKeyError::new_err(format!("unknown word: {w}")),
        other => PyValueError::new_err(other.to_string()),
    }
}

#[pymodule]
fn _keinontolibrary(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Inflector>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
