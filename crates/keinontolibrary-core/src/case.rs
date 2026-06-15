//! Grammatical number and the 15 cases handled by the library.

use std::fmt;
use std::str::FromStr;

/// Grammatical number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Number {
    Singular,
    Plural,
}

impl Number {
    /// Both numbers, in stable order.
    pub const ALL: [Number; 2] = [Number::Singular, Number::Plural];

    /// Stable index (`Singular` = 0, `Plural` = 1) for array-backed paradigm storage.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Lowercase English name, e.g. `"singular"`.
    pub const fn name(self) -> &'static str {
        match self {
            Number::Singular => "singular",
            Number::Plural => "plural",
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Error returned when parsing a [`Number`] or [`Case`] from an unknown string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    kind: &'static str,
    input: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown {}: {:?}", self.kind, self.input)
    }
}

impl std::error::Error for ParseError {}

impl FromStr for Number {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "singular" | "sg" | "yksikkö" => Ok(Number::Singular),
            "plural" | "pl" | "monikko" => Ok(Number::Plural),
            other => Err(ParseError {
                kind: "number",
                input: other.to_owned(),
            }),
        }
    }
}

/// The grammatical cases the library can produce.
///
/// English names are the public surface; the ingester maps Voikko's Finnish `SIJAMUOTO`
/// tokens onto these (see `keinontolibrary-ingest`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Case {
    Nominative,
    Genitive,
    Partitive,
    Accusative,
    Inessive,
    Elative,
    Illative,
    Adessive,
    Ablative,
    Allative,
    Essive,
    Translative,
    Abessive,
    Comitative,
    Instructive,
}

impl Case {
    /// All 15 cases, in stable order.
    pub const ALL: [Case; 15] = [
        Case::Nominative,
        Case::Genitive,
        Case::Partitive,
        Case::Accusative,
        Case::Inessive,
        Case::Elative,
        Case::Illative,
        Case::Adessive,
        Case::Ablative,
        Case::Allative,
        Case::Essive,
        Case::Translative,
        Case::Abessive,
        Case::Comitative,
        Case::Instructive,
    ];

    /// Stable index into [`Case::ALL`], for array-backed paradigm storage.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Lowercase English name, e.g. `"inessive"`.
    pub const fn name(self) -> &'static str {
        match self {
            Case::Nominative => "nominative",
            Case::Genitive => "genitive",
            Case::Partitive => "partitive",
            Case::Accusative => "accusative",
            Case::Inessive => "inessive",
            Case::Elative => "elative",
            Case::Illative => "illative",
            Case::Adessive => "adessive",
            Case::Ablative => "ablative",
            Case::Allative => "allative",
            Case::Essive => "essive",
            Case::Translative => "translative",
            Case::Abessive => "abessive",
            Case::Comitative => "comitative",
            Case::Instructive => "instructive",
        }
    }
}

impl fmt::Display for Case {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Case {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.trim().to_ascii_lowercase();
        // A direct match instead of scanning Case::ALL: the compiler turns this into a
        // jump/hash rather than 15 string comparisons.
        let case = match lower.as_str() {
            "nominative" => Case::Nominative,
            "genitive" => Case::Genitive,
            "partitive" => Case::Partitive,
            "accusative" => Case::Accusative,
            "inessive" => Case::Inessive,
            "elative" => Case::Elative,
            "illative" => Case::Illative,
            "adessive" => Case::Adessive,
            "ablative" => Case::Ablative,
            "allative" => Case::Allative,
            "essive" => Case::Essive,
            "translative" => Case::Translative,
            "abessive" => Case::Abessive,
            "comitative" => Case::Comitative,
            "instructive" => Case::Instructive,
            _ => {
                return Err(ParseError {
                    kind: "case",
                    input: lower,
                })
            }
        };
        Ok(case)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_index_round_trips_through_all() {
        for (i, c) in Case::ALL.into_iter().enumerate() {
            assert_eq!(c.index(), i);
            assert_eq!(Case::ALL[c.index()], c);
        }
    }

    #[test]
    fn number_index_round_trips() {
        for (i, n) in Number::ALL.into_iter().enumerate() {
            assert_eq!(n.index(), i);
        }
    }

    #[test]
    fn case_parses_from_english_name() {
        assert_eq!("Inessive".parse::<Case>().unwrap(), Case::Inessive);
        assert_eq!("  genitive ".parse::<Case>().unwrap(), Case::Genitive);
        assert!("sisaolento".parse::<Case>().is_err());
    }

    #[test]
    fn number_parses_aliases() {
        assert_eq!("PL".parse::<Number>().unwrap(), Number::Plural);
        assert_eq!("singular".parse::<Number>().unwrap(), Number::Singular);
        assert!("dual".parse::<Number>().is_err());
    }
}
