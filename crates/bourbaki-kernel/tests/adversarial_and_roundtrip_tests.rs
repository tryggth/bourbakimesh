//! Tier 3b Differential and Adversarial Inconsistency Test Suite.
//!
//! Falsification hunt targeting False (Contradiction) and round-trip
//! isomorphism verification (CIC -> Strategy -> CIC -> Lean 4 Kernel).

use bourbaki_kernel::{CICDecompiler, LeanEnvironment, StrategyExtractor, Term, VerificationError};

#[test]
fn test_inconsistency_hunt_false_rejection() {
    let env = LeanEnvironment::default_target();

    // 1. Proposition: False
    let false_prop = Term::var("False");

    // 2. Contradictory fake proof attempt: fun (x : Prop) => x
    let bad_proof = Term::lam("x", Term::prop(), Term::var("x"));

    let err = env
        .verify_term("test_adversarial_false_proof", &false_prop, &bad_proof)
        .expect_err("Lean 4 kernel must reject any fake proof for False");

    assert!(matches!(err, VerificationError::TypeCheckFailure { .. }));
}

#[test]
fn test_roundtrip_identity() {
    let env = LeanEnvironment::default_target();

    // (A : Prop) -> A -> A
    let prop_type = Term::pi(
        "A",
        Term::prop(),
        Term::arrow(Term::var("A"), Term::var("A")),
    );

    let original_proof = Term::lam(
        "A",
        Term::prop(),
        Term::lam("h", Term::var("A"), Term::var("h")),
    );

    // 1. Decompile CIC Term -> StrategyTree
    let strategy = CICDecompiler::term_to_strategy("identity", &prop_type, &original_proof)
        .expect("Decompilation of identity must succeed");

    // 2. Re-extract StrategyTree -> CIC Term
    let extracted_term = StrategyExtractor::compile_strategy(&strategy)
        .expect("Re-extraction of identity strategy must succeed");

    // 3. Operational verification with Lean 4 kernel
    let report = env
        .verify_term("test_roundtrip_identity_thm", &prop_type, &extracted_term)
        .expect("Lean 4 kernel verification of round-trip identity must succeed");

    assert!(report.success);
}

#[test]
fn test_roundtrip_weakening() {
    let env = LeanEnvironment::default_target();

    // (A : Prop) -> (B : Prop) -> A -> B -> A
    let prop_type = Term::pi(
        "A",
        Term::prop(),
        Term::pi(
            "B",
            Term::prop(),
            Term::arrow(Term::var("A"), Term::arrow(Term::var("B"), Term::var("A"))),
        ),
    );

    let original_proof = Term::lam(
        "A",
        Term::prop(),
        Term::lam(
            "B",
            Term::prop(),
            Term::lam(
                "a",
                Term::var("A"),
                Term::lam("b", Term::var("B"), Term::var("a")),
            ),
        ),
    );

    // 1. Decompile
    let strategy = CICDecompiler::term_to_strategy("weakening", &prop_type, &original_proof)
        .expect("Decompilation must succeed");

    // 2. Re-extract
    let extracted_term =
        StrategyExtractor::compile_strategy(&strategy).expect("Re-extraction must succeed");

    // 3. Verify in Lean 4 kernel
    let report = env
        .verify_term("test_roundtrip_weakening_thm", &prop_type, &extracted_term)
        .expect("Lean 4 kernel verification of weakening must succeed");

    assert!(report.success);
}

#[test]
fn test_roundtrip_modus_ponens() {
    let env = LeanEnvironment::default_target();

    // (A : Prop) -> (B : Prop) -> A -> (A -> B) -> B
    let prop_type = Term::pi(
        "A",
        Term::prop(),
        Term::pi(
            "B",
            Term::prop(),
            Term::arrow(
                Term::var("A"),
                Term::arrow(Term::arrow(Term::var("A"), Term::var("B")), Term::var("B")),
            ),
        ),
    );

    let original_proof = Term::lam(
        "A",
        Term::prop(),
        Term::lam(
            "B",
            Term::prop(),
            Term::lam(
                "a",
                Term::var("A"),
                Term::lam(
                    "f",
                    Term::arrow(Term::var("A"), Term::var("B")),
                    Term::app(Term::var("f"), Term::var("a")),
                ),
            ),
        ),
    );

    let strategy = CICDecompiler::term_to_strategy("modus_ponens", &prop_type, &original_proof)
        .expect("Decompilation must succeed");

    let extracted_term =
        StrategyExtractor::compile_strategy(&strategy).expect("Re-extraction must succeed");

    let report = env
        .verify_term(
            "test_roundtrip_modus_ponens_thm",
            &prop_type,
            &extracted_term,
        )
        .expect("Lean 4 kernel verification of modus ponens must succeed");

    assert!(report.success);
}
