//! Integration test suite for Strategy Extraction Compiler and Lean 4 emitter.

use bourbaki_ir::{
    ConjunctionBranch, LogicalPayload, Move, PlayTrace, Polarity, StrategyNode, StrategyTree,
};
use bourbaki_kernel::{MatchCase, StrategyExtractor, Term, ToLean, Universe};

#[test]
fn test_compile_identity_strategy() {
    // A -> A
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

    let term = StrategyExtractor::compile_trace(&trace).expect("Compilation failed");
    let lean_code = term.to_lean_string();

    assert_eq!(lean_code, "fun (hyp_0 : A_0) => hyp_0");
    match term {
        Term::Lam(binder, ty, body) => {
            assert_eq!(binder, "hyp_0");
            assert_eq!(*ty, Term::Var("A_0".into()));
            assert_eq!(*body, Term::Var("hyp_0".into()));
        }
        _ => panic!("Expected Lam"),
    }
}

#[test]
fn test_compile_k_combinator_weakening() {
    // A -> B -> A
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
            LogicalPayload::AttackHypothesis { hyp_id: 1 },
        ))
        .unwrap();
    trace
        .push(Move::answer(
            4,
            Polarity::Proponent,
            3,
            LogicalPayload::AxiomDischarge { premise_id: 0 },
        ))
        .unwrap();

    let term = StrategyExtractor::compile_trace(&trace).expect("Compilation failed");
    let lean_code = term.to_lean_string();

    assert_eq!(lean_code, "fun (hyp_0 : A_0) => fun (hyp_1 : A_1) => hyp_0");
}

#[test]
fn test_compile_modus_ponens() {
    // Modus Ponens term: fun (a : A) => fun (f : A -> B) => f a
    let a_type = Term::var("A");
    let b_type = Term::var("B");
    let f_type = Term::arrow(a_type.clone(), b_type);

    let mp_body = Term::app(Term::var("f"), Term::var("a"));
    let mp_term = Term::lam("a", a_type, Term::lam("f", f_type, mp_body));

    let lean_code = mp_term.to_lean_string();
    assert_eq!(lean_code, "fun (a : A) => fun (f : A -> B) => f a");
}

#[test]
fn test_compile_conjunction_introduction() {
    // A -> B -> A ∧ B
    let root_mv = Move::root_goal("A ∧ B");
    let mut tree = StrategyTree::from_root(root_mv);
    let root_node = tree.root.as_mut().unwrap();

    // Left branch: Opponent attacks left, Proponent discharges hyp_a
    let mut left_branch = StrategyNode::new(Move::question(
        1,
        Polarity::Opponent,
        0,
        LogicalPayload::AttackConjunction {
            branch: ConjunctionBranch::Left,
        },
    ));
    left_branch.add_child(StrategyNode::new(Move::answer(
        2,
        Polarity::Proponent,
        1,
        LogicalPayload::ProvideWitness {
            term_repr: "witness_A".into(),
        },
    )));

    // Right branch: Opponent attacks right, Proponent discharges hyp_b
    let mut right_branch = StrategyNode::new(Move::question(
        1,
        Polarity::Opponent,
        0,
        LogicalPayload::AttackConjunction {
            branch: ConjunctionBranch::Right,
        },
    ));
    right_branch.add_child(StrategyNode::new(Move::answer(
        2,
        Polarity::Proponent,
        1,
        LogicalPayload::ProvideWitness {
            term_repr: "witness_B".into(),
        },
    )));

    root_node.add_child(left_branch);
    root_node.add_child(right_branch);

    let term = StrategyExtractor::compile_strategy(&tree).expect("Compilation failed");
    let lean_code = term.to_lean_string();

    assert_eq!(lean_code, "And.intro witness_A witness_B");
    match term {
        Term::App(fun, arg2) => {
            assert_eq!(*arg2, Term::Var("witness_B".into()));
            match *fun {
                Term::App(c, arg1) => {
                    assert_eq!(*c, Term::Const("And.intro".into(), vec![]));
                    assert_eq!(*arg1, Term::Var("witness_A".into()));
                }
                _ => panic!("Expected nested App"),
            }
        }
        _ => panic!("Expected App"),
    }
}

#[test]
fn test_term_serialization_round_trip() {
    // Construct a rich CIC term with Lam, Let, Match, and Sorts
    let term = Term::lam(
        "x",
        Term::sort(Universe::prop()),
        Term::let_in(
            "y",
            Term::sort(Universe::type_0()),
            Term::var("x"),
            Term::match_term(
                Term::var("y"),
                None,
                vec![
                    MatchCase::new("Option.none", vec![], Term::var("x")),
                    MatchCase::new("Option.some", vec!["val".into()], Term::var("val")),
                ],
            ),
        ),
    );

    // JSON round-trip
    let json = serde_json::to_string(&term).expect("JSON serialization failed");
    let deserialized_json: Term = serde_json::from_str(&json).expect("JSON deserialization failed");
    assert_eq!(term, deserialized_json);

    // Bincode round-trip
    let bincode_bytes = bincode::serialize(&term).expect("Bincode serialization failed");
    let deserialized_bincode: Term =
        bincode::deserialize(&bincode_bytes).expect("Bincode deserialization failed");
    assert_eq!(term, deserialized_bincode);

    // Lean emission check
    let lean_code = term.to_lean_string();
    assert!(lean_code.starts_with("fun (x : Prop) => let y : Type := x; match y with"));
}

#[test]
fn test_compile_cut_lemma_strategy() {
    use bourbaki_ir::cut::ArenaCut;

    // Lemma: proof of True via True.intro
    let lem_root = StrategyNode::new(Move::new(
        1,
        Polarity::Proponent,
        bourbaki_ir::MoveKind::Answer,
        Some(0),
        LogicalPayload::ProvideWitness {
            term_repr: "True.intro".into(),
        },
    ));
    let lem_strat = StrategyTree {
        root: Some(lem_root),
    };

    // Continuation: proof of True using lem_0
    let cont_root = StrategyNode::new(Move::new(
        2,
        Polarity::Proponent,
        bourbaki_ir::MoveKind::Answer,
        Some(0),
        LogicalPayload::ProvideWitness {
            term_repr: "lem_0".into(),
        },
    ));
    let cont_strat = StrategyTree {
        root: Some(cont_root),
    };

    let cut = ArenaCut::new(0, "True", lem_strat, cont_strat);
    let fused_tree = cut.into_strategy_tree();

    let term = StrategyExtractor::compile_strategy(&fused_tree).expect("Cut compilation failed");
    let lean_code = term.to_lean_string();

    assert_eq!(lean_code, "let lem_0 : True := True.intro; lem_0");
    match term {
        Term::Let(name, ty, val, body) => {
            assert_eq!(name, "lem_0");
            assert_eq!(*ty, Term::Var("True".into()));
            assert_eq!(*val, Term::Var("True.intro".into()));
            assert_eq!(*body, Term::Var("lem_0".into()));
        }
        _ => panic!("Expected Let"),
    }
}
