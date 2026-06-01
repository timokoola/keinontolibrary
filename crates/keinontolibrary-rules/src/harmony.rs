//! Vowel harmony: Finnish endings come in back (`a/o/u`) and front (`ä/ö/y`) variants.

/// Whether a word takes back-vowel harmony.
///
/// A word is back-harmonic if it contains any of `a/o/u`; otherwise it is front
/// (this includes words with only the neutral vowels `e/i`, which take front endings).
#[must_use]
pub fn is_back(word: &str) -> bool {
    word.chars().any(|c| matches!(c, 'a' | 'o' | 'u'))
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
}
