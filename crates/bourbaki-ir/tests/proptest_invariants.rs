//! Tier 3a Property-Based Invariant Fuzzing for Bourbaki IR.
//!
//! Fuzzes game-semantic dialogue arena properties: strict alternation,
//! Hyland-Ong view calculations (P-view / O-view), stack-disciplined well-bracketing,
//! pointer bounds, and serialization invariance across millions of arbitrary inputs.

use bourbaki_ir::{
    verify_all, ArenaValidationError, ConjunctionBranch, LogicalPayload, Move, MoveKind, PlayTrace,
    Polarity,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Proptest Strategies for Generating Dialogue AST
// ---------------------------------------------------------------------------

fn arb_polarity() -> impl Strategy<Value = Polarity> {
    prop_oneof![Just(Polarity::Proponent), Just(Polarity::Opponent)]
}

fn arb_move_kind() -> impl Strategy<Value = MoveKind> {
    prop_oneof![Just(MoveKind::Question), Just(MoveKind::Answer)]
}

fn arb_conjunction_branch() -> impl Strategy<Value = ConjunctionBranch> {
    prop_oneof![
        Just(ConjunctionBranch::Left),
        Just(ConjunctionBranch::Right)
    ]
}

fn arb_logical_payload() -> impl Strategy<Value = LogicalPayload> {
    prop_oneof![
        "[a-zA-Z0-9_ ->()]{1,20}".prop_map(LogicalPayload::RootGoal),
        (0usize..100).prop_map(|hyp_id| LogicalPayload::AttackHypothesis { hyp_id }),
        arb_conjunction_branch().prop_map(|branch| LogicalPayload::AttackConjunction { branch }),
        Just(LogicalPayload::DemandWitness),
        "[a-zA-Z0-9_]{1,10}".prop_map(|term_repr| LogicalPayload::ProvideWitness { term_repr }),
        "[a-zA-Z0-9_]{1,10}"
            .prop_map(|term_repr| LogicalPayload::InstantiateUniversal { term_repr }),
        (0usize..100).prop_map(|premise_id| LogicalPayload::AxiomDischarge { premise_id }),
        (0usize..10)
            .prop_map(|constructor_idx| LogicalPayload::InductiveCaseDemand { constructor_idx }),
        (0usize..50, "[a-zA-Z0-9_ ->]{1,15}").prop_map(|(lemma_id, statement)| {
            LogicalPayload::AssertCutLemma {
                lemma_id,
                statement,
            }
        }),
    ]
}

/// Strategy for generating arbitrary (unvalidated) raw moves.
fn arb_raw_move(max_step: usize) -> impl Strategy<Value = Move> {
    (
        0..=max_step,
        arb_polarity(),
        arb_move_kind(),
        proptest::option::of(0..=max_step),
        arb_logical_payload(),
    )
        .prop_map(|(id, player, kind, justifier, payload)| Move {
            id,
            player,
            kind,
            justifier,
            payload,
        })
}

/// Strategy generating arbitrary raw move vectors.
fn arb_raw_trace(max_len: usize) -> impl Strategy<Value = Vec<Move>> {
    (0..=max_len).prop_flat_map(|len| {
        let strategies: Vec<_> = (0..len).map(arb_raw_move).collect();
        strategies
    })
}

/// Strategy generating strictly valid generative dialogue play traces.
fn arb_valid_trace(max_depth: usize) -> impl Strategy<Value = PlayTrace> {
    proptest::collection::vec((any::<bool>(), 0..10usize), 1..=max_depth).prop_map(|actions| {
        let mut trace = PlayTrace::new();
        // Step 0: Proponent root goal
        trace.push(Move::root_goal("Goal")).unwrap();

        let mut open_questions: Vec<usize> = vec![0];
        let mut step = 1;
        let mut current_player = Polarity::Opponent;

        for (ask_question, param) in actions {
            let player_view = match current_player {
                Polarity::Proponent => trace.get_p_view(),
                Polarity::Opponent => trace.get_o_view(),
            };

            let active_q = trace.active_question_target(current_player);

            if !ask_question && active_q.is_some() {
                // Play Answer move
                let target = active_q.unwrap();
                let answer_move = Move::answer(
                    step,
                    current_player,
                    target,
                    LogicalPayload::AxiomDischarge { premise_id: param },
                );
                if trace.push(answer_move).is_ok() {
                    if let Some(pos) = open_questions.iter().rposition(|&q| q == target) {
                        open_questions.remove(pos);
                    }
                    step += 1;
                    current_player = current_player.dual();
                }
            } else if !player_view.is_empty() {
                // Play Question move pointing to an enabled move in view
                let valid_targets: Vec<usize> = player_view
                    .into_iter()
                    .filter(|&idx| trace.moves()[idx].player == current_player.dual())
                    .collect();

                if !valid_targets.is_empty() {
                    let justifier = valid_targets[param % valid_targets.len()];
                    let question_move = Move::question(
                        step,
                        current_player,
                        justifier,
                        LogicalPayload::AttackHypothesis { hyp_id: param },
                    );
                    if trace.push(question_move).is_ok() {
                        open_questions.push(step);
                        step += 1;
                        current_player = current_player.dual();
                    }
                }
            }
        }
        trace
    })
}

// ---------------------------------------------------------------------------
// Property Tests
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// Property 1: Generatively constructed valid dialogue plays always pass `verify_all`.
    #[test]
    fn prop_valid_traces_pass_all_checks(trace in arb_valid_trace(15)) {
        let is_ok = verify_all(&trace).is_ok();
        prop_assert!(is_ok, "Generated valid trace must pass verify_all");
    }

    /// Property 2: Alternation Invariant - Traces with duplicate adjacent polarities are rejected.
    #[test]
    fn prop_consecutive_same_player_rejected(
        mut trace in arb_valid_trace(10),
    ) {
        if !trace.is_empty() {
            let step = trace.len();
            let last_player = trace.moves().last().unwrap().player;
            // Force same player on next move
            let bad_move = Move {
                id: step,
                player: last_player,
                kind: MoveKind::Question,
                justifier: Some(0),
                payload: LogicalPayload::DemandWitness,
            };
            let err = trace.push(bad_move);
            let is_alternation_err = matches!(
                err,
                Err(ArenaValidationError::StrictAlternationViolation { .. })
            );
            prop_assert!(is_alternation_err, "Consecutive same-player move must fail with StrictAlternationViolation");
        }
    }

    /// Property 3: Pointer Bounds Invariant - Forward pointers are always rejected.
    #[test]
    fn prop_forward_pointers_rejected(
        mut trace in arb_valid_trace(10),
        offset in 1..20usize
    ) {
        if !trace.is_empty() {
            let step = trace.len();
            let expected_player = trace.moves().last().unwrap().player.dual();
            let bad_move = Move {
                id: step,
                player: expected_player,
                kind: MoveKind::Question,
                justifier: Some(step + offset), // Forward pointer
                payload: LogicalPayload::DemandWitness,
            };
            let err = trace.push(bad_move);
            let is_invalid_ptr_err = matches!(
                err,
                Err(ArenaValidationError::InvalidJustificationPointer { .. })
            );
            prop_assert!(is_invalid_ptr_err, "Forward pointer must fail with InvalidJustificationPointer");
        }
    }

    /// Property 4: View Slicing Invariance - Views are strictly monotonic, bounded, and never panic.
    #[test]
    fn prop_view_indices_valid(raw_moves in arb_raw_trace(20)) {
        let mut trace = PlayTrace::new();
        *trace.moves_mut() = raw_moves;

        let p_view = trace.get_p_view();
        let o_view = trace.get_o_view();

        // 1. P-view bounds and monotonicity
        for window in p_view.windows(2) {
            prop_assert!(window[0] < window[1], "P-view must be strictly monotonically increasing");
        }
        for &idx in &p_view {
            prop_assert!(idx < trace.len(), "P-view indices must be within trace bounds");
        }

        // 2. O-view bounds and monotonicity
        for window in o_view.windows(2) {
            prop_assert!(window[0] < window[1], "O-view must be strictly monotonically increasing");
        }
        for &idx in &o_view {
            prop_assert!(idx < trace.len(), "O-view indices must be within trace bounds");
        }
    }

    /// Property 5: Serialization Invariance - Traces round-trip losslessly through bincode and JSON.
    #[test]
    fn prop_bincode_and_json_serde_roundtrip(trace in arb_valid_trace(12)) {
        // Bincode round-trip
        let bincode_bytes = bincode::serialize(&trace).expect("Bincode serialization must succeed");
        let decoded_bincode: PlayTrace =
            bincode::deserialize(&bincode_bytes).expect("Bincode deserialization must succeed");
        prop_assert_eq!(&trace, &decoded_bincode);

        // JSON round-trip
        let json_str = serde_json::to_string(&trace).expect("JSON serialization must succeed");
        let decoded_json: PlayTrace =
            serde_json::from_str(&json_str).expect("JSON deserialization must succeed");
        prop_assert_eq!(&trace, &decoded_json);
    }
}
