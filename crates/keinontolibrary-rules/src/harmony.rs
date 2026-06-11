//! Vowel harmony: Finnish endings come in back (`a/o/u`) and front (`ä/ö/y`) variants.

/// Whether a word takes back-vowel harmony.
///
/// The last STRONG vowel decides, scanning from the end: back for `a/o/u`, front for
/// `ä/ö`; a word with no strong vowel is front. This gets disharmonic loanwords right
/// (`afääri → afääriä`, `tyranni → tyrannia` — Voikko-verified; the old "contains any
/// back vowel" rule produced *afääria). `y` deliberately does NOT decide: in
/// English-orthography loans it is not the front vowel (`country → countrya`,
/// `jury → jurya` — Voikko-verified), and words whose only non-neutral vowel is `y`
/// (`lyhyt`, `kymmenes`) take front endings via the fallback anyway. Compound harmony
/// (follows
/// the final component) is handled separately by the engine's compound override.
#[must_use]
pub fn is_back(word: &str) -> bool {
    word.chars()
        .rev()
        .find_map(|c| match c {
            'a' | 'o' | 'u' => Some(true),
            'ä' | 'ö' => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

/// The harmonic `a`/`ä` for a word (used in `-ssa`, `-lla`, `-na`, `-ta`, `-ksi`-less
/// endings, etc.).
#[must_use]
pub fn aa(word: &str) -> &'static str {
    if is_back(word) {
        "a"
    } else {
        "ä"
    }
}

/// The harmonic `o`/`ö` for a word (used in the plural `-oi-`/`-öi-` stem).
#[must_use]
pub fn oo(word: &str) -> char {
    if is_back(word) {
        'o'
    } else {
        'ö'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harmony_picks_back_or_front() {
        assert!(is_back("valo"));
        assert!(is_back("kala"));
        assert!(!is_back("risti")); // only i -> front
        assert!(!is_back("pöytä"));
        assert!(!is_back("metsä"));
        assert_eq!(aa("valo"), "a");
        assert_eq!(aa("risti"), "ä");
    }

    // Disharmonic loanwords: the LAST strong vowel decides (Voikko-verified: afääriä,
    // tyrannia, countrya, jurya). Found by the QA loop; the country/jury/y cases by its
    // regression gate.
    #[test]
    fn disharmonic_loanwords_follow_last_strong_vowel() {
        assert!(!is_back("afääri")); // a…ä -> front: afääriä
        assert!(is_back("tyranni")); // y…a -> back: tyrannia
        assert!(is_back("country")); // orthographic y does not decide: countrya
        assert!(is_back("jury")); // jurya
        assert!(!is_back("kymmenes")); // no strong vowel -> front: kymmenettä
        assert_eq!(aa("afääri"), "ä");
        assert_eq!(aa("country"), "a");
    }
}
