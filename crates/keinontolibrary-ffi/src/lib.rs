//! `keinontolibrary-ffi` — a binding-ready surface over the declension engine.
//!
//! Two layers:
//! - A clean **Rust API** ([`Inflector`]) using only plain owned types (no leaked
//!   lifetimes, no library-specific enums across the boundary): numbers/cases are passed
//!   as strings and results are JSON. This is intentionally UniFFI-shaped — wrapping these
//!   methods with `#[uniffi::export]` (Swift) is the next step.
//! - A portable **C ABI** ([`keinonto_open`] etc.) callable from Swift, Python (ctypes),
//!   or wasm hosts today.
//!
//! Feature-gated `wasm` (Cloudflare Workers) and `PyO3` (Python) packaging will build on
//! this same `Inflector` surface.
#![allow(unsafe_code)] // The C ABI requires `unsafe`; this crate is the FFI boundary.

use std::ffi::{c_char, CStr, CString};

use keinontolibrary_core::{Case, Error, Number};
use keinontolibrary_data::{build_engine, EngineBundle};
use serde_json::json;

/// A loaded declension engine with a JSON-in/JSON-out, FFI-clean surface.
#[derive(Debug)]
pub struct Inflector {
    bundle: EngineBundle,
}

impl Inflector {
    /// Open an inflector backed by the artifact and overlay at the given paths.
    ///
    /// # Errors
    /// Returns a message if the artifact cannot be loaded.
    pub fn open(artifact_path: &str, overlay_path: &str) -> Result<Self, String> {
        let bundle = build_engine(artifact_path, overlay_path).map_err(|e| e.to_string())?;
        Ok(Self { bundle })
    }

    /// Decline `word` into one `(number, case)` slot, returning a JSON object.
    ///
    /// On success: `{"variants":[...],"status":...,"source":...,"coincides_with":...}`.
    /// On failure: `{"error":...,...}`.
    #[must_use]
    pub fn decline(&self, word: &str, number: &str, case: &str) -> String {
        let (number, case) = match parse_pair(number, case) {
            Ok(pair) => pair,
            Err(msg) => return json!({ "error": "bad_argument", "message": msg }).to_string(),
        };
        match self.bundle.engine.decline(word, number, case) {
            Ok(forms) => serde_json::to_string(&forms)
                .unwrap_or_else(|_| error_json(&Error::UnknownWord(word.to_owned()))),
            Err(e) => error_json(&e),
        }
    }

    /// Build the whole paradigm for `word`, returning a JSON object (or an error object).
    #[must_use]
    pub fn paradigm(&self, word: &str) -> String {
        match self.bundle.engine.paradigm(word) {
            Ok(p) => serde_json::to_string(&p).unwrap_or_else(|_| "{}".to_owned()),
            Err(e) => error_json(&e),
        }
    }
}

fn parse_pair(number: &str, case: &str) -> Result<(Number, Case), String> {
    let number = number.parse::<Number>().map_err(|e| e.to_string())?;
    let case = case.parse::<Case>().map_err(|e| e.to_string())?;
    Ok((number, case))
}

fn error_json(e: &Error) -> String {
    let value = match e {
        Error::UnknownWord(word) => json!({ "error": "unknown_word", "word": word }),
        Error::Ambiguous { lemma, paradigms } => json!({
            "error": "ambiguous",
            "lemma": lemma,
            "paradigms": paradigms.iter().map(|p| json!({ "tn": p.tn, "hn": p.hn })).collect::<Vec<_>>(),
        }),
        Error::DefectiveForm {
            lemma,
            number,
            case,
        } => json!({
            "error": "defective_form", "lemma": lemma,
            "number": number.name(), "case": case.name(),
        }),
    };
    value.to_string()
}

// ----------------------------------------------------------------------------------------
// C ABI. Memory contract: strings returned by `keinonto_*` must be freed with
// `keinonto_string_free`; an `Inflector*` from `keinonto_open` must be freed with
// `keinonto_free`. All `*const c_char` inputs must be valid NUL-terminated UTF-8.
// ----------------------------------------------------------------------------------------

/// Read a C string into an owned `String`, or `None` if null/invalid UTF-8.
///
/// # Safety
/// `ptr` must be null or a valid NUL-terminated C string.
unsafe fn cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok().map(ToOwned::to_owned)
}

fn into_c_string(s: String) -> *mut c_char {
    CString::new(s).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// Open an inflector. Returns null on failure.
///
/// # Safety
/// `artifact` and `overlay` must be valid NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn keinonto_open(
    artifact: *const c_char,
    overlay: *const c_char,
) -> *mut Inflector {
    let (Some(artifact), Some(overlay)) = (cstr(artifact), cstr(overlay)) else {
        return std::ptr::null_mut();
    };
    match Inflector::open(&artifact, &overlay) {
        Ok(inflector) => Box::into_raw(Box::new(inflector)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Decline a word; returns a JSON C string (free with `keinonto_string_free`), or null.
///
/// # Safety
/// `inflector` must come from [`keinonto_open`]; the string args must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn keinonto_decline(
    inflector: *const Inflector,
    word: *const c_char,
    number: *const c_char,
    case: *const c_char,
) -> *mut c_char {
    let (Some(inflector), Some(word), Some(number), Some(case)) =
        (inflector.as_ref(), cstr(word), cstr(number), cstr(case))
    else {
        return std::ptr::null_mut();
    };
    into_c_string(inflector.decline(&word, &number, &case))
}

/// Build a paradigm; returns a JSON C string (free with `keinonto_string_free`), or null.
///
/// # Safety
/// `inflector` must come from [`keinonto_open`]; `word` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn keinonto_paradigm(
    inflector: *const Inflector,
    word: *const c_char,
) -> *mut c_char {
    let (Some(inflector), Some(word)) = (inflector.as_ref(), cstr(word)) else {
        return std::ptr::null_mut();
    };
    into_c_string(inflector.paradigm(&word))
}

/// Free a string returned by this library.
///
/// # Safety
/// `s` must have been returned by a `keinonto_*` function and not already freed.
#[no_mangle]
pub unsafe extern "C" fn keinonto_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Free an inflector returned by [`keinonto_open`].
///
/// # Safety
/// `inflector` must have been returned by [`keinonto_open`] and not already freed.
#[no_mangle]
pub unsafe extern "C" fn keinonto_free(inflector: *mut Inflector) {
    if !inflector.is_null() {
        drop(Box::from_raw(inflector));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pair_accepts_english_names() {
        assert!(parse_pair("singular", "inessive").is_ok());
        assert!(parse_pair("plural", "genitive").is_ok());
        assert!(parse_pair("dual", "inessive").is_err());
        assert!(parse_pair("singular", "sisaolento").is_err());
    }

    #[test]
    fn error_json_shapes() {
        let s = error_json(&Error::UnknownWord("xyz".into()));
        assert!(s.contains("unknown_word"));
        assert!(s.contains("xyz"));
    }
}
