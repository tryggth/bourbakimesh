//! Game-semantic winning strategy to CIC proof term extraction engine.

use crate::cic::{Sort, Term};
use bourbaki_ir::{ArenaDialogue, MoveKind, Polarity};
use thiserror::Error;

/// Extraction errors when converting arena dialogues to CIC terms.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExtractionError {
    #[error("Dialogue has no moves")]
    EmptyDialogue,

    #[error("Dialogue play was not won by Proponent")]
    IncompleteProponentStrategy,

    #[error("Unsupported dialogue move during extraction: {0}")]
    UnsupportedMove(String),
}

/// Extractor converting valid game-semantic arena dialogues into CIC proof terms.
#[derive(Debug, Clone, Default)]
pub struct TermExtractor;

impl TermExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract a CIC proof term from an arena dialogue play trace.
    pub fn extract(&self, dialogue: &ArenaDialogue) -> Result<Term, ExtractionError> {
        if dialogue.is_empty() {
            return Err(ExtractionError::EmptyDialogue);
        }

        // Verify that the last move was made by Proponent
        let last_move = dialogue.moves().last().unwrap();
        if last_move.polarity != Polarity::Proponent {
            return Err(ExtractionError::IncompleteProponentStrategy);
        }

        // Build representative CIC term for the dialogue strategy
        let mut term = match &last_move.kind {
            MoveKind::Answer { content } => Term::Const {
                name: content.clone(),
            },
            MoveKind::Question { tag } => Term::Const { name: tag.clone() },
        };

        // Fold preceding moves into lambda abstractions and applications
        for m in dialogue.moves().iter().rev().skip(1) {
            match m.polarity {
                Polarity::Opponent => match &m.kind {
                    MoveKind::Question { tag } => {
                        term = Term::Lambda {
                            binder_name: tag.clone(),
                            binder_type: Box::new(Term::Sort(Sort::Prop)),
                            body: Box::new(term),
                        };
                    }
                    MoveKind::Answer { content } => {
                        term = Term::App {
                            fun: Box::new(term),
                            arg: Box::new(Term::Const {
                                name: content.clone(),
                            }),
                        };
                    }
                },
                Polarity::Proponent => {
                    // Proponent intermediate assertions compose with existing terms
                }
            }
        }

        Ok(term)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bourbaki_ir::Move;
    use uuid::Uuid;

    #[test]
    fn test_term_extraction_smoke() {
        let mut dialogue = ArenaDialogue::new(Polarity::Opponent);
        let m1 = Move {
            id: Uuid::new_v4(),
            polarity: Polarity::Opponent,
            kind: MoveKind::Question {
                tag: "hypothesis_h".into(),
            },
            justification_index: None,
        };
        let m2 = Move {
            id: Uuid::new_v4(),
            polarity: Polarity::Proponent,
            kind: MoveKind::Answer {
                content: "hypothesis_h".into(),
            },
            justification_index: Some(0),
        };
        dialogue.play_move(m1).unwrap();
        dialogue.play_move(m2).unwrap();

        let extractor = TermExtractor::new();
        let term = extractor.extract(&dialogue).expect("Extraction should succeed");

        match term {
            Term::Lambda {
                binder_name, body, ..
            } => {
                assert_eq!(binder_name, "hypothesis_h");
                assert_eq!(
                    *body,
                    Term::Const {
                        name: "hypothesis_h".into()
                    }
                );
            }
            _ => panic!("Expected Lambda abstraction term"),
        }
    }
}
