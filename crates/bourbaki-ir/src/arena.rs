//! Game-semantic arena definitions and dialogue move sequences.

use crate::polarity::Polarity;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Errors arising from invalid dialogue arena operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArenaError {
    #[error("Out-of-turn move: expected {expected:?}, got {actual:?}")]
    OutOfTurn {
        expected: Polarity,
        actual: Polarity,
    },

    #[error("Invalid justification link: target move index {0} out of bounds")]
    InvalidJustification(usize),

    #[error("Empty dialogue arena has no moves")]
    EmptyArena,
}

/// The kind of dialogue move being performed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoveKind {
    /// Question attacking a prior claim or requesting instantiation.
    Question { tag: String },
    /// Answer defending a prior question or making an assertion.
    Answer { content: String },
}

/// A discrete move played in an arena dialogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Move {
    pub id: Uuid,
    pub polarity: Polarity,
    pub kind: MoveKind,
    /// Optional index of the prior move justifying/attacked by this move.
    pub justification_index: Option<usize>,
}

/// A game-semantic dialogue arena representing a game play trace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaDialogue {
    moves: Vec<Move>,
    expected_polarity: Option<Polarity>,
}

impl ArenaDialogue {
    /// Create a new empty arena dialogue with initial expected polarity.
    pub fn new(initial_polarity: Polarity) -> Self {
        Self {
            moves: Vec::new(),
            expected_polarity: Some(initial_polarity),
        }
    }

    /// Number of moves in the play.
    pub fn len(&self) -> usize {
        self.moves.len()
    }

    /// True if no moves have been played.
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// Read-only access to moves.
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// Current expected polarity.
    pub fn expected_polarity(&self) -> Option<Polarity> {
        self.expected_polarity
    }

    /// Append a move according to Lorenzen/Hyland-Ong alternation rules.
    pub fn play_move(&mut self, m: Move) -> Result<(), ArenaError> {
        if let Some(expected) = self.expected_polarity {
            if m.polarity != expected {
                return Err(ArenaError::OutOfTurn {
                    expected,
                    actual: m.polarity,
                });
            }
        }

        if let Some(target) = m.justification_index {
            if target >= self.moves.len() {
                return Err(ArenaError::InvalidJustification(target));
            }
        }

        self.expected_polarity = Some(m.polarity.dual());
        self.moves.push(m);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialogue_alternation() {
        let mut dialogue = ArenaDialogue::new(Polarity::Opponent);
        assert_eq!(dialogue.expected_polarity(), Some(Polarity::Opponent));

        let m1 = Move {
            id: Uuid::new_v4(),
            polarity: Polarity::Opponent,
            kind: MoveKind::Question {
                tag: "Attack: Left conjunct".into(),
            },
            justification_index: None,
        };
        assert!(dialogue.play_move(m1).is_ok());
        assert_eq!(dialogue.expected_polarity(), Some(Polarity::Proponent));

        // Invalid: same player tries to play again
        let m2_bad = Move {
            id: Uuid::new_v4(),
            polarity: Polarity::Opponent,
            kind: MoveKind::Answer {
                content: "Invalid move".into(),
            },
            justification_index: None,
        };
        assert!(matches!(
            dialogue.play_move(m2_bad),
            Err(ArenaError::OutOfTurn { .. })
        ));

        // Valid: Proponent responds
        let m2_good = Move {
            id: Uuid::new_v4(),
            polarity: Polarity::Proponent,
            kind: MoveKind::Answer {
                content: "Witness A".into(),
            },
            justification_index: Some(0),
        };
        assert!(dialogue.play_move(m2_good).is_ok());
        assert_eq!(dialogue.len(), 2);
    }
}
