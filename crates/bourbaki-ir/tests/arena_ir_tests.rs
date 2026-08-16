//! Integration test suite for Bourbaki IR game semantics and validation engine.

use bourbaki_ir::{
    verify_all, ArenaValidationError, ConjunctionBranch, LogicalPayload, Move, PlayTrace, Polarity,
    StrategyNode, StrategyTree,
};

#[test]
fn test_modus_ponens_dialogue_and_p_views() {
    let mut trace = PlayTrace::new();

    // Step 0: P declares root theorem goal: A -> (B -> A)
    trace
        .push(Move::root_goal("A -> (B -> A)"))
        .expect("Root goal insertion failed");
    assert_eq!(trace.get_p_view(), vec![0]);
    assert_eq!(trace.get_o_view(), vec![0]);

    // Step 1: O attacks hypothesis A
    trace
        .push(Move::question(
            1,
            Polarity::Opponent,
            0,
            LogicalPayload::AttackHypothesis { hyp_id: 0 },
        ))
        .expect("Step 1 failed");
    assert_eq!(trace.get_p_view(), vec![0, 1]);
    assert_eq!(trace.get_o_view(), vec![0, 1]);

    // Step 2: P challenges with sub-question
    trace
        .push(Move::question(
            2,
            Polarity::Proponent,
            1,
            LogicalPayload::AttackHypothesis { hyp_id: 1 },
        ))
        .expect("Step 2 failed");
    assert_eq!(trace.get_p_view(), vec![0, 1, 2]);
    assert_eq!(trace.get_o_view(), vec![0, 1, 2]);

    // Step 3: O attacks question 2
    trace
        .push(Move::question(
            3,
            Polarity::Opponent,
            2,
            LogicalPayload::AttackHypothesis { hyp_id: 1 },
        ))
        .expect("Step 3 failed");
    assert_eq!(trace.get_p_view(), vec![0, 1, 2, 3]);

    // Step 4: P answers question 3
    assert_eq!(trace.active_question_target(Polarity::Proponent), Some(3));
    trace
        .push(Move::answer(
            4,
            Polarity::Proponent,
            3,
            LogicalPayload::ProvideWitness {
                term_repr: "witness_B".into(),
            },
        ))
        .expect("Step 4 failed");

    // Full trace verification
    verify_all(&trace).expect("Modus ponens dialogue must be valid");
    assert_eq!(trace.len(), 5);
}

#[test]
fn test_strict_alternation_violation() {
    let mut trace = PlayTrace::new();

    // Step 0: P root
    trace.push(Move::root_goal("A -> A")).unwrap();

    // Bad Step 1: P attempts to play again out-of-order
    let bad_mv = Move::question(
        1,
        Polarity::Proponent,
        0,
        LogicalPayload::AttackHypothesis { hyp_id: 0 },
    );
    let err = trace.push(bad_mv).unwrap_err();
    assert_eq!(
        err,
        ArenaValidationError::StrictAlternationViolation {
            step: 1,
            expected: Polarity::Opponent,
            found: Polarity::Proponent,
        }
    );
}

#[test]
fn test_forward_and_out_of_view_justification() {
    let mut trace = PlayTrace::new();
    trace.push(Move::root_goal("A -> A")).unwrap();

    // Forward pointer
    let forward_mv = Move::question(
        1,
        Polarity::Opponent,
        5,
        LogicalPayload::AttackHypothesis { hyp_id: 0 },
    );
    let err = trace.push(forward_mv).unwrap_err();
    assert!(matches!(
        err,
        ArenaValidationError::InvalidJustificationPointer { step: 1, .. }
    ));

    // Construct a branch to test out-of-view pointer
    trace
        .push(Move::question(
            1,
            Polarity::Opponent,
            0,
            LogicalPayload::AttackHypothesis { hyp_id: 0 },
        ))
        .unwrap();

    trace
        .push(Move::question(
            2,
            Polarity::Proponent,
            1,
            LogicalPayload::AttackHypothesis { hyp_id: 1 },
        ))
        .unwrap();

    // O branches back to step 0
    trace
        .push(Move::question(
            3,
            Polarity::Opponent,
            0,
            LogicalPayload::AttackConjunction {
                branch: ConjunctionBranch::Left,
            },
        ))
        .unwrap();

    // P view is now [0, 3]. Step 1 and Step 2 are pruned from P view.
    assert_eq!(trace.get_p_view(), vec![0, 3]);

    // P attempts to point to step 1 (which is outside P's view)
    let out_of_view_mv = Move::question(4, Polarity::Proponent, 1, LogicalPayload::DemandWitness);
    let err = trace.push(out_of_view_mv).unwrap_err();
    assert_eq!(
        err,
        ArenaValidationError::JustificationOutsideView {
            step: 4,
            justifier: 1,
        }
    );
}

