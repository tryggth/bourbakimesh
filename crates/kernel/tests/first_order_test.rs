use kernel::ast::{DeductionStep, Expr, ProofStatus, Term};
use kernel::state::ProofState;
use std::time::Instant;

#[test]
fn test_universal_modus_ponens() {
    // Initial: h0: ∀x. (P(x) -> Q(x)), h1: P(c) ⊢ Q(c)
    let p_x = Expr::Pred("P".to_string(), vec![Term::Var("x".to_string())]);
    let q_x = Expr::Pred("Q".to_string(), vec![Term::Var("x".to_string())]);
    let forall_impl = Expr::Forall {
        var: "x".to_string(),
        body: Box::new(Expr::Impl(Box::new(p_x), Box::new(q_x))),
    };

    let p_c = Expr::Pred("P".to_string(), vec![Term::Const("c".to_string())]);
    let q_c = Expr::Pred("Q".to_string(), vec![Term::Const("c".to_string())]);

    let initial_hyps = vec![
        ("h0".to_string(), forall_impl),
        ("h1".to_string(), p_c.clone()),
    ];

    let mut state = ProofState::new(initial_hyps, q_c.clone());
    assert_eq!(state.status, ProofStatus::Open);

    let start = Instant::now();

    // Step 1: ForallElim(h0, Const("c")) -> h2: P(c) -> Q(c)
    let h2 = state
        .apply_step(&DeductionStep::ForallElim {
            hyp: "h0".to_string(),
            term: Term::Const("c".to_string()),
        })
        .expect("ForallElim failed")
        .expect("Expected new hyp");
    assert_eq!(h2, "h2");
    assert_eq!(
        state.hyps.get("h2"),
        Some(&Expr::Impl(Box::new(p_c.clone()), Box::new(q_c.clone())))
    );

    // Step 2: ModusPonens(h2, h1) -> h3: Q(c)
    let h3 = state
        .apply_step(&DeductionStep::ModusPonens {
            r#impl: "h2".to_string(),
            arg: "h1".to_string(),
        })
        .expect("ModusPonens failed")
        .expect("Expected new hyp");
    assert_eq!(h3, "h3");
    assert_eq!(state.hyps.get("h3"), Some(&q_c));

    // Step 3: Exact(h3) -> Closes proof
    let close = state
        .apply_step(&DeductionStep::Exact {
            hyp: "h3".to_string(),
        })
        .expect("Exact failed");
    assert_eq!(close, None);
    assert_eq!(state.status, ProofStatus::Proven);

    let elapsed = start.elapsed();
    println!("⏱️ Universal Modus Ponens proof time: {:?}", elapsed);
    assert!(
        elapsed.as_micros() < 500,
        "Kernel step evaluation exceeded latency ceiling"
    );
}

#[test]
fn test_existential_generalization() {
    // Initial: h0: P(c) ⊢ ∃x. P(x)
    let p_c = Expr::Pred("P".to_string(), vec![Term::Const("c".to_string())]);
    let target = Expr::Exists {
        var: "x".to_string(),
        body: Box::new(Expr::Pred(
            "P".to_string(),
            vec![Term::Var("x".to_string())],
        )),
    };

    let mut state = ProofState::new(vec![("h0".to_string(), p_c)], target.clone());
    assert_eq!(state.status, ProofStatus::Open);

    // Step 1: ExistsIntro { hyp: "h0", var: "x", body: Pred("P", [Var("x")]) } -> h1: ∃x. P(x)
    let h1 = state
        .apply_step(&DeductionStep::ExistsIntro {
            hyp: "h0".to_string(),
            var: "x".to_string(),
            body: Expr::Pred("P".to_string(), vec![Term::Var("x".to_string())]),
        })
        .expect("ExistsIntro failed")
        .expect("Expected new hyp");
    assert_eq!(h1, "h1");
    assert_eq!(state.hyps.get("h1"), Some(&target));

    // Step 2: Exact(h1) -> Closes proof
    let res = state
        .apply_step(&DeductionStep::Exact {
            hyp: "h1".to_string(),
        })
        .expect("Exact failed");
    assert_eq!(res, None);
    assert_eq!(state.status, ProofStatus::Proven);
}

#[test]
fn test_leibniz_rewrite() {
    // Initial: h0: Eq(a, b), h1: P(a) ⊢ P(b)
    let a = Term::Const("a".to_string());
    let b = Term::Const("b".to_string());
    let p_a = Expr::Pred("P".to_string(), vec![a.clone()]);
    let p_b = Expr::Pred("P".to_string(), vec![b.clone()]);

    let initial_hyps = vec![
        ("h0".to_string(), Expr::Eq(a.clone(), b.clone())),
        ("h1".to_string(), p_a.clone()),
    ];

    let mut state = ProofState::new(initial_hyps, p_b.clone());
    assert_eq!(state.status, ProofStatus::Open);

    // Step 1: Rewrite { eq_hyp: "h0", target_hyp: "h1" } -> h2: P(b)
    let h2 = state
        .apply_step(&DeductionStep::Rewrite {
            eq_hyp: "h0".to_string(),
            target_hyp: "h1".to_string(),
        })
        .expect("Rewrite failed")
        .expect("Expected new hyp");
    assert_eq!(h2, "h2");
    assert_eq!(state.hyps.get("h2"), Some(&p_b));

    // Step 2: Exact(h2) -> Closes proof
    let res = state
        .apply_step(&DeductionStep::Exact {
            hyp: "h2".to_string(),
        })
        .expect("Exact failed");
    assert_eq!(res, None);
    assert_eq!(state.status, ProofStatus::Proven);
}

#[test]
fn test_existential_elimination() {
    // h0: ∃x. P(x)
    // h1: ∀x. (P(x) -> C)
    // Target: C
    let c = Expr::Prop("C".to_string());
    let p_x = Expr::Pred("P".to_string(), vec![Term::Var("x".to_string())]);

    let h0 = Expr::Exists {
        var: "x".to_string(),
        body: Box::new(p_x.clone()),
    };
    let h1 = Expr::Forall {
        var: "x".to_string(),
        body: Box::new(Expr::Impl(Box::new(p_x), Box::new(c.clone()))),
    };

    let mut state = ProofState::new(
        vec![("h0".to_string(), h0), ("h1".to_string(), h1)],
        c.clone(),
    );

    // Step 1: ExistsElim { hyp_exists: "h0", hyp_impl: "h1" } -> h2: C
    let h2 = state
        .apply_step(&DeductionStep::ExistsElim {
            hyp_exists: "h0".to_string(),
            hyp_impl: "h1".to_string(),
        })
        .expect("ExistsElim failed")
        .expect("Expected new hyp");
    assert_eq!(h2, "h2");
    assert_eq!(state.hyps.get("h2"), Some(&c));

    // Step 2: Exact(h2) -> Closes proof
    let res = state
        .apply_step(&DeductionStep::Exact {
            hyp: "h2".to_string(),
        })
        .expect("Exact failed");
    assert_eq!(res, None);
    assert_eq!(state.status, ProofStatus::Proven);
}
