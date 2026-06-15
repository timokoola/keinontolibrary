//! End-to-end compound-noun harmony: a compound declines on its final component, so vowel
//! harmony follows that component even when the modifier prefix has the opposite harmony.
//! Regression for koirankeksi/beaujolaisviini coming out as -ssa instead of -ssä.

use std::collections::HashMap;

use keinontolibrary_core::{
    Case, Engine, Forms, MemoryStore, Number, ParadigmRef, PluralHead, Source,
};
use keinontolibrary_rules::RuleEngine;

/// Engine whose lookup only knows the *paradigm* of each component (one seeded slot, so
/// `resolve()` finds it); every actual form is produced by the real rule engine.
fn engine() -> Engine {
    let mut store = MemoryStore::new();
    for lemma in ["keksi", "viini"] {
        store.insert(
            lemma,
            ParadigmRef::new(None, 5),
            Number::Singular,
            Case::Nominative,
            Forms::present(vec![lemma.into()], Source::Lookup),
        );
    }
    Engine::builder()
        .lookup(Box::new(store))
        .generator(Box::new(RuleEngine::new()))
        .build()
}

fn form(e: &Engine, lemma: &str, number: Number, case: Case) -> String {
    e.decline(lemma, number, case)
        .unwrap()
        .primary()
        .unwrap()
        .to_string()
}

/// Engine knowing a handful of heads/modifiers, to exercise the frontier resolvers:
/// bound-prefix splits, hyphen boundaries, 2-char heads, and productive class inference.
fn engine_frontier() -> Engine {
    let mut store = MemoryStore::new();
    for (lemma, tn) in [
        ("auto", 1u8),
        ("nopeus", 40),
        ("väline", 48),
        ("yö", 19),
        ("aamu", 1),
    ] {
        store.insert(
            lemma,
            ParadigmRef::new(None, tn),
            Number::Singular,
            Case::Nominative,
            Forms::present(vec![lemma.into()], Source::Lookup),
        );
    }
    Engine::builder()
        .lookup(Box::new(store))
        .generator(Box::new(RuleEngine::new()))
        .build()
}

#[test]
fn plural_head_compound_resolves() {
    let mut store = MemoryStore::new();
    for (lemma, tn) in [("ajo", 1u8), ("valo", 1)] {
        store.insert(
            lemma,
            ParadigmRef::new(None, tn),
            Number::Singular,
            Case::Nominative,
            Forms::present(vec![lemma.into()], Source::Lookup),
        );
    }
    // Reverse index: the plural-nominative surface `valot` -> head lemma `valo`.
    let mut index = HashMap::new();
    index.insert(
        "valot".to_string(),
        PluralHead {
            lemma: "valo".into(),
            reference: ParadigmRef::new(None, 1),
        },
    );
    let e = Engine::builder()
        .lookup(Box::new(store))
        .generator(Box::new(RuleEngine::new()))
        .plural_index(index)
        .build();
    // ajo (known modifier) + valot (plural of valo) -> decline valo in the plural.
    assert_eq!(
        form(&e, "ajovalot", Number::Plural, Case::Inessive),
        "ajovaloissa"
    );
    assert_eq!(
        form(&e, "ajovalot", Number::Plural, Case::Ablative),
        "ajovaloilta"
    );
    // A plural lemma has no singular.
    assert!(e
        .decline("ajovalot", Number::Singular, Case::Inessive)
        .is_err());
    // Without a plausible modifier prefix, no false split.
    assert!(e
        .decline("xyvalot", Number::Plural, Case::Inessive)
        .is_err());
    // Simplex plurale tantum: the whole word is a plural in the index (no prefix).
    assert_eq!(
        form(&e, "valot", Number::Plural, Case::Inessive),
        "valoissa"
    );
    // Explicit hyphen boundary with a plural head.
    assert_eq!(
        form(&e, "tv-valot", Number::Plural, Case::Inessive),
        "tv-valoissa"
    );
}

#[test]
fn combining_head_and_nested_compounds() {
    // Heads that are not free Kotus words (kulmio, niekka) decline via the combining-head
    // registry; a nested compound (jää + viileäkaappi, itself viileä + kaappi) recurses.
    let mut store = MemoryStore::new();
    for (lemma, tn, av) in [
        ("moni", 5u8, None),
        ("kynä", 10, None),
        ("jää", 18, None),
        ("viileä", 10, None),
        ("kaappi", 5, Some('B')),
        ("viileäkaappi", 50, None), // a tn50 compound headword
    ] {
        store.insert(
            lemma,
            ParadigmRef::new(None, tn).with_av(av),
            Number::Singular,
            Case::Nominative,
            Forms::present(vec![lemma.into()], Source::Lookup),
        );
    }
    let e = Engine::builder()
        .lookup(Box::new(store))
        .generator(Box::new(RuleEngine::new()))
        .build();
    // combining head kulmio (tn3) behind a numeral combining-form prefix.
    assert_eq!(
        form(&e, "monikulmio", Number::Singular, Case::Inessive),
        "monikulmiossa"
    );
    // combining head niekka (tn9*A) behind a known modifier.
    assert_eq!(
        form(&e, "kynäniekka", Number::Singular, Case::Inessive),
        "kynäniekassa"
    );
    // nested compound: jää + viileäkaappi (which is viileä + kaappi) -> recurse.
    assert_eq!(
        form(&e, "jääviileäkaappi", Number::Singular, Case::Inessive),
        "jääviileäkaapissa"
    );
}

