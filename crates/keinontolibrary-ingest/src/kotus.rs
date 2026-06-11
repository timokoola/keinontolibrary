//! Parsing the Kotus *Nykysuomen sanalista 2024* TSV into the lemma inventory.
//!
//! Columns: `Hakusana \t Homonymia \t Sanaluokka \t Taivutustiedot`. We keep an entry iff
//! it is tagged a **nominal** — substantiivi, adjektiivi, numeraali or pronomini, which all
//! decline through the same case system — and has at least one in-scope `tn`: the regular
//! classes 1–49, the pronouns' irregular tn 101 (whose forms come from the exception
//! registry, not the rule generator), or a compound class — tn 50 (head inflects, modifier
//! frozen) or tn 51 (both parts inflect) — which the engine routes to its compound decliner.
//! Empty-`tn` rows are dropped. Paradigms are deduplicated by `(tn, av)` across homonyms,
//! since `hn` does not affect declension.

use std::collections::HashMap;

use keinontolibrary_core::normalize;

/// Nominal word classes we keep — they all decline through the same case system.
const NOMINALS: [&str; 4] = ["substantiivi", "adjektiivi", "numeraali", "pronomini"];

/// One declension paradigm parsed from the `Taivutustiedot` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KotusParadigm {
    /// Declension class (taivutusnumero): 1–49 for regular nominals, 50/51 for compounds
    /// (head-inflecting / both-parts-inflecting), or 101 for pronouns.
    pub tn: u8,
    /// Gradation letter (astevaihtelu), if any.
    pub av: Option<char>,
    /// Secondary/rare paradigm — parenthesized in the source, e.g. `(5)`.
    pub rare: bool,
}

/// One kept lemma: its paradigms plus the word-class summary across all its rows.
#[derive(Debug, Default, Clone)]
pub struct KotusLemma {
    /// The distinct declension paradigms (deduplicated by `(tn, av)`), primary first.
    pub paradigms: Vec<KotusParadigm>,
    /// True when every reading is a modifier (adjektiivi/numeraali) and none a
    /// substantiivi — modifiers take the bare `-ine` plural comitative, not the noun
    /// citation `-ineen`.
    pub adjective: bool,
}

