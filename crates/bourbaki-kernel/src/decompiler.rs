//! Term-to-Strategy Decompiler ($\text{Term}_{\text{CIC}} \to \text{StrategyTree}$).

use crate::ast::Term;
use crate::emitter::ToLean;
use bourbaki_ir::{ConjunctionBranch, LogicalPayload, Move, Polarity, StrategyNode, StrategyTree};
use thiserror::Error;

/// Errors arising during term decompilation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecompileError {
    #[error("Unsupported term structure for decompilation: {0}")]
    UnsupportedTerm(String),

    #[error("Empty term")]
    EmptyTerm,

    #[error("Invalid decompilation structure: {0}")]
    InvalidStructure(String),
}

/// Decompiler converting CIC proof terms back into game-semantic arena strategy trees.
pub struct CICDecompiler;

impl CICDecompiler {
    /// Decompile a CIC proposition type and proof term into an arena StrategyTree.
    pub fn term_to_strategy(
        _name: &str,
        prop_type: &Term,
        proof_term: &Term,
    ) -> Result<StrategyTree, DecompileError> {
        let root_goal_str = prop_type.to_lean_string();
        let root_move = Move::root_goal(&root_goal_str);
        let mut tree = StrategyTree::from_root(root_move);

        let mut current_step = 1;
        let mut binders = Vec::new();
        let mut curr = proof_term;

        // 1. Unfold outer lambda abstractions into Opponent attack listeners
        while let Term::Lam(name, ty, body) = curr {
            binders.push((name.clone(), ty.to_lean_string()));
            curr = body;
        }

        let root_node = tree.root.as_mut().unwrap();
        let mut parent_node = root_node;

        for (name, ty_str) in binders {
            let opp_move = Move::question(
                current_step,
                Polarity::Opponent,
                if current_step == 1 {
                    0
                } else {
                    current_step - 1
                },
                LogicalPayload::InstantiateUniversal {
                    term_repr: format!("{} : {}", name, ty_str),
                },
            );
            current_step += 1;
            let opp_node = StrategyNode::new(opp_move);
            parent_node.add_child(opp_node);
            parent_node = parent_node.children.last_mut().unwrap();
        }

        // 2. Decompile the inner proof body
        match curr {
            Term::Var(name) => {
                let answer_move = Move::answer(
                    current_step,
                    Polarity::Proponent,
                    current_step - 1,
                    LogicalPayload::ProvideWitness {
                        term_repr: name.clone(),
                    },
                );
                parent_node.add_child(StrategyNode::new(answer_move));
            }
            Term::App(fun, arg) => {
                if let Term::Const(c_name, _) = &**fun {
                    if c_name == "And.intro" {
                        let mut left_node = StrategyNode::new(Move::question(
                            current_step,
                            Polarity::Opponent,
                            current_step - 1,
                            LogicalPayload::AttackConjunction {
                                branch: ConjunctionBranch::Left,
                            },
                        ));
                        left_node.add_child(StrategyNode::new(Move::answer(
                            current_step + 1,
                            Polarity::Proponent,
                            current_step,
                            LogicalPayload::ProvideWitness {
                                term_repr: arg.to_lean_string(),
                            },
                        )));
                        parent_node.add_child(left_node);
                        return Ok(tree);
                    }
                }

                let answer_move = Move::answer(
                    current_step,
                    Polarity::Proponent,
                    current_step - 1,
                    LogicalPayload::ProvideWitness {
                        term_repr: format!("{} {}", fun.to_lean_string(), arg.to_lean_string()),
                    },
                );
                parent_node.add_child(StrategyNode::new(answer_move));
            }
            other => {
                let answer_move = Move::answer(
                    current_step,
                    Polarity::Proponent,
                    current_step - 1,
                    LogicalPayload::ProvideWitness {
                        term_repr: other.to_lean_string(),
                    },
                );
                parent_node.add_child(StrategyNode::new(answer_move));
            }
        }

        Ok(tree)
    }
}
