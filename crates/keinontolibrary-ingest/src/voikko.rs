//! Parsing the Voikko-generated JSONL corpus into clean noun surface forms.
//!
//! Each row is one inflected form. We keep only simple noun forms: dropping adjectives,
//! verbs, participles, comparison, enclitics (focus/question clitics) and possessive
//! suffixes — except the comitative's obligatory 3rd-person form, which is its citation
//! form. The surface form is the `BOOKWORD` field.

use keinontolibrary_core::{normalize, Case, Number};
use serde::Deserialize;

/// A surviving, mapped noun form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanForm {
    /// Normalized lemma (from `BASEFORM`).
    pub lemma: String,
    /// Declension class.
    pub tn: u8,
    /// Gradation letter, if any (`_` in the source means none).
    pub av: Option<char>,
    /// Grammatical number.
    pub number: Number,
    /// Case.
    pub case: Case,
    /// The surface form (`BOOKWORD`), kept verbatim.
    pub form: String,
}

/// Raw JSONL row. Unused fields are still listed so their presence can be tested.
#[derive(Debug, Deserialize)]
struct RawRow {
    #[serde(rename = "BASEFORM")]
    baseform: Option<String>,
    tn: Option<u8>,
    av: Option<String>,
    #[serde(rename = "CLASS")]
    class: Option<String>,
    #[serde(rename = "NUMBER")]
    number: Option<String>,
    #[serde(rename = "SIJAMUOTO")]
    sijamuoto: Option<String>,
    #[serde(rename = "BOOKWORD")]
    bookword: Option<String>,
    #[serde(rename = "FSTOUTPUT")]
    fstoutput: Option<String>,
    #[serde(rename = "COMPARISON")]
    comparison: Option<String>,
    #[serde(rename = "PARTICIPLE")]
    participle: Option<String>,
    #[serde(rename = "FOCUS")]
    focus: Option<String>,
    #[serde(rename = "KYSYMYSLIITE")]
    kysymysliite: Option<String>,
    #[serde(rename = "POSSESSIVE")]
    possessive: Option<String>,
    #[serde(rename = "MOOD")]
    mood: Option<String>,
    #[serde(rename = "TENSE")]
    tense: Option<String>,
    #[serde(rename = "PERSON")]
    person: Option<String>,
    #[serde(rename = "NEGATIVE")]
    negative: Option<String>,
}

/// Map a Voikko `SIJAMUOTO` token to a [`Case`].
///
/// `kohdanto` (accusative) never appears in the corpus and is derived during the build, so
/// it is intentionally absent here. Tokens outside the 15 supported cases (e.g.
/// `kerrontosti`) map to `None` and are dropped.
#[must_use]
pub fn case_from_sijamuoto(token: &str) -> Option<Case> {
    Some(match token {
        "nimento" => Case::Nominative,
        "omanto" => Case::Genitive,
        "osanto" => Case::Partitive,
        "sisaolento" => Case::Inessive,
        "sisaeronto" => Case::Elative,
        "sisatulento" => Case::Illative,
        "ulkoolento" => Case::Adessive,
        "ulkoeronto" => Case::Ablative,
        "ulkotulento" => Case::Allative,
        "olento" => Case::Essive,
        "tulento" => Case::Translative,
        "vajanto" => Case::Abessive,
        "seuranto" => Case::Comitative,
        "keinonto" => Case::Instructive,
        _ => return None,
    })
}

fn number_from(token: &str) -> Option<Number> {
    match token {
        "singular" => Some(Number::Singular),
        "plural" => Some(Number::Plural),
        _ => None,
    }
}

impl RawRow {
    /// Apply all filters and map to a [`CleanForm`], or `None` if the row is out of scope.
    fn clean(self) -> Option<CleanForm> {
        // Nouns only. Minimal rows (the clean base forms, e.g. `talo`) carry no CLASS
        // field at all and must be kept; only reject rows that are explicitly a non-noun
        // class (adjective/verb/numeral/pronoun/...). Non-noun lemmas are dropped anyway
        // at the Kotus join.
        match self.class.as_deref() {
            None | Some("nimisana" | "nimisana_laatusana") => {}
            Some(_) => return None,
        }
        // Drop comparison (non-positive), participles, and verb features.
        if self.comparison.as_deref().is_some_and(|c| c != "positive") {
            return None;
        }
        if self.participle.is_some()
            || self.mood.is_some()
            || self.tense.is_some()
            || self.person.is_some()
            || self.negative.is_some()
        {
            return None;
        }
        // Drop enclitics. The `-kin/-kaan` and `-ko/-kö` clitics set FOCUS/KYSYMYSLIITE,
        // but `-han/-hän/-pa/-pä/-s` are only marked by the `[Ef]` enclitic-focus tag in
        // FSTOUTPUT, so check all three signals.
        if self.focus.is_some() || self.kysymysliite.is_some() {
            return None;
        }
        if self
            .fstoutput
            .as_deref()
            .is_some_and(|f| f.contains("[Ef]"))
        {
            return None;
        }

        let sijamuoto = self.sijamuoto.as_deref()?;
        let case = case_from_sijamuoto(sijamuoto)?;

        // Possessive suffixes are out of scope, except the comitative's obligatory
        // 3rd-person form (`-ineen`), which is the citation form we keep.
        if let Some(poss) = self.possessive.as_deref() {
            if !(case == Case::Comitative && poss == "3") {
                return None;
            }
        } else if case == Case::Comitative {
            // The comitative citation form carries the 3rd-person suffix; the bare
            // possessive-less form is not the form we publish.
            return None;
        }

        let number = number_from(self.number.as_deref()?)?;
        let tn = self.tn?;
        let form = self.bookword?;
        if form.is_empty() {
            return None;
        }
        let lemma = normalize(&self.baseform?);
        if lemma.is_empty() {
            return None;
        }
        let av = match self.av.as_deref() {
            Some("_") | None => None,
            Some(s) => s.chars().next(),
        };
        Some(CleanForm {
            lemma,
            tn,
            av,
            number,
            case,
            form,
        })
    }
}

