//! Strategy DAG, Strategy Tree, and branched game representations.

use crate::moves::Move;
use crate::polarity::Polarity;
use crate::trace::PlayTrace;
use crate::validator::{verify_all, ArenaValidationError};
use serde::{Deserialize, Serialize};

/// A node in a branched dialogue game strategy tree / DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyNode {
    /// Move played at this strategy node.
    pub current_move: Move,
    /// Branched response continuations.
    pub children: Vec<StrategyNode>,
}

impl StrategyNode {
    /// Create a new leaf strategy node.
    pub fn new(current_move: Move) -> Self {
        Self {
            current_move,
            children: Vec::new(),
        }
    }

    /// Add a child continuation branch.
    pub fn add_child(&mut self, child: StrategyNode) {
        self.children.push(child);
    }

    /// Recursively count total nodes in this subtree.
    pub fn count_nodes(&self) -> usize {
        1 + self.children.iter().map(|c| c.count_nodes()).sum::<usize>()
    }

    /// Recursively collect all root-to-leaf play traces.
    pub fn collect_traces(&self, current_prefix: &mut PlayTrace, traces: &mut Vec<PlayTrace>) {
        let mut extended = current_prefix.clone();
        extended.moves_mut().push(self.current_move.clone());

        if self.children.is_empty() {
            traces.push(extended);
        } else {
            for child in &self.children {
                child.collect_traces(&mut extended, traces);
            }
        }
    }
}

/// A complete branched strategy tree for a dialogue arena game.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyTree {
    /// Root node of the strategy (None if empty).
    pub root: Option<StrategyNode>,
}

impl StrategyTree {
    /// Create an empty strategy tree.
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Create a strategy tree with an initial root move.
    pub fn from_root(root_move: Move) -> Self {
        Self {
            root: Some(StrategyNode::new(root_move)),
        }
    }

    /// Total number of moves / nodes across all strategy branches.
    pub fn node_count(&self) -> usize {
        self.root.as_ref().map_or(0, |r| r.count_nodes())
    }

    /// Extract all linear play traces from root to leaves.
    pub fn extract_traces(&self) -> Vec<PlayTrace> {
        let mut traces = Vec::new();
        if let Some(ref root) = self.root {
            let mut prefix = PlayTrace::new();
            root.collect_traces(&mut prefix, &mut traces);
        }
        traces
    }

    /// Verify that every branch in the strategy tree is a valid, well-bracketed play trace.
    pub fn verify_all_branches(&self) -> Result<(), ArenaValidationError> {
        for trace in self.extract_traces() {
            verify_all(&trace)?;
        }
        Ok(())
    }
}

/// Legacy/Convenience alias for linear dialogue arenas.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaDialogue {
    trace: PlayTrace,
    expected_polarity: Option<Polarity>,
}

impl ArenaDialogue {
    /// Create a new dialogue with initial expected polarity.
    pub fn new(initial_polarity: Polarity) -> Self {
        Self {
            trace: PlayTrace::new(),
            expected_polarity: Some(initial_polarity),
        }
    }

    /// Number of moves in dialogue.
    pub fn len(&self) -> usize {
        self.trace.len()
    }

    /// True if no moves played.
    pub fn is_empty(&self) -> bool {
        self.trace.is_empty()
    }

    /// Moves in the dialogue.
    pub fn moves(&self) -> &[Move] {
        self.trace.moves()
    }

    /// Expected player polarity.
    pub fn expected_polarity(&self) -> Option<Polarity> {
        self.expected_polarity
    }

    /// Play a move.
    pub fn play_move(&mut self, m: Move) -> Result<(), ArenaValidationError> {
        self.trace.push(m.clone())?;
        self.expected_polarity = Some(m.player.dual());
        Ok(())
    }

    /// Inner trace reference.
    pub fn trace(&self) -> &PlayTrace {
        &self.trace
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::LogicalPayload;

    #[test]
    fn test_strategy_tree_branching() {
        let root_mv = Move::root_goal("A ∧ B");
        let mut tree = StrategyTree::from_root(root_mv);

        let root_node = tree.root.as_mut().unwrap();

        // Branch 1: Opponent attacks Left conjunct
        let mut branch1 = StrategyNode::new(Move::question(
            1,
            Polarity::Opponent,
            0,
            LogicalPayload::AttackConjunction {
                branch: crate::moves::ConjunctionBranch::Left,
            },
        ));
        branch1.add_child(StrategyNode::new(Move::answer(
            2,
            Polarity::Proponent,
            1,
            LogicalPayload::AxiomDischarge { premise_id: 0 },
        )));

        // Branch 2: Opponent attacks Right conjunct
        let mut branch2 = StrategyNode::new(Move::question(
            1,
            Polarity::Opponent,
            0,
            LogicalPayload::AttackConjunction {
                branch: crate::moves::ConjunctionBranch::Right,
            },
        ));
        branch2.add_child(StrategyNode::new(Move::answer(
            2,
            Polarity::Proponent,
            1,
            LogicalPayload::AxiomDischarge { premise_id: 1 },
        )));

        root_node.add_child(branch1);
        root_node.add_child(branch2);

        assert_eq!(tree.node_count(), 5);
        let traces = tree.extract_traces();
        assert_eq!(traces.len(), 2);
        assert_eq!(traces[0].len(), 3);
        assert_eq!(traces[1].len(), 3);
    }
}
