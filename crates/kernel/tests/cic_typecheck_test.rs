//! Unit and Integration Tests for the Calculus of Inductive Constructions (CIC) Core & Type Checker.

use kernel::cic::{
    check_type, infer_type, Environment, Expr, LocalContext, TypeError,
};

#[test]
fn test_identity_term_typecheck() {
    let env = Environment::new();
    let prop = Expr::prop();

    // Context: A : Prop
    let ctx = LocalContext::new().extend("A", "A", prop.clone());

    // Term: λ (x : A) => x (represented as Lam("x", FVar("A"), BVar(0)))
    let id_term = Expr::lam("x", Expr::fvar("A"), Expr::BVar(0));

    // Expected Type: A → A
    let expected_type = Expr::arrow(Expr::fvar("A"), Expr::fvar("A"));

    // Check type
    assert!(check_type(&id_term, &expected_type, &env, &ctx).is_ok());

    // Infer type
    let inferred = infer_type(&id_term, &env, &ctx).expect("Inference should succeed");
    assert_eq!(inferred, Expr::forall_e("x", Expr::fvar("A"), Expr::fvar("A")));
}

#[test]
fn test_modus_ponens_term_application() {
    let env = Environment::new();
    let prop = Expr::prop();

    // Context: A : Prop, B : Prop, h1 : A → B, h2 : A
    let ctx = LocalContext::new()
        .extend("A", "A", prop.clone())
        .extend("B", "B", prop.clone())
        .extend(
            "h1",
            "h1",
            Expr::arrow(Expr::fvar("A"), Expr::fvar("B")),
        )
        .extend("h2", "h2", Expr::fvar("A"));

    // Term: h1 h2 (App(FVar("h1"), FVar("h2")))
    let mp_term = Expr::app(Expr::fvar("h1"), Expr::fvar("h2"));

    // Expected Type: B (FVar("B"))
    let expected_type = Expr::fvar("B");

    assert!(check_type(&mp_term, &expected_type, &env, &ctx).is_ok());

    let inferred = infer_type(&mp_term, &env, &ctx).expect("MP inference should succeed");
    assert_eq!(inferred, Expr::fvar("B"));
}

#[test]
fn test_conjunction_swap_proof_term() {
    // Standard logic environment with And, And.intro, And.left, And.right
    let env = Environment::default_with_logic();
    let prop = Expr::prop();

    // Context: A : Prop, B : Prop
    let ctx = LocalContext::new()
        .extend("A", "A", prop.clone())
        .extend("B", "B", prop.clone());

    // and_ab = And A B
    let and_ab = Expr::mk_app(
        Expr::const_term("And", vec![]),
        vec![Expr::fvar("A"), Expr::fvar("B")],
    );

    // and_ba = And B A
    let and_ba = Expr::mk_app(
        Expr::const_term("And", vec![]),
        vec![Expr::fvar("B"), Expr::fvar("A")],
    );

    // Conjunction swap term:
    // λ (h : And A B) => And.intro B A (And.right A B h) (And.left A B h)
    let left_proj = Expr::mk_app(
        Expr::const_term("And.left", vec![]),
        vec![Expr::fvar("A"), Expr::fvar("B"), Expr::BVar(0)],
    );

    let right_proj = Expr::mk_app(
        Expr::const_term("And.right", vec![]),
        vec![Expr::fvar("A"), Expr::fvar("B"), Expr::BVar(0)],
    );

    let swap_body = Expr::mk_app(
        Expr::const_term("And.intro", vec![]),
        vec![Expr::fvar("B"), Expr::fvar("A"), right_proj, left_proj],
    );

    let swap_proof_term = Expr::lam("h", and_ab.clone(), swap_body);

    // Target theorem type: And A B → And B A
    let theorem_type = Expr::arrow(and_ab, and_ba);

    let result = check_type(&swap_proof_term, &theorem_type, &env, &ctx);
    assert!(result.is_ok(), "Conjunction swap typecheck failed: {:?}", result);
}

#[test]
fn test_polymorphic_identity_term() {
    let env = Environment::new();
    let type_0 = Expr::type_0();
    let ctx = LocalContext::new();

    // Term: λ (A : Type 0) (x : A) => x
    let poly_id = Expr::lam(
        "A",
        type_0.clone(),
        Expr::lam("x", Expr::BVar(0), Expr::BVar(0)),
    );

    // Expected Type: ∀ (A : Type 0) (x : A), A
    let expected_type = Expr::forall_e(
        "A",
        type_0.clone(),
        Expr::forall_e("x", Expr::BVar(0), Expr::BVar(1)),
    );

    let check_res = check_type(&poly_id, &expected_type, &env, &ctx);
    assert!(check_res.is_ok(), "Polymorphic id failed: {:?}", check_res);
}

#[test]
fn test_negative_typecheck_mismatches() {
    let env = Environment::new();
    let prop = Expr::prop();

    // Context: A : Prop, B : Prop, h : A
    let ctx = LocalContext::new()
        .extend("A", "A", prop.clone())
        .extend("B", "B", prop.clone())
        .extend("h", "h", Expr::fvar("A"));

    // Applying a non-function: (h h)
    let bad_app = Expr::app(Expr::fvar("h"), Expr::fvar("h"));
    let res = infer_type(&bad_app, &env, &ctx);
    assert!(matches!(res, Err(TypeError::NotAFunction(_))));

    // Type mismatch: claiming h has type B
    let mismatch_check = check_type(&Expr::fvar("h"), &Expr::fvar("B"), &env, &ctx);
    assert!(matches!(mismatch_check, Err(TypeError::TypeMismatch { .. })));
}
