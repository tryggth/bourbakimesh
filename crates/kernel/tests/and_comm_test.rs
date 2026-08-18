use std::time::Instant;
use kernel::ast::{DeductionStep, Expr, ProofStatus};
use kernel::state::ProofState;

#[test]
fn test_and_commutativity_proof_lifecycle() {
    // Initial State: h0: And(A, B) |- And(B, A)
    let a = Expr::Prop("A".to_string());
    let b = Expr::Prop("B".to_string());
    let initial_hyp = Expr::And(Box::new(a.clone()), Box::new(b.clone()));
    let target = Expr::And(Box::new(b.clone()), Box::new(a.clone()));

    let mut state = ProofState::new(vec![("h0".to_string(), initial_hyp)], target);
    assert_eq!(state.status, ProofStatus::Open);

    let start = Instant::now();

    // Step 1: AndElimR(h0) -> h1: B
    let h1 = state.apply_step(&DeductionStep::AndElimR { hyp: "h0".to_string() }).unwrap().unwrap();
    assert_eq!(h1, "h1");
    assert_eq!(state.hyps.get("h1"), Some(&b));

    // Step 2: AndElimL(h0) -> h2: A
    let h2 = state.apply_step(&DeductionStep::AndElimL { hyp: "h0".to_string() }).unwrap().unwrap();
    assert_eq!(h2, "h2");
    assert_eq!(state.hyps.get("h2"), Some(&a));

    // Step 3: AndIntro(h1, h2) -> h3: And(B, A)
    let h3 = state.apply_step(&DeductionStep::AndIntro { left: "h1".to_string(), right: "h2".to_string() }).unwrap().unwrap();
    assert_eq!(h3, "h3");
    assert_eq!(state.hyps.get("h3"), Some(&Expr::And(Box::new(b), Box::new(a))));

    // Step 4: Exact(h3) -> Proof Closed
    let close = state.apply_step(&DeductionStep::Exact { hyp: "h3".to_string() }).unwrap();
    assert_eq!(close, None);
    assert_eq!(state.status, ProofStatus::Proven);

    let elapsed = start.elapsed();
    println!("⏱️ Total AndComm 4-step proof time: {:?}", elapsed);
    assert!(elapsed.as_micros() < 50, "Kernel step evaluation exceeded latency ceiling");
}

#[test]
fn test_json_ast_roundtrip_and_rejection() {
    let raw_json = r#"{"rule":"AndIntro","left":"h1","right":"h2"}"#;
    let parsed: DeductionStep = serde_json::from_str(raw_json).unwrap();
    assert_eq!(parsed, DeductionStep::AndIntro { left: "h1".to_string(), right: "h2".to_string() });

    // Invalid JSON schema check
    let bad_json = r#"{"rule":"MagicClose","target":"all"}"#;
    let err = serde_json::from_str::<DeductionStep>(bad_json);
    assert!(err.is_err());
}
