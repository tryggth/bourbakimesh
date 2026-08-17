//! Integration test suite for Zero-Trust Lean 4 kernel verification bridge.

use bourbaki_kernel::{LeanEnvironment, Term, VerificationError};

#[test]
fn test_lean_verify_identity() {
    let env = LeanEnvironment::default_target();

    // Prop type: (A : Prop) -> A -> A
    let prop_type = Term::pi(
        "A",
        Term::prop(),
        Term::arrow(Term::var("A"), Term::var("A")),
    );

    // Proof term: fun (A : Prop) => fun (h : A) => h
    let proof_term = Term::lam(
        "A",
        Term::prop(),
        Term::lam("h", Term::var("A"), Term::var("h")),
    );

    let report = env
        .verify_term("test_kernel_identity", &prop_type, &proof_term)
        .expect("Lean 4 kernel verification of identity must succeed");

    assert!(report.success);
    assert_eq!(report.theorem_name, "test_kernel_identity");
    assert!(report
        .emitted_code
        .contains("theorem test_kernel_identity : (A : Prop) -> A -> A :="));
}

#[test]
fn test_lean_verify_weakening_k_combinator() {
    let env = LeanEnvironment::default_target();

    // Prop type: (A : Prop) -> (B : Prop) -> A -> B -> A
    let prop_type = Term::pi(
        "A",
        Term::prop(),
        Term::pi(
            "B",
            Term::prop(),
            Term::arrow(Term::var("A"), Term::arrow(Term::var("B"), Term::var("A"))),
        ),
    );

    // Proof term: fun (A : Prop) => fun (B : Prop) => fun (a : A) => fun (b : B) => a
    let proof_term = Term::lam(
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

    let report = env
        .verify_term("test_kernel_weakening", &prop_type, &proof_term)
        .expect("Lean 4 kernel verification of weakening must succeed");

    assert!(report.success);
}

#[test]
fn test_lean_verify_modus_ponens() {
    let env = LeanEnvironment::default_target();

    // Prop type: (A : Prop) -> (B : Prop) -> A -> (A -> B) -> B
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

    // Proof term: fun (A : Prop) => fun (B : Prop) => fun (a : A) => fun (f : A -> B) => f a
    let proof_term = Term::lam(
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

    let report = env
        .verify_term("test_kernel_modus_ponens", &prop_type, &proof_term)
        .expect("Lean 4 kernel verification of modus ponens must succeed");

    assert!(report.success);
}

#[test]
fn test_lean_negative_typecheck_failure() {
    let env = LeanEnvironment::default_target();

    // Proposition: (A : Prop) -> (B : Prop) -> A -> B (False claim)
    let prop_type = Term::pi(
        "A",
        Term::prop(),
        Term::pi(
            "B",
            Term::prop(),
            Term::arrow(Term::var("A"), Term::var("B")),
        ),
    );

    // Ill-typed proof: fun (A : Prop) => fun (B : Prop) => fun (a : A) => a (returns a:A instead of B)
    let bad_proof = Term::lam(
        "A",
        Term::prop(),
        Term::lam(
            "B",
            Term::prop(),
            Term::lam("a", Term::var("A"), Term::var("a")),
        ),
    );

    let err = env
        .verify_term("test_ill_typed_failure", &prop_type, &bad_proof)
        .expect_err("Lean 4 kernel must reject ill-typed proof");

    match err {
        VerificationError::TypeCheckFailure { diagnostics, code } => {
            assert!(diagnostics.contains("error:") || diagnostics.contains("type"));
            assert!(code.contains("test_ill_typed_failure"));
        }
        other => panic!("Expected TypeCheckFailure, got {:?}", other),
    }

    // Negative test with `sorry`
    let sorry_code = "import LeanTarget.Harness\ntheorem test_sorry (A : Prop) : A := sorry\n";
    let sorry_err = env
        .verify_raw_lean("test_sorry", sorry_code)
        .expect_err("Lean 4 kernel must reject unverified sorry");

    assert!(matches!(
        sorry_err,
        VerificationError::TypeCheckFailure { .. }
    ));
}

#[test]
fn test_lean_verify_cut_lemma_let_binding() {
    let env = LeanEnvironment::default_target();

    // Prop type: (A : Prop) -> (B : Prop) -> (A -> B) -> A -> B
    let prop_type = Term::pi(
        "A",
        Term::prop(),
        Term::pi(
            "B",
            Term::prop(),
            Term::arrow(
                Term::arrow(Term::var("A"), Term::var("B")),
                Term::arrow(Term::var("A"), Term::var("B")),
            ),
        ),
    );

    // Proof term with let-bound intermediate lemma:
    // fun (A : Prop) => fun (B : Prop) => fun (f : A -> B) => fun (a : A) =>
    //   let lem_0 : B := f a; lem_0
    let proof_term = Term::lam(
        "A",
        Term::prop(),
        Term::lam(
            "B",
            Term::prop(),
            Term::lam(
                "f",
                Term::arrow(Term::var("A"), Term::var("B")),
                Term::lam(
                    "a",
                    Term::var("A"),
                    Term::let_in(
                        "lem_0",
                        Term::var("B"),
                        Term::app(Term::var("f"), Term::var("a")),
                        Term::var("lem_0"),
                    ),
                ),
            ),
        ),
    );

    let report = env
        .verify_term("test_kernel_cut_lemma", &prop_type, &proof_term)
        .expect("Lean 4 kernel verification of cut lemma must succeed");

    assert!(report.success);
    assert_eq!(report.theorem_name, "test_kernel_cut_lemma");
    assert!(report.emitted_code.contains("let lem_0 : B := f a; lem_0"));
}
