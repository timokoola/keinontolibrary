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