/// The parsed, filtered lemma inventory: normalized lemma → its distinct paradigms
/// (primary first).
#[derive(Debug, Default)]
pub struct Inventory {
    /// Normalized lemma → paradigms + word-class summary.
    pub lemmas: HashMap<String, KotusLemma>,
    /// Count of nominal rows whose `Taivutustiedot` had no in-scope class (1–49 or 101) —
    /// i.e. compounds and indeclinables that were dropped.
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

/// Parse the whole `Taivutustiedot` field (comma-separated tokens) into paradigms, keeping
/// the regular nominal classes 1–49, the pronoun class 101, and the compound class 50.
///
/// The compound classes both decline on the final component: tn50 freezes the modifier, tn51
/// inflects it too. The engine routes each to its compound decliner. Co-listing precedence:
/// a regular class 1–49 (e.g. `villiviini` → `5, 50`) declines the whole word correctly, so
/// it supersedes any compound tag (keeping both would only create a spurious ambiguity); and
/// tn51 (both inflect, the fuller reading) supersedes a co-listed tn50 (`isoveli` → `51, 50`).
fn parse_paradigms(field: &str) -> Vec<KotusParadigm> {
    let mut ps: Vec<KotusParadigm> = field
        .split(',')
        .filter_map(parse_token)
        .filter(|p| (1..=49).contains(&p.tn) || p.tn == 50 || p.tn == 51 || p.tn == 101)
        .collect();
    if ps.iter().any(|p| (1..=49).contains(&p.tn)) {
        ps.retain(|p| p.tn != 50 && p.tn != 51);
    } else if ps.iter().any(|p| p.tn == 51) {
        ps.retain(|p| p.tn != 50);
    }
    ps
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
            // Keep all nominals — they share the declension classes. Pronouns are nominals
            // too, carrying the irregular tn 101 that `parse_paradigms` keeps alongside 1–49.
            let classes: Vec<&str> = sanaluokka.split([',', ' ']).collect();
            if !classes.iter().any(|w| NOMINALS.contains(w)) {
                inv.skipped_non_nouns += 1;
                continue;
            }
            let paradigms = parse_paradigms(taivutus);
            if paradigms.is_empty() {
                inv.dropped_compounds += 1;
                continue;
            }
            // Modifier-only rows (adjektiivi/numeraali without a substantiivi reading)
            // take the bare -ine comitative; any noun reading wins across homonym rows.
            let modifier_only = !classes.contains(&"substantiivi")
                && classes
                    .iter()
                    .any(|w| matches!(*w, "adjektiivi" | "numeraali"));
            let lemma = normalize(word);
            let entry = inv.lemmas.entry(lemma).or_insert_with(|| KotusLemma {
                paradigms: Vec::new(),
                adjective: modifier_only,
            });
            entry.adjective = entry.adjective && modifier_only;
            merge(&mut entry.paradigms, paradigms);
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
    fn keeps_compound_classes_drops_verbs() {
        let tn = |n| KotusParadigm {
            tn: n,
            av: None,
            rare: false,
        };
        // Both compound classes are kept and routed to their compound decliners.
        assert_eq!(parse_paradigms("50"), vec![tn(50)]);
        assert_eq!(parse_paradigms("51"), vec![tn(51)]);
        // Co-listing precedence: tn51 (both inflect) supersedes a co-listed tn50...
        assert_eq!(parse_paradigms("51, 50"), vec![tn(51)]);
        // ...and a regular class supersedes any compound tag (villiviini `5, 50` -> tn5).
        assert_eq!(parse_paradigms("5, 50"), vec![tn(5)]);
        // Verbs and indeclinables are still dropped.
        assert!(parse_paradigms("53*C").is_empty()); // verb class
        assert!(parse_paradigms("99").is_empty()); // indeclinable
    }

    #[test]
    fn keeps_pronoun_class_101() {
        // Pronouns carry the irregular tn 101 (registry-backed), kept alongside 1–49.
        assert_eq!(
            parse_paradigms("101"),
            vec![KotusParadigm {
                tn: 101,
                av: None,
                rare: false
            }]
        );
    }

    #[test]
    fn inventory_keeps_all_nominals_and_dedups_homonyms() {
        let tsv = "Hakusana\tHomonymia\tSanaluokka\tTaivutustiedot\n\
                   talo\t\tsubstantiivi\t1\n\
                   aarnio\t1\tsubstantiivi\t3\n\
                   aarnio\t2\tsubstantiivi\t3\n\
                   nopea\t\tadjektiivi\t10\n\
                   kolmas\t\tnumeraali\t45\n\
                   juosta\t\tverbi\t74\n\
                   tämä\t\tpronomini\t101\n\
                   aakkosjärjestys\t\tsubstantiivi\t\n";
        let inv = Inventory::parse_str(tsv);
        assert_eq!(
            inv.lemmas["talo"].paradigms,
            vec![KotusParadigm {
                tn: 1,
                av: None,
                rare: false
            }]
        );
        assert!(!inv.lemmas["talo"].adjective);
        // Two homonyms, same tn=3 -> collapsed to one paradigm.
        assert_eq!(inv.lemmas["aarnio"].paradigms.len(), 1);
        // Adjectives and numerals are nominals -> kept (they share the declension classes),
        // and flagged as modifiers (bare -ine comitative).
        assert!(inv.lemmas["nopea"].adjective);
        assert!(inv.lemmas["kolmas"].adjective);
        // Pronouns are nominals too; tn 101 is in scope (registry-backed) -> kept.
        assert_eq!(
            inv.lemmas["tämä"].paradigms,
            vec![KotusParadigm {
                tn: 101,
                av: None,
                rare: false
            }]
        );
        // Verb skipped (non-nominal); compound (empty tn) dropped.
        assert!(!inv.lemmas.contains_key("juosta"));
        assert!(!inv.lemmas.contains_key("aakkosjärjestys"));
        assert_eq!(inv.skipped_non_nouns, 1); // only the verb
    }
}
