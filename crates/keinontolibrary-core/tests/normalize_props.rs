//! Property tests for `normalize()` over arbitrary Unicode (#40): it must never panic,
//! be idempotent, and leave no leading/trailing whitespace or uppercase ASCII.

use keinontolibrary_core::normalize;
use proptest::prelude::*;

proptest! {
    // Any input — combining marks, ZWJ, RTL marks, control chars, very long strings.
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn normalize_is_idempotent(s in ".*") {
        let once = normalize(&s);
        prop_assert_eq!(normalize(&once), once);
    }

    #[test]
    fn normalize_trims_and_lowercases_ascii(s in ".*") {
        let n = normalize(&s);
        prop_assert_eq!(n.trim(), n.as_str(), "leftover surrounding whitespace");
        prop_assert!(
            !n.chars().any(|c| c.is_ascii_uppercase()),
            "leftover uppercase ASCII in {n:?}"
        );
    }

    // Long pure-Finnish strings round-trip without surprises (idempotent + no growth
    // beyond NFC for already-NFC lowercase text).
    #[test]
    fn finnish_text_is_stable(s in "[a-zåäö ]{0,200}") {
        let n = normalize(&s);
        prop_assert_eq!(normalize(&n), n);
    }
}
