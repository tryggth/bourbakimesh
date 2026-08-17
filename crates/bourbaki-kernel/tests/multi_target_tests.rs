//! Integration tests for Universal Multi-Target Proof Extraction (Lean 4, Coq, Isabelle/HOL, Dedukti).

use bourbaki_ir::{ConjunctionBranch, LogicalPayload, Move, Polarity, StrategyNode, StrategyTree};
use bourbaki_kernel::ast::Term;
use bourbaki_kernel::emitter::ProofEmitter;
use bourbaki_kernel::emitters::{CoqEmitter, DeduktiEmitter, IsabelleEmitter, LeanEmitter};

/// Helper to verify balanced parentheses, brackets, and braces in emitted code.
fn assert_balanced_delimiters(code: &str) {
    let mut parens = 0isize;
    let mut brackets = 0isize;
    let mut braces = 0isize;

    for ch in code.chars() {
        match ch {
            '(' => parens += 1,
            ')' => {
                parens -= 1;
                assert!(parens >= 0, "Unmatched closing parenthesis in: {}", code);
            }
            '[' => brackets += 1,
            ']' => {
                brackets -= 1;
                assert!(brackets >= 0, "Unmatched closing bracket in: {}", code);
            }
            '{' => braces += 1,
            '}' => {
                braces -= 1;
                assert!(braces >= 0, "Unmatched closing brace in: {}", code);
            }
            _ => {}
        }
    }

    assert_eq!(parens, 0, "Unclosed parenthesis in: {}", code);
    assert_eq!(brackets, 0, "Unclosed bracket in: {}", code);
    assert_eq!(braces, 0, "Unclosed brace in: {}", code);
}

#[test]
fn test_all_emitters_target_metadata() {
    let lean = LeanEmitter::new();
    let coq = CoqEmitter::new();
    let isabelle = IsabelleEmitter::new();
    let dedukti = DeduktiEmitter::new();

    assert_eq!(lean.target_name(), "Lean 4");
    assert_eq!(lean.file_extension(), "lean");

    assert_eq!(coq.target_name(), "Coq");
    assert_eq!(coq.file_extension(), "v");

    assert_eq!(isabelle.target_name(), "Isabelle/HOL");
    assert_eq!(isabelle.file_extension(), "thy");

    assert_eq!(dedukti.target_name(), "Dedukti");
    assert_eq!(dedukti.file_extension(), "dk");
}

