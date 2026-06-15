//! Property tests for the rule generator (#40): it must never panic on arbitrary
//! lemma-like input, and the locative endings it produces must respect vowel harmony.

use keinontolibrary_core::{Case, Number};
use keinontolibrary_rules::generate;
use proptest::prelude::*;

const ALL_CASES: [Case; 15] = Case::ALL;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1500))]

    // No (lemma, tn, av, number, case) may panic — the many `strip_suffix(...)?` and
    // char-boundary operations must degrade to `None`, never index past a boundary.
    #[test]
    fn generate_never_panics(
        lemma in "[a-zåäö'-]{1,16}",
        tn in 1u8..=51,
        av in proptest::option::of(prop::sample::select(
            "ABCDEFGHIJKLM".chars().collect::<Vec<_>>())),
        adjective in any::<bool>(),
        front in proptest::option::of(any::<bool>()),
    ) {
        for number in Number::ALL {
            for case in ALL_CASES {
                let _ = generate(&lemma, tn, av, adjective, front, number, case);
            }
        }
    }

    // A back-harmonic tn1 lemma (only a/o/u among its vowels) takes back endings; the
    // front counterpart takes front endings. Checked on the unambiguous locatives.
    #[test]
    fn locative_harmony_follows_the_stem(
        stem in "[bcdfghjklmnprstv][aou][bcdfghjklmnprstv]",
    ) {
        let back = format!("{stem}o"); // e.g. "talo" shape: only back vowels
        let front: String = back.chars().map(flip_vowel).collect();

        for (lemma, want_back) in [(back.as_str(), true), (front.as_str(), false)] {
            for (case, back_end, front_end) in [
                (Case::Inessive, "ssa", "ssä"),
                (Case::Adessive, "lla", "llä"),
                (Case::Abessive, "tta", "ttä"),
            ] {
                let forms = generate(lemma, 1, None, false, None, Number::Singular, case)
                    .expect("tn1 generates");
                let f = &forms[0];
                let (want, bad) = if want_back { (back_end, front_end) } else { (front_end, back_end) };
                prop_assert!(f.ends_with(want), "{lemma} {case:?}: {f} should end {want}");
                prop_assert!(!f.ends_with(bad), "{lemma} {case:?}: {f} should not end {bad}");
            }
        }
    }
}

/// Map back vowels/consonants to a front-harmonic counterpart so a back stem becomes a
/// purely front one (a→ä, o→ö, u→y; letters with no harmony pair stay put).
fn flip_vowel(c: char) -> char {
    match c {
        'a' => 'ä',
        'o' => 'ö',
        'u' => 'y',
        other => other,
    }
}
