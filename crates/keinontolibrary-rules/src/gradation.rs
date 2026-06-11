//! Consonant gradation (astevaihtelu): the k/p/t alternations of types A–M, and which
//! slots take the strong vs weak grade.
//!
//! This implements *direct* gradation (strong nominative singular, weak oblique), which
//! covers the high-frequency vowel-stem classes. Reverse gradation (weak nominative, e.g.
//! types 32/33) is not yet handled.

use keinontolibrary_core::{Case, Number};

/// Strong or weak consonant grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    Strong,
    Weak,
}

/// The grade a `(number, case)` slot takes under direct gradation.
///
/// Derived from real paradigms (kortti/pukki/suku/...): weak when the gradating syllable
/// closes (genitive, the `-ssa/-lla/...` locatives, translative, abessive, nominative
/// plural); strong otherwise (nominative sg, partitive, illative, essive, and the
/// genitive/partitive/comitative plural).
#[must_use]
pub fn grade(number: Number, case: Case) -> Grade {
    use Case::{Comitative, Essive, Genitive, Illative, Nominative, Partitive};
    // Strong before an open final syllable: nominative/partitive/illative/essive singular,
    // and the genitive/partitive/illative/essive/comitative plural. Everything else (the
    // genitive and locatives singular, the nominative plural, translative, abessive, ...)
    // closes the syllable and takes the weak grade.
    let strong = matches!(
        (number, case),
        (Number::Singular, Nominative | Partitive | Illative | Essive)
            | (
                Number::Plural,
                Genitive | Partitive | Illative | Essive | Comitative
            )
    );
    if strong {
        Grade::Strong
    } else {
        Grade::Weak
    }
}

const fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'ä' | 'ö')
}

/// The (strong, weak) consonant pair for a gradation letter.
fn pair(av: char) -> Option<(&'static str, &'static str)> {
    Some(match av.to_ascii_uppercase() {
        'A' => ("kk", "k"),
        'B' => ("pp", "p"),
        'C' => ("tt", "t"),
        'D' => ("k", ""),
        'E' => ("p", "v"),
        'F' => ("t", "d"),
        'G' => ("nk", "ng"),
        'H' => ("mp", "mm"),
        'I' => ("lt", "ll"),
        'J' => ("nt", "nn"),
        'K' => ("rt", "rr"),
        'L' => ("k", "j"),
        'M' => ("k", "v"),
        _ => return None,
    })
}

/// Locate the gradating consonant cluster: the consonant run immediately before the
/// stem's trailing vowel sequence. Returns `(prefix, cluster, trailing_vowels)`.
///
/// Skipping the *whole* trailing vowel run (not just one vowel) is what makes long-vowel
/// stems like `aartee` work — the cluster `rt` sits before `ee`, not before the last `e`.
fn split(stem: &str) -> Option<(String, String, String)> {
    let chars: Vec<char> = stem.chars().collect();
    let mut vstart = chars.len();
    while vstart > 0 && is_vowel(chars[vstart - 1]) {
        vstart -= 1;
    }
    if vstart == 0 {
        return None; // no consonant before the trailing vowels
    }
    let mut cstart = vstart;
    while cstart > 0 && !is_vowel(chars[cstart - 1]) {
        cstart -= 1;
    }
    let prefix: String = chars[..cstart].iter().collect();
    let cluster: String = chars[cstart..vstart].iter().collect();
    let trailing: String = chars[vstart..].iter().collect();
    Some((prefix, cluster, trailing))
}

/// Replace the trailing `from` of the gradating cluster with `to`.
///
/// Orthography for full elision (D `k:∅`, leaving no consonant): when the gap sits
/// between identical vowels AND a vowel precedes it, the syllable boundary is written
/// with an apostrophe — `ruoko → ruo'on`, `vaaka → vaa'an` — but after a consonant the
/// vowels merge into a long vowel: `koko → koon`, `rako → raon` (all Voikko-verified).
fn regrade(stem: &str, from: &str, to: &str) -> String {
    let Some((prefix, cluster, trailing)) = split(stem) else {
        return stem.to_owned();
    };
    match cluster.strip_suffix(from) {
        Some(head) if to.is_empty() && head.is_empty() => {
            let mut left = prefix.chars().rev();
            let apostrophe = match (left.next(), left.next(), trailing.chars().next()) {
                (Some(l), Some(before), Some(r)) => l == r && is_vowel(before),
                _ => false,
            };
            if apostrophe {
                format!("{prefix}'{trailing}")
            } else {
                format!("{prefix}{trailing}")
            }
        }
        Some(head) => format!("{prefix}{head}{to}{trailing}"),
        None => stem.to_owned(),
    }
}

/// Apply weak gradation to a strong vowel stem (strong → weak). Identity if no gradation
/// applies or the expected strong cluster isn't present.
#[must_use]
pub fn weaken(stem: &str, av: Option<char>) -> String {
    match av.and_then(pair) {
        Some((strong, weak)) => regrade(stem, strong, weak),
        None => stem.to_owned(),
    }
}

/// Apply strong gradation to a weak stem (weak → strong) — used for reverse-gradation
/// classes whose nominative shows the weak grade (e.g. type 48 `aarre` → `aartee`).
#[must_use]
pub fn strengthen(stem: &str, av: Option<char>) -> String {
    match av.and_then(pair) {
        Some((strong, weak)) => regrade(stem, weak, strong),
        None => stem.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weakens_each_type() {
        assert_eq!(weaken("pukki", Some('A')), "puki");
        assert_eq!(weaken("kaappi", Some('B')), "kaapi");
        assert_eq!(weaken("kortti", Some('C')), "korti");
        assert_eq!(weaken("reikä", Some('D')), "reiä");
        assert_eq!(weaken("leipä", Some('E')), "leivä"); // E = p:v
        assert_eq!(weaken("pöytä", Some('F')), "pöydä"); // F = t:d
        assert_eq!(weaken("kenkä", Some('G')), "kengä");
        assert_eq!(weaken("kampa", Some('H')), "kamma");
        assert_eq!(weaken("valta", Some('I')), "valla");
        assert_eq!(weaken("ranta", Some('J')), "ranna");
        assert_eq!(weaken("parta", Some('K')), "parra");
        assert_eq!(weaken("kurke", Some('L')), "kurje");
        assert_eq!(weaken("suku", Some('M')), "suvu");
    }

    #[test]
    fn no_gradation_is_identity() {
        assert_eq!(weaken("valo", None), "valo");
        assert_eq!(weaken("kala", None), "kala");
    }

    #[test]
    fn grade_table_spot_checks() {
        assert_eq!(grade(Number::Singular, Case::Nominative), Grade::Strong);
        assert_eq!(grade(Number::Singular, Case::Genitive), Grade::Weak);
        assert_eq!(grade(Number::Singular, Case::Partitive), Grade::Strong);
        assert_eq!(grade(Number::Singular, Case::Essive), Grade::Strong);
        assert_eq!(grade(Number::Singular, Case::Translative), Grade::Weak);
        assert_eq!(grade(Number::Plural, Case::Nominative), Grade::Weak);
        assert_eq!(grade(Number::Plural, Case::Genitive), Grade::Strong);
    }
}