/// Parse one shard of JSONL text into clean forms, in file order.
///
/// Malformed lines are skipped silently (counted by the caller via the returned length vs
/// input line count if desired).
#[must_use]
pub fn parse_shard(text: &str) -> Vec<CleanForm> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<RawRow>(line).ok())
        .filter_map(RawRow::clean)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean_one(json: &str) -> Option<CleanForm> {
        serde_json::from_str::<RawRow>(json)
            .ok()
            .and_then(RawRow::clean)
    }

    #[test]
    fn keeps_basic_noun_form() {
        let row = r#"{"BASEFORM":"kevät","tn":44,"av":"_","CLASS":"nimisana",
            "NUMBER":"singular","SIJAMUOTO":"ulkoolento","BOOKWORD":"keväällä"}"#;
        let f = clean_one(row).unwrap();
        assert_eq!(f.lemma, "kevät");
        assert_eq!(f.case, Case::Adessive);
        assert_eq!(f.number, Number::Singular);
        assert_eq!(f.form, "keväällä");
        assert_eq!(f.tn, 44);
    }

    #[test]
    fn drops_adjective_verb_comparison_participle() {
        let adj = r#"{"BASEFORM":"painava","tn":10,"CLASS":"laatusana",
            "NUMBER":"singular","SIJAMUOTO":"omanto","BOOKWORD":"painavan"}"#;
        assert!(clean_one(adj).is_none());
        let comp = r#"{"BASEFORM":"suuri","tn":26,"CLASS":"nimisana","COMPARISON":"superlative",
            "NUMBER":"plural","SIJAMUOTO":"nimento","BOOKWORD":"suurimmat"}"#;
        assert!(clean_one(comp).is_none());
    }

    #[test]
    fn drops_enclitics_and_possessives() {
        let focus = r#"{"BASEFORM":"talo","tn":1,"CLASS":"nimisana","FOCUS":"kin",
            "NUMBER":"singular","SIJAMUOTO":"nimento","BOOKWORD":"talokin"}"#;
        assert!(clean_one(focus).is_none());
        let poss = r#"{"BASEFORM":"naapuri","tn":6,"CLASS":"nimisana","POSSESSIVE":"3",
            "NUMBER":"singular","SIJAMUOTO":"nimento","BOOKWORD":"naapurinsa"}"#;
        assert!(clean_one(poss).is_none());
    }

    #[test]
    fn keeps_comitative_third_person_citation_form() {
        let com = r#"{"BASEFORM":"lahko","tn":1,"av":"_","CLASS":"nimisana","POSSESSIVE":"3",
            "NUMBER":"plural","SIJAMUOTO":"seuranto","BOOKWORD":"lahkoineen"}"#;
        let f = clean_one(com).unwrap();
        assert_eq!(f.case, Case::Comitative);
        assert_eq!(f.form, "lahkoineen");
        // ...but a comitative without the suffix is dropped.
        let bare = r#"{"BASEFORM":"lahko","tn":1,"CLASS":"nimisana",
            "NUMBER":"plural","SIJAMUOTO":"seuranto","BOOKWORD":"lahkoine"}"#;
        assert!(clean_one(bare).is_none());
    }

    #[test]
    fn drops_unmapped_case_and_bad_number() {
        let sti = r#"{"BASEFORM":"nopea","tn":10,"CLASS":"nimisana",
            "NUMBER":"singular","SIJAMUOTO":"kerrontosti","BOOKWORD":"nopeasti"}"#;
        assert!(clean_one(sti).is_none());
    }

    #[test]
    fn keeps_minimal_base_row_without_class() {
        // The clean nominative citation rows carry no CLASS/FSTOUTPUT.
        let row = r#"{"av":"_","tn":1,"word":"talo","BASEFORM":"talo","BOOKWORD":"talo",
            "SIJAMUOTO":"nimento","NUMBER":"singular"}"#;
        let f = clean_one(row).unwrap();
        assert_eq!(f.form, "talo");
        assert_eq!(f.case, Case::Nominative);
    }

    #[test]
    fn drops_focus_clitic_marked_only_in_fstoutput() {
        // `talohan` has no FOCUS field; the clitic is marked solely by `[Ef]` in FSTOUTPUT.
        let row = r#"{"av":"_","tn":1,"word":"talo","BASEFORM":"talo","BOOKWORD":"talohan",
            "CLASS":"nimisana","FSTOUTPUT":"[Ln][Xp]talo[X]talo[Sn][Ny]han[Ef]",
            "SIJAMUOTO":"nimento","NUMBER":"singular"}"#;
        assert!(clean_one(row).is_none());
    }

    #[test]
    fn gradation_letter_parsed() {
        let row = r#"{"BASEFORM":"aallokko","tn":4,"av":"A","CLASS":"nimisana",
            "NUMBER":"singular","SIJAMUOTO":"omanto","BOOKWORD":"aallokon"}"#;
        assert_eq!(clean_one(row).unwrap().av, Some('A'));
    }
}
