//! Dialogue Move AST, classifications, and logical payloads.

use crate::polarity::Polarity;
use serde::{Deserialize, Serialize};

/// Classification of a dialogue move according to game semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoveKind {
    /// A question challenging a claim or demanding evidence.
    Question,
    /// An answer resolving an open question or asserting a witness.
    Answer,
}

/// Branch selector when attacking a conjunction (A ∧ B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConjunctionBranch {
    /// Demand proof of the left conjunct.
    Left,
    /// Demand proof of the right conjunct.
    Right,
}

/// Logical payload and game action represented by a dialogue move.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogicalPayload {
    /// Opening move declaring the root theorem or proposition to prove.
    RootGoal(String),
    /// Challenge an implication / hypothesis by supplying premise index.
    AttackHypothesis { hyp_id: usize },
    /// Challenge a conjunction by specifying which branch to verify.
    AttackConjunction { branch: ConjunctionBranch },
    /// Challenge an existential quantifier by demanding a constructive witness.
    DemandWitness,
    /// Provide a witness term for an existential or inductive claim.
    ProvideWitness { term_repr: String },
    /// Specialize a universally quantified statement with a term.
    InstantiateUniversal { term_repr: String },
    /// Discharge an atomic hypothesis by citing a matching active hypothesis or premise.
    AxiomDischarge { premise_id: usize },
    /// Attack an inductive elimination by demanding proof for a specific constructor.
    InductiveCaseDemand { constructor_idx: usize },
    /// Assert an intermediate auxiliary cut lemma to be proved and used in subsequent moves.
    AssertCutLemma { lemma_id: usize, statement: String },
}

/// A discrete, justified move in a Hyland-Ong / Lorenzen dialogue arena.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Move {
    /// Sequential 0-indexed position within the play trace.
    pub id: usize,
    /// Player executing the move (Proponent or Opponent).
    pub player: Polarity,
    /// Classification as Question or Answer.
    pub kind: MoveKind,
    /// Index of the enabling move justifying this move (None for root move 0).
    pub justifier: Option<usize>,
    /// Semantic payload and action of the move.
    pub payload: LogicalPayload,
}

impl Move {
    /// Create a new dialogue move.
    pub fn new(
        id: usize,
        player: Polarity,
        kind: MoveKind,
        justifier: Option<usize>,
        payload: LogicalPayload,
    ) -> Self {
        Self {
            id,
            player,
            kind,
            justifier,
            payload,
        }
    }

    /// Construct an initial root goal move for Proponent at step 0.
    pub fn root_goal(goal: impl Into<String>) -> Self {
        Self {
            id: 0,
            player: Polarity::Proponent,
            kind: MoveKind::Question,
            justifier: None,
            payload: LogicalPayload::RootGoal(goal.into()),
        }
    }

    /// Construct a question move.
    pub fn question(
        id: usize,
        player: Polarity,
        justifier: usize,
        payload: LogicalPayload,
    ) -> Self {
        Self {
            id,
            player,
            kind: MoveKind::Question,
            justifier: Some(justifier),
            payload,
        }
    }

    /// Construct an answer move.
    pub fn answer(id: usize, player: Polarity, justifier: usize, payload: LogicalPayload) -> Self {
        Self {
            id,
            player,
            kind: MoveKind::Answer,
            justifier: Some(justifier),
            payload,
        }
    }

    /// Construct a cut lemma assertion move.
    pub fn assert_cut_lemma(
        id: usize,
        player: Polarity,
        justifier: Option<usize>,
        lemma_id: usize,
        statement: impl Into<String>,
    ) -> Self {
        Self {
            id,
            player,
            kind: MoveKind::Answer,
            justifier,
            payload: LogicalPayload::AssertCutLemma {
                lemma_id,
                statement: statement.into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_constructors() {
        let m0 = Move::root_goal("A -> A");
        assert_eq!(m0.id, 0);
        assert_eq!(m0.player, Polarity::Proponent);
        assert_eq!(m0.kind, MoveKind::Question);
        assert_eq!(m0.justifier, None);

        let m1 = Move::question(
            1,
            Polarity::Opponent,
            0,
            LogicalPayload::AttackHypothesis { hyp_id: 0 },
        );
        assert_eq!(m1.id, 1);
        assert_eq!(m1.player, Polarity::Opponent);
        assert_eq!(m1.justifier, Some(0));

        let m2 = Move::answer(
            2,
            Polarity::Proponent,
            1,
            LogicalPayload::AxiomDischarge { premise_id: 0 },
        );
        assert_eq!(m2.id, 2);
        assert_eq!(m2.player, Polarity::Proponent);
        assert_eq!(m2.justifier, Some(1));
    }
}