#[test]
fn test_multi_target_identity_emission() {
    // Proposition: P -> P
    let ty = Term::arrow(Term::var("P"), Term::var("P"));
    // Proof term: fun (p : P) => p
    let proof = Term::lam("p", Term::var("P"), Term::var("p"));

    let emitters: Vec<Box<dyn ProofEmitter>> = vec![
        Box::new(LeanEmitter::new()),
        Box::new(CoqEmitter::new()),
        Box::new(IsabelleEmitter::new()),
        Box::new(DeduktiEmitter::new()),
    ];

    for emitter in emitters {
        let thm = emitter.emit_theorem("id_proof", &ty, &proof).unwrap();
        assert_balanced_delimiters(&thm);

        match emitter.target_name() {
            "Lean 4" => {
                assert!(thm.contains("theorem id_proof : P -> P :="));
                assert!(thm.contains("fun (p : P) => p"));
            }
            "Coq" => {
                assert!(thm.contains("Theorem id_proof : P -> P."));
                assert!(thm.contains("Proof."));
                assert!(thm.contains("exact (fun (p : P) => p)."));
                assert!(thm.contains("Qed."));
            }
            "Isabelle/HOL" => {
                assert!(thm.contains("lemma id_proof:"));
                assert!(thm.contains(r"P \<longrightarrow> P"));
                assert!(thm.contains("proof -"));
                assert!(thm.contains("qed"));
            }
            "Dedukti" => {
                assert_eq!(thm, "def id_proof : P -> P := (p : P => p).");
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn test_multi_target_modus_ponens_emission() {
    // Proposition: P -> (P -> Q) -> Q
    let ty = Term::arrow(
        Term::var("P"),
        Term::arrow(Term::arrow(Term::var("P"), Term::var("Q")), Term::var("Q")),
    );
    // Proof: fun (p : P) => fun (f : P -> Q) => f p
    let proof = Term::lam(
        "p",
        Term::var("P"),
        Term::lam(
            "f",
            Term::arrow(Term::var("P"), Term::var("Q")),
            Term::app(Term::var("f"), Term::var("p")),
        ),
    );

    let emitters: Vec<Box<dyn ProofEmitter>> = vec![
        Box::new(LeanEmitter::new()),
        Box::new(CoqEmitter::new()),
        Box::new(IsabelleEmitter::new()),
        Box::new(DeduktiEmitter::new()),
    ];

    for emitter in emitters {
        let thm = emitter.emit_theorem("modus_ponens", &ty, &proof).unwrap();
        assert_balanced_delimiters(&thm);
        assert!(thm.contains("modus_ponens"));
    }
}

#[test]
fn test_multi_target_transitivity_emission() {
    // Proposition: (P -> Q) -> (Q -> R) -> (P -> R)
    let ty = Term::arrow(
        Term::arrow(Term::var("P"), Term::var("Q")),
        Term::arrow(
            Term::arrow(Term::var("Q"), Term::var("R")),
            Term::arrow(Term::var("P"), Term::var("R")),
        ),
    );
    // Proof: fun (f : P -> Q) => fun (g : Q -> R) => fun (p : P) => g (f p)
    let proof = Term::lam(
        "f",
        Term::arrow(Term::var("P"), Term::var("Q")),
        Term::lam(
            "g",
            Term::arrow(Term::var("Q"), Term::var("R")),
            Term::lam(
                "p",
                Term::var("P"),
                Term::app(Term::var("g"), Term::app(Term::var("f"), Term::var("p"))),
            ),
        ),
    );

    let emitters: Vec<Box<dyn ProofEmitter>> = vec![
        Box::new(LeanEmitter::new()),
        Box::new(CoqEmitter::new()),
        Box::new(IsabelleEmitter::new()),
        Box::new(DeduktiEmitter::new()),
    ];

    for emitter in emitters {
        let thm = emitter
            .emit_theorem("implication_trans", &ty, &proof)
            .unwrap();
        assert_balanced_delimiters(&thm);
        assert!(thm.contains("implication_trans"));
    }
}

#[test]
fn test_multi_target_let_cut_lemma_emission() {
    // Proposition: P -> P
    let ty = Term::arrow(Term::var("P"), Term::var("P"));
    // Proof: fun (p : P) => let lemma_1 : P := p; lemma_1
    let proof = Term::lam(
        "p",
        Term::var("P"),
        Term::let_in(
            "lemma_1",
            Term::var("P"),
            Term::var("p"),
            Term::var("lemma_1"),
        ),
    );

    let emitters: Vec<Box<dyn ProofEmitter>> = vec![
        Box::new(LeanEmitter::new()),
        Box::new(CoqEmitter::new()),
        Box::new(IsabelleEmitter::new()),
        Box::new(DeduktiEmitter::new()),
    ];

    for emitter in emitters {
        let thm = emitter.emit_theorem("cut_lemma_test", &ty, &proof).unwrap();
        assert_balanced_delimiters(&thm);

        match emitter.target_name() {
            "Lean 4" => {
                assert!(thm.contains("let lemma_1 : P := p; lemma_1"));
            }
            "Coq" => {
                assert!(thm.contains("let lemma_1 : P := p in lemma_1"));
            }
            "Isabelle/HOL" => {
                assert!(thm.contains("let lemma_1 = p in lemma_1"));
            }
            "Dedukti" => {
                assert!(thm.contains("let lemma_1 : P := p in lemma_1"));
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn test_multi_target_conjunction_strategy_extraction_and_emission() {
    // A -> B -> A ∧ B
    let root_mv = Move::root_goal("A ∧ B");
    let mut tree = StrategyTree::from_root(root_mv);
    let root_node = tree.root.as_mut().unwrap();

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

    let emitters: Vec<Box<dyn ProofEmitter>> = vec![
        Box::new(LeanEmitter::new()),
        Box::new(CoqEmitter::new()),
        Box::new(IsabelleEmitter::new()),
        Box::new(DeduktiEmitter::new()),
    ];

    for emitter in emitters {
        let code = emitter.emit_strategy(&tree).unwrap();
        assert_balanced_delimiters(&code);
        assert!(code.contains("witness_A"));
        assert!(code.contains("witness_B"));
    }
}