#[test]
fn test_well_bracketing_violation() {
    let mut trace = PlayTrace::new();
    trace.push(Move::root_goal("A -> B -> A")).unwrap();

    trace
        .push(Move::question(
            1,
            Polarity::Opponent,
            0,
            LogicalPayload::AttackHypothesis { hyp_id: 0 },
        ))
        .unwrap();

    trace
        .push(Move::question(
            2,
            Polarity::Proponent,
            1,
            LogicalPayload::AttackHypothesis { hyp_id: 1 },
        ))
        .unwrap();

    trace
        .push(Move::question(
            3,
            Polarity::Opponent,
            2,
            LogicalPayload::AttackHypothesis { hyp_id: 2 },
        ))
        .unwrap();

    // Active open question in P view is 3. Attempting to answer outer question 1 must fail.
    assert_eq!(trace.active_question_target(Polarity::Proponent), Some(3));

    let bad_answer = Move::answer(
        4,
        Polarity::Proponent,
        1,
        LogicalPayload::AxiomDischarge { premise_id: 0 },
    );
    let err = trace.push(bad_answer).unwrap_err();
    assert_eq!(
        err,
        ArenaValidationError::WellBracketingViolation {
            step: 4,
            attempted_target: 1,
            expected_target: 3,
        }
    );
}

#[test]
fn test_serialization_round_trip() {
    let mut trace = PlayTrace::new();
    trace.push(Move::root_goal("A -> A")).unwrap();
    trace
        .push(Move::question(
            1,
            Polarity::Opponent,
            0,
            LogicalPayload::AttackHypothesis { hyp_id: 0 },
        ))
        .unwrap();
    trace
        .push(Move::answer(
            2,
            Polarity::Proponent,
            1,
            LogicalPayload::AxiomDischarge { premise_id: 0 },
        ))
        .unwrap();

    // JSON Round-trip
    let json_data = serde_json::to_string(&trace).expect("JSON serialization failed");
    let deserialized_json: PlayTrace =
        serde_json::from_str(&json_data).expect("JSON deserialization failed");
    assert_eq!(trace, deserialized_json);

    // Bincode Round-trip
    let bincode_data = bincode::serialize(&trace).expect("Bincode serialization failed");
    let deserialized_bincode: PlayTrace =
        bincode::deserialize(&bincode_data).expect("Bincode deserialization failed");
    assert_eq!(trace, deserialized_bincode);

    // Strategy Tree Round-trip
    let mut tree = StrategyTree::from_root(Move::root_goal("A -> A"));
    let mut n1 = StrategyNode::new(Move::question(
        1,
        Polarity::Opponent,
        0,
        LogicalPayload::AttackHypothesis { hyp_id: 0 },
    ));
    n1.add_child(StrategyNode::new(Move::answer(
        2,
        Polarity::Proponent,
        1,
        LogicalPayload::AxiomDischarge { premise_id: 0 },
    )));
    tree.root.as_mut().unwrap().add_child(n1);

    let tree_json = serde_json::to_string(&tree).expect("Tree JSON serialization failed");
    let deserialized_tree: StrategyTree =
        serde_json::from_str(&tree_json).expect("Tree JSON deserialization failed");
    assert_eq!(tree, deserialized_tree);
    assert_eq!(deserialized_tree.node_count(), 3);
}
