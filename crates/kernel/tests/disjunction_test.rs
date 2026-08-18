use kernel::ast::{DeductionStep, Expr, ProofStatus};
use kernel::state::{KernelError, ProofState};

#[test]
fn test_or_commutativity_via_or_elim() {
    let a = Expr::Prop("A".to_string());
    let b = Expr::Prop("B".to_string());
    let b_or_a = Expr::Or(Box::new(b.clone()), Box::new(a.clone()));

    // h0: A ∨ B
    // h1: A -> (B ∨ A)
    // h2: B -> (B ∨ A)
    // Target: B ∨ A
    let initial_hyps = vec![
        ("h0".to_string(), Expr::Or(Box::new(a.clone()), Box::new(b.clone()))),
        ("h1".to_string(), Expr::Impl(Box::new(a.clone()), Box::new(b_or_a.clone()))),
        ("h2".to_string(), Expr::Impl(Box::new(b.clone()), Box::new(b_or_a.clone()))),
    ];

    let mut state = ProofState::new(initial_hyps, b_or_a.clone());

    // Step 1: OrElim(h0, h1, h2) -> derives h3: B ∨ A
    let step1 = DeductionStep::OrElim {
        hyp_or: "h0".to_string(),
        left_impl: "h1".to_string(),
        right_impl: "h2".to_string(),
    };
    let h3 = state.apply_step(&step1).expect("OrElim failed").expect("Expected new hyp");
    assert_eq!(h3, "h3");
    assert_eq!(state.hyps.get("h3"), Some(&b_or_a));

    // Step 2: Exact(h3) -> closes goal
    let step2 = DeductionStep::Exact { hyp: "h3".to_string() };
    let res = state.apply_step(&step2).expect("Exact failed");
    assert_eq!(res, None);
    assert_eq!(state.status, ProofStatus::Proven);
}

#[test]
fn test_ex_falso_and_disjunctive_syllogism() {
    let target = Expr::Prop("ArbitraryGoal".to_string());

    // h0: False ⊢ ArbitraryGoal
    let mut state = ProofState::new(
        vec![("h0".to_string(), Expr::False)],
        target.clone(),
    );

    // Apply FalseElim(h0) -> closes goal immediately
    let res = state.apply_step(&DeductionStep::FalseElim {
        hyp_false: "h0".to_string(),
    }).expect("FalseElim failed");
    assert_eq!(res, None);
    assert_eq!(state.status, ProofStatus::Proven);
}

#[test]
fn test_or_intro_and_contradiction_lifecycle() {
    let p = Expr::Prop("P".to_string());
    let q = Expr::Prop("Q".to_string());
    let not_p = Expr::Not(Box::new(p.clone()));
    let target = Expr::Or(Box::new(p.clone()), Box::new(q.clone()));

    // h0: P, h1: ¬P
    let mut state = ProofState::new(
        vec![
            ("h0".to_string(), p.clone()),
            ("h1".to_string(), not_p.clone()),
        ],
        target.clone(),
    );

    // Step 1: OrIntroL(h0, Q) -> h2: P ∨ Q
    let h2 = state.apply_step(&DeductionStep::OrIntroL {
        hyp: "h0".to_string(),
        right: q.clone(),
    }).expect("OrIntroL failed").unwrap();
    assert_eq!(h2, "h2");
    assert_eq!(state.hyps.get("h2"), Some(&target));

    // Step 2: Contradiction(h0, h1) -> h3: False
    let h3 = state.apply_step(&DeductionStep::Contradiction {
        pos_hyp: "h0".to_string(),
        neg_hyp: "h1".to_string(),
    }).expect("Contradiction failed").unwrap();
    assert_eq!(h3, "h3");
    assert_eq!(state.hyps.get("h3"), Some(&Expr::False));

    // Step 3: OrIntroR(P, h0) -> h4: P ∨ P
    let h4 = state.apply_step(&DeductionStep::OrIntroR {
        left: p.clone(),
        hyp: "h0".to_string(),
    }).expect("OrIntroR failed").unwrap();
    assert_eq!(h4, "h4");
    assert_eq!(state.hyps.get("h4"), Some(&Expr::Or(Box::new(p.clone()), Box::new(p.clone()))));

    // Step 4: FalseElim(h3) -> closes proof
    let res = state.apply_step(&DeductionStep::FalseElim {
        hyp_false: "h3".to_string(),
    }).expect("FalseElim failed");
    assert_eq!(res, None);
    assert_eq!(state.status, ProofStatus::Proven);
}

#[test]
fn test_or_elim_mismatched_branches_rejected() {
    let a = Expr::Prop("A".to_string());
    let b = Expr::Prop("B".to_string());
    let c = Expr::Prop("C".to_string());
    let d = Expr::Prop("D".to_string());

    let initial_hyps = vec![
        ("h0".to_string(), Expr::Or(Box::new(a.clone()), Box::new(b.clone()))),
        ("h1".to_string(), Expr::Impl(Box::new(a.clone()), Box::new(c.clone()))),
        ("h2".to_string(), Expr::Impl(Box::new(b.clone()), Box::new(d.clone()))), // Concludes D instead of C
    ];

    let mut state = ProofState::new(initial_hyps, c.clone());

    let err = state.apply_step(&DeductionStep::OrElim {
        hyp_or: "h0".to_string(),
        left_impl: "h1".to_string(),
        right_impl: "h2".to_string(),
    });
    assert!(matches!(err, Err(KernelError::TypeMismatch { .. })));
}
