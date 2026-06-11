//! [`ParadigmRef`] — identifies one inflection paradigm of a lemma.

use std::fmt;

/// Identifies a single paradigm of a lemma, used to disambiguate homonyms and
/// multi-paradigm words.
///
/// A lemma may have several paradigms because it carries multiple homonym numbers
/// (`hn`) and/or multiple declension classes (`tn`) in the Kotus list. `(hn, tn)` is the
/// disambiguating key; `gloss` is an optional human-readable hint used only in error
/// messages.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ParadigmRef {
    /// Kotus homonym number (`Homonymia`), if the lemma has more than one entry.
    pub hn: Option<u8>,
    /// Kotus declension class / taivutusnumero (1–49 for in-scope simple nouns).
    pub tn: u8,
    /// Optional consonant-gradation letter (`av`), e.g. `'C'`.
    pub av: Option<char>,
    /// Whether every reading of the lemma is a modifier (adjective/numeral). Modifiers
    /// take the bare `-ine` plural comitative (`punaisine`); only head nouns carry the
    /// possessive citation `-ineen`/`-inensA` (`taloineen`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub adjective: bool,
    /// Optional human-readable gloss, surfaced only in error messages.
    pub gloss: Option<String>,
}

impl ParadigmRef {
    /// Construct a reference from the disambiguating `(hn, tn)` key.
    pub fn new(hn: Option<u8>, tn: u8) -> Self {
        Self {
            hn,
            tn,
            av: None,
            adjective: false,
            gloss: None,
        }
    }

    /// Builder-style setter for the gradation letter.
    #[must_use]
    pub fn with_av(mut self, av: Option<char>) -> Self {
        self.av = av;
        self
    }

    /// Builder-style setter for the modifier (adjective/numeral) flag.
    #[must_use]
    pub fn with_adjective(mut self, adjective: bool) -> Self {
        self.adjective = adjective;
        self
    }

    /// Builder-style setter for the gloss.
    #[must_use]
    pub fn with_gloss(mut self, gloss: impl Into<String>) -> Self {
        self.gloss = Some(gloss.into());
        self
    }

    /// Whether this reference matches a user-supplied `(hn, tn)` filter.
    ///
    /// A `None` field in the filter matches anything; this lets callers disambiguate by
    /// `tn` alone, by `hn` alone, or by both.
    pub fn matches(&self, hn: Option<u8>, tn: Option<u8>) -> bool {
        hn.is_none_or(|h| self.hn == Some(h)) && tn.is_none_or(|t| self.tn == t)
    }
}

impl fmt::Display for ParadigmRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tn={}", self.tn)?;
        if let Some(hn) = self.hn {
            write!(f, " hn={hn}")?;
        }
        if let Some(av) = self.av {
            write!(f, " av={av}")?;
        }
        if let Some(gloss) = &self.gloss {
            write!(f, " ({gloss})")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_treats_none_as_wildcard() {
        let r = ParadigmRef::new(Some(2), 5);
        assert!(r.matches(None, None));
        assert!(r.matches(None, Some(5)));
        assert!(r.matches(Some(2), None));
        assert!(r.matches(Some(2), Some(5)));
        assert!(!r.matches(Some(1), None));
        assert!(!r.matches(None, Some(7)));
    }
}
