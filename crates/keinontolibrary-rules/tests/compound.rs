//! End-to-end compound-noun harmony: a compound declines on its final component, so vowel
//! harmony follows that component even when the modifier prefix has the opposite harmony.
//! Regression for koirankeksi/beaujolaisviini coming out as -ssa instead of -ssä.

use keinontolibrary_core::{Case, Engine, Forms, MemoryStore, Number, ParadigmRef, Source};
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
