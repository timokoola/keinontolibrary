//! Parsing the Kotus *Nykysuomen sanalista 2024* TSV into the lemma inventory.
//!
//! Columns: `Hakusana \t Homonymia \t Sanaluokka \t Taivutustiedot`. We keep an entry iff
//! it is tagged a noun (`substantiivi`) and has at least one nominal `tn` in 1–49.
//! Compound types 50/51 and empty-`tn` rows (whose final component is itself listed) are
//! dropped. Paradigms are deduplicated by `(tn, av)` across homonyms, since `hn` does not
//! affect declension.

use std::collections::HashMap;

use keinontolibrary_core::normalize;

/// One declension paradigm parsed from the `Taivutustiedot` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KotusParadigm {
    /// Declension class (taivutusnumero), 1–49 for in-scope nouns.
    pub tn: u8,
    /// Gradation letter (astevaihtelu), if any.
    pub av: Option<char>,
    /// Secondary/rare paradigm — parenthesized in the source, e.g. `(5)`.
    pub rare: bool,
}

/// The parsed, filtered lemma inventory: normalized lemma → its distinct paradigms
/// (primary first).
#[derive(Debug, Default)]
pub struct Inventory {
    /// Normalized lemma → paradigms.
    pub lemmas: HashMap<String, Vec<KotusParadigm>>,
    /// Count of noun rows whose `Taivutustiedot` had no in-scope (1–49) class — i.e.
    /// compounds and indeclinables that were dropped.
    pub dropped_compounds: usize,
    /// Count of non-noun rows skipped.
    pub skipped_non_nouns: usize,
}

/// Parse a single `Taivutustiedot` token like `5`, `5*C`, `(5)`, or `41*A` into a paradigm.
///
/// Returns `None` for tokens that do not begin with a class number.
fn parse_token(token: &str) -> Option<KotusParadigm> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    // Parenthesized tokens mark a secondary/rare paradigm.
    let (inner, rare) = match token.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        Some(inner) => (inner.trim(), true),
        None => (token, false),
    };
    let mut parts = inner.splitn(2, '*');
    let tn: u8 = parts.next()?.trim().parse().ok()?;
    // The gradation letter is the first alphabetic char after `*` (handles `C`, `(C)`).
    let av = parts
        .next()
        .and_then(|s| s.chars().find(char::is_ascii_alphabetic));
    Some(KotusParadigm { tn, av, rare })
}

/// Parse the whole `Taivutustiedot` field (comma-separated tokens) into paradigms,
/// keeping only nominal classes 1–49.
fn parse_paradigms(field: &str) -> Vec<KotusParadigm> {
    field
        .split(',')
        .filter_map(parse_token)
        .filter(|p| (1..=49).contains(&p.tn))
        .collect()
}

/// Merge a row's paradigms into a lemma's accumulated list, deduplicating by `(tn, av)`.
/// A non-rare paradigm supersedes a rare duplicate.
fn merge(existing: &mut Vec<KotusParadigm>, incoming: Vec<KotusParadigm>) {
    for p in incoming {
        if let Some(found) = existing.iter_mut().find(|e| e.tn == p.tn && e.av == p.av) {
            if found.rare && !p.rare {
                found.rare = false;
            }
        } else {
            existing.push(p);
        }
    }
}

impl Inventory {
    /// Parse the Kotus TSV text into an inventory.
    pub fn parse_str(text: &str) -> Self {
        let mut inv = Inventory::default();
        for (i, line) in text.lines().enumerate() {
            // Skip the header row.
            if i == 0 && line.starts_with("Hakusana") {
                continue;
            }
            if line.trim().is_empty() {
                continue;
            }
            let mut cols = line.split('\t');
            let word = cols.next().unwrap_or("");
            let _hn = cols.next().unwrap_or("");
            let sanaluokka = cols.next().unwrap_or("");
            let taivutus = cols.next().unwrap_or("");
            if word.is_empty() {
                continue;
            }
            if !sanaluokka.split([',', ' ']).any(|w| w == "substantiivi") {
                inv.skipped_non_nouns += 1;
                continue;
            }
            let paradigms = parse_paradigms(taivutus);
            if paradigms.is_empty() {
                inv.dropped_compounds += 1;
                continue;
            }
            let lemma = normalize(word);
            merge(inv.lemmas.entry(lemma).or_default(), paradigms);
        }
        inv
    }

    /// Number of lemmas kept.
    pub fn len(&self) -> usize {
        self.lemmas.len()
    }

    /// Whether the inventory is empty.
    pub fn is_empty(&self) -> bool {
        self.lemmas.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_and_gradation_tokens() {
        assert_eq!(
            parse_token("5"),
            Some(KotusParadigm {
                tn: 5,
                av: None,
                rare: false
            })
        );
        assert_eq!(
            parse_token("41*A"),
            Some(KotusParadigm {
                tn: 41,
                av: Some('A'),
                rare: false
            })
        );
        assert_eq!(
            parse_token("(5)"),
            Some(KotusParadigm {
                tn: 5,
                av: None,
                rare: true
            })
        );
        assert_eq!(
            parse_token("32*C"),
            Some(KotusParadigm {
                tn: 32,
                av: Some('C'),
                rare: false
            })
        );
        assert_eq!(parse_token(""), None);
        assert_eq!(parse_token("abc"), None);
    }

    #[test]
    fn multi_paradigm_field_with_rare_secondary() {
        // `alpi -> 7*E, (5)`
        let ps = parse_paradigms("7*E, (5)");
        assert_eq!(
            ps,
            vec![
                KotusParadigm {
                    tn: 7,
                    av: Some('E'),
                    rare: false
                },
                KotusParadigm {
                    tn: 5,
                    av: None,
                    rare: true
                },
            ]
        );
    }

    #[test]
    fn drops_compound_classes_50_51_and_verbs() {
        assert!(parse_paradigms("50").is_empty());
        assert!(parse_paradigms("51").is_empty());
        assert!(parse_paradigms("53*C").is_empty()); // verb class
        assert!(parse_paradigms("99").is_empty()); // indeclinable
    }

    #[test]
    fn inventory_filters_nouns_and_dedups_homonyms() {
        let tsv = "Hakusana\tHomonymia\tSanaluokka\tTaivutustiedot\n\
                   talo\t\tsubstantiivi\t1\n\
                   aarnio\t1\tsubstantiivi\t3\n\
                   aarnio\t2\tsubstantiivi\t3\n\
                   nopea\t\tadjektiivi\t10\n\
                   aakkosjärjestys\t\tsubstantiivi\t\n";
        let inv = Inventory::parse_str(tsv);
        assert_eq!(
            inv.lemmas["talo"],
            vec![KotusParadigm {
                tn: 1,
                av: None,
                rare: false
            }]
        );
        // Two homonyms, same tn=3 -> collapsed to one paradigm.
        assert_eq!(inv.lemmas["aarnio"].len(), 1);
        // Adjective dropped, compound (empty tn) dropped.
        assert!(!inv.lemmas.contains_key("nopea"));
        assert!(!inv.lemmas.contains_key("aakkosjärjestys"));
        assert_eq!(inv.dropped_compounds, 1);
        assert_eq!(inv.skipped_non_nouns, 1);
    }
}
