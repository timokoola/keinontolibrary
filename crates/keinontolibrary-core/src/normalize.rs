//! Input normalization applied before every lookup.

use unicode_normalization::UnicodeNormalization;

/// Normalize a lemma for lookup: trim surrounding whitespace, apply Unicode NFC, and
/// lowercase.
///
/// This is idempotent: `normalize(normalize(s)) == normalize(s)`.
///
/// Lowercasing uses Unicode-aware `to_lowercase`, which correctly handles `Å/å`, `Ä/ä`,
/// `Ö/ö` and the rest of the Finnish alphabet.
pub fn normalize(input: &str) -> String {
    // NFC first so that precomposed/decomposed forms collapse before casing, then
    // lowercase, then NFC again because case folding can denormalize.
    input
        .trim()
        .nfc()
        .collect::<String>()
        .to_lowercase()
        .nfc()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn trims_and_lowercases() {
        assert_eq!(normalize("  Hevonen  "), "hevonen");
        assert_eq!(normalize("KISSA"), "kissa");
    }

    #[test]
    fn handles_finnish_letters() {
        assert_eq!(normalize("PÖYTÄ"), "pöytä");
        assert_eq!(normalize("Ääni"), "ääni");
    }

    #[test]
    fn is_idempotent() {
        for s in ["  Pöytä ", "KÄSI", "åäö", "Hevonen", "  3D-tulostin"] {
            let once = normalize(s);
            assert_eq!(normalize(&once), once, "not idempotent for {s:?}");
        }
    }

    #[test]
    fn collapses_decomposed_a_ring_to_nfc() {
        // "å" as a + combining ring (U+0061 U+030A) must normalize to U+00E5.
        let decomposed = "A\u{030A}";
        assert_eq!(normalize(decomposed), "\u{00e5}");
    }
}