#[test]
fn paradigm_agrees_with_decline_for_compounds() {
    // Regression: paradigm() once built empty slots for compounds where decline() served a
    // form (the two used different code paths). They now share one per-slot router.
    let e = engine(); // knows keksi, viini
    for (w, n, c) in [
        ("koirankeksi", Number::Singular, Case::Inessive),
        ("koirankeksi", Number::Plural, Case::Inessive),
        ("beaujolaisviini", Number::Singular, Case::Adessive),
    ] {
        let from_decline = e.decline(w, n, c).unwrap().primary().unwrap().to_string();
        let from_paradigm = e
            .paradigm(w)
            .unwrap()
            .get(n, c)
            .variants
            .first()
            .cloned()
            .unwrap_or_default();
        assert_eq!(from_decline, from_paradigm, "{w} {n:?} {c:?}");
    }
}

#[test]
fn compound_numerals_decline_both_parts() {
    // The numeral parts (kahdeksan/kaksi tn10/31, kymmenen tn32, sata tn9, kahdes tn45)
    // come from the real rule engine + registry; seed only their paradigms so resolve finds
    // them. tuhat is registry-served.
    let mut store = MemoryStore::new();
    for (lemma, tn, av) in [
        ("kahdeksan", 10u8, None),
        ("kaksi", 31, None),
        ("kolme", 8, None),
        ("kymmenen", 32, None),
        ("sata", 9, Some('F')), // t:d gradation -> sadassa
        ("kahdes", 45, None),
    ] {
        store.insert(
            lemma,
            ParadigmRef::new(None, tn).with_av(av),
            Number::Singular,
            Case::Nominative,
            Forms::present(vec![lemma.into()], Source::Lookup),
        );
    }
    let e = Engine::builder()
        .lookup(Box::new(store))
        .generator(Box::new(RuleEngine::new()))
        .build();
    let g = |w, c| form(&e, w, Number::Singular, c);
    // Cardinal X+kymmentä: both parts inflect; base reads partitive in the nominative.
    assert_eq!(
        g("kahdeksankymmentä", Case::Nominative),
        "kahdeksankymmentä"
    );
    assert_eq!(g("kahdeksankymmentä", Case::Genitive), "kahdeksankymmenen");
    assert_eq!(
        g("kahdeksankymmentä", Case::Inessive),
        "kahdeksassakymmenessä"
    );
    // Cardinal X+sataa.
    assert_eq!(g("kaksisataa", Case::Inessive), "kahdessasadassa");
    // Teen X+toista: leading numeral inflects, toista frozen.
    assert_eq!(g("kaksitoista", Case::Inessive), "kahdessatoista");
    // Ordinal-teen: kahdes (tn45) + toista.
    assert_eq!(g("kahdestoista", Case::Genitive), "kahdennentoista");
}

#[test]
fn frontier_resolvers_split_and_infer() {
    let e = engine_frontier();
    // Bound prefix (avo-, ali-) + known head, even though the prefix is too short to be a
    // free word.
    assert_eq!(
        form(&e, "avoauto", Number::Singular, Case::Genitive),
        "avoauton"
    );
    assert_eq!(
        form(&e, "alinopeus", Number::Singular, Case::Genitive),
        "alinopeuden"
    );
    // Explicit hyphen boundary: known head after the last hyphen, frozen prefix kept.
    assert_eq!(
        form(&e, "av-väline", Number::Singular, Case::Genitive),
        "av-välineen"
    );
    // 2-char head (yö) behind a known modifier (aamu).
    assert_eq!(
        form(&e, "aamuyö", Number::Singular, Case::Genitive),
        "aamuyön"
    );
    // Productive class inference (no lookup, no compound head): -nen -> tn38.
    assert_eq!(
        form(&e, "ahdaskatseinen", Number::Singular, Case::Genitive),
        "ahdaskatseisen"
    );
    // epä- is a productive bound prefix (epä + known head).
    assert_eq!(
        form(&e, "epäauto", Number::Singular, Case::Genitive),
        "epäauton"
    );
    // A simplex word with no inferable class still errors (not a false split).
    assert!(e
        .decline("pökkylä", Number::Singular, Case::Genitive)
        .is_err());
}

