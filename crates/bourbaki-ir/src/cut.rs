//! Cut elimination and auxiliary lemma injection in game-semantic dialogue arenas.
//!
//! Allows injecting intermediate lemma obligations ($\phi \implies \psi$) as justified
//! dialogue cuts, which branch into:
//! 1. Proving the auxiliary lemma (Child 0).
//! 2. Utilizing the proven lemma to discharge the parent theorem (Child 1).

use crate::arena::{StrategyNode, StrategyTree};
use crate::moves::{LogicalPayload, Move, MoveKind};
use crate::polarity::Polarity;
use serde::{Deserialize, Serialize};

/// Specification of an auxiliary lemma cut injected into an arena dialogue game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaCut {
    /// Identifier index for the lemma (used to bind `lem_{id}`).
    pub lemma_id: usize,
    /// Propositional / type statement of the lemma (e.g. `A -> B` or `Nat.succ x = x + 1`).
    pub statement: String,
    /// Winning strategy proving the auxiliary lemma.
    pub lemma_strategy: StrategyTree,
    /// Continuation winning strategy utilizing the lemma.
    pub continuation_strategy: StrategyTree,
}

impl ArenaCut {
    /// Create a new ArenaCut specification.
    pub fn new(
        lemma_id: usize,
        statement: impl Into<String>,
        lemma_strategy: StrategyTree,
        continuation_strategy: StrategyTree,
    ) -> Self {
        Self {
            lemma_id,
            statement: statement.into(),
            lemma_strategy,
            continuation_strategy,
        }
    }

    /// Convert this cut into a unified branched `StrategyTree`.
    ///
    /// The root node of the resulting tree asserts the cut lemma.
    /// - Branch 0 contains the subtree proving the lemma.
    /// - Branch 1 contains the subtree continuing the main proof.
    pub fn into_strategy_tree(self) -> StrategyTree {
        let cut_move = Move::new(
            0,
            Polarity::Proponent,
            MoveKind::Answer,
            None,
            LogicalPayload::AssertCutLemma {
                lemma_id: self.lemma_id,
                statement: self.statement,
            },
        );

        let mut root_node = StrategyNode::new(cut_move);

        if let Some(lemma_root) = self.lemma_strategy.root {
            root_node.add_child(lemma_root);
        }

        if let Some(cont_root) = self.continuation_strategy.root {
            root_node.add_child(cont_root);
        }

        StrategyTree {
            root: Some(root_node),
        }
    }

    /// Inject an auxiliary lemma cut at the root of an existing continuation strategy.
    pub fn inject_at_root(
        continuation: StrategyTree,
        lemma_id: usize,
        statement: impl Into<String>,
        lemma_strategy: StrategyTree,
    ) -> StrategyTree {
        let cut = Self::new(lemma_id, statement, lemma_strategy, continuation);
        cut.into_strategy_tree()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_cut_creation_and_node_count() {
        // Lemma: proof of True
        let lem_root = StrategyNode::new(Move::new(
            1,
            Polarity::Proponent,
            MoveKind::Answer,
            Some(0),
            LogicalPayload::ProvideWitness {
                term_repr: "True.intro".into(),
            },
        ));
        let lem_strat = StrategyTree {
            root: Some(lem_root),
        };

        // Continuation: proof of P -> P using lemma
        let cont_opp = StrategyNode::new(Move::question(
            1,
            Polarity::Opponent,
            0,
            LogicalPayload::AttackHypothesis { hyp_id: 0 },
        ));
        let cont_prop = StrategyNode::new(Move::answer(
            2,
            Polarity::Proponent,
            1,
            LogicalPayload::AxiomDischarge { premise_id: 0 },
        ));
        let mut cont_root = StrategyNode::new(Move::root_goal("P -> P"));
        let mut cont_opp_node = cont_opp;
        cont_opp_node.add_child(cont_prop);
        cont_root.add_child(cont_opp_node);
        let cont_strat = StrategyTree {
            root: Some(cont_root),
        };

        let cut = ArenaCut::new(0, "True", lem_strat, cont_strat);
        let fused_tree = cut.into_strategy_tree();

        assert!(fused_tree.root.is_some());
        let root = fused_tree.root.as_ref().unwrap();
        assert_eq!(root.children.len(), 2);
        assert_eq!(fused_tree.node_count(), 5); // 1 cut node + 1 lemma node + 3 continuation nodes
    }
}