#[test]
fn compound_harmony_follows_final_component() {
    let e = engine();
    // koira (back) + keksi (front) -> front endings via the rule engine.
    assert_eq!(
        form(&e, "koirankeksi", Number::Singular, Case::Inessive),
        "koirankeksissä"
    );
    assert_eq!(
        form(&e, "koirankeksi", Number::Singular, Case::Adessive),
        "koirankeksillä"
    );
    assert_eq!(
        form(&e, "koirankeksi", Number::Plural, Case::Inessive),
        "koirankekseissä"
    );
    // beaujolais (back) + viini (front) -> front.
    assert_eq!(
        form(&e, "beaujolaisviini", Number::Singular, Case::Inessive),
        "beaujolaisviinissä"
    );
    // sanity: the bare components are unchanged.
    assert_eq!(
        form(&e, "keksi", Number::Singular, Case::Inessive),
        "keksissä"
    );
}

/// Engine that knows punaviini/puna/viini/laviini and koira/keksi/koirankeksi as plain tn
/// lemmas — but NOT "la" and NOT the linking form "koiran".
fn engine_known() -> Engine {
    let mut store = MemoryStore::new();
    for (lemma, tn) in [
        ("puna", 10u8),
        ("viini", 5),
        ("punaviini", 5),
        ("laviini", 5),
        ("koira", 10),
        ("keksi", 5),
        ("koirankeksi", 5),
    ] {
        store.insert(
            lemma,
            ParadigmRef::new(None, tn),
            Number::Singular,
            Case::Nominative,
            Forms::present(vec![lemma.into()], Source::Lookup),
        );
    }
    Engine::builder()
        .lookup(Box::new(store))
        .generator(Box::new(RuleEngine::new()))
        .build()
}

#[test]
fn known_compound_harmony_is_overridden() {
    // punaviini is a known tn5 lemma; the whole-word rule would back-harmonize (puna), but
    // the final component viini is front. Prefix "puna" IS a known lemma, so we override.
    let e = engine_known();
    assert_eq!(
        form(&e, "punaviini", Number::Singular, Case::Partitive),
        "punaviiniä"
    );
    assert_eq!(
        form(&e, "punaviini", Number::Plural, Case::Inessive),
        "punaviineissä"
    );
}

#[test]
fn genitive_linked_known_compound_is_overridden() {
    // koirankeksi is a known tn5 lemma whose modifier links with the genitive -n ("koiran").
    // "koiran" is not itself a lemma, but stripping the linker yields "koira", which is — so
    // harmony must follow the front-harmonic "keksi": koirankeksi -> koirankekse(i)ssä, not -ssa.
    let e = engine_known();
    assert_eq!(
        form(&e, "koirankeksi", Number::Singular, Case::Inessive),
        "koirankeksissä"
    );
    assert_eq!(
        form(&e, "koirankeksi", Number::Plural, Case::Inessive),
        "koirankekseissä"
    );
}

#[test]
fn non_compound_ending_in_known_word_is_left_alone() {
    // laviini ends in "viini" but is NOT a compound — "la" is not a known lemma — so harmony
    // stays back (the rule's result). This is the laviini false-positive guard.
    let e = engine_known();
    assert_eq!(
        form(&e, "laviini", Number::Singular, Case::Partitive),
        "laviinia"
    );
    assert_eq!(
        form(&e, "laviini", Number::Plural, Case::Inessive),
        "laviineissa"
    );
}

// Compound ordinals inflect BOTH parts (Voikko-verified: kahdennenkymmenennen,
// kahdennessakymmenennessä, kahdensissakymmenensissä). Cycle 10b of the 100% roadmap.
#[test]
fn compound_ordinals_decline_both_parts() {
    use keinontolibrary_core::{Case, Engine, MemoryStore, Number};
    use keinontolibrary_rules::RuleEngine;
    // The lemmas must resolve: insert citation slots for the parts and the compound.
    let mut store = MemoryStore::new();
    for (lemma, tn) in [("kahdes", 45), ("kymmenes", 45), ("kahdeskymmenes", 45)] {
        store.insert(
            lemma,
            keinontolibrary_core::ParadigmRef::new(None, tn),
            keinontolibrary_core::Number::Singular,
            keinontolibrary_core::Case::Nominative,
            keinontolibrary_core::Forms::present(
                vec![lemma.to_owned()],
                keinontolibrary_core::Source::Lookup,
            ),
        );
    }
    let e = Engine::builder()
        .lookup(Box::new(store))
        .generator(Box::new(RuleEngine::new()))
        .build();
    let f = |n, c| {
        e.decline("kahdeskymmenes", n, c)
            .unwrap()
            .primary()
            .unwrap()
            .to_string()
    };
    assert_eq!(f(Number::Singular, Case::Genitive), "kahdennenkymmenennen");
    assert_eq!(
        f(Number::Singular, Case::Inessive),
        "kahdennessakymmenennessä"
    );
    assert_eq!(
        f(Number::Plural, Case::Inessive),
        "kahdensissakymmenensissä"
    );
}
