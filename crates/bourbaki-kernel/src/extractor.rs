//! Game-semantic winning strategy to CIC proof term extraction engine.

use crate::cic::{Sort, Term};
use bourbaki_ir::{ArenaDialogue, LogicalPayload, MoveKind, PlayTrace, Polarity};
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

    /// Extract a CIC proof term from a play trace.
    pub fn extract_trace(&self, trace: &PlayTrace) -> Result<Term, ExtractionError> {
        if trace.is_empty() {
            return Err(ExtractionError::EmptyDialogue);
        }

        // Verify that the last move was made by Proponent
        let last_move = trace.moves().last().unwrap();
        if last_move.player != Polarity::Proponent {
            return Err(ExtractionError::IncompleteProponentStrategy);
        }

        // Build representative CIC term for the dialogue strategy
        let mut term = match &last_move.payload {
            LogicalPayload::ProvideWitness { term_repr } => Term::Const {
                name: term_repr.clone(),
            },
            LogicalPayload::AxiomDischarge { premise_id } => Term::Const {
                name: format!("hyp_{}", premise_id),
            },
            LogicalPayload::RootGoal(goal) => Term::Const { name: goal.clone() },
            other => Term::Const {
                name: format!("{:?}", other),
            },
        };

        // Fold preceding moves into lambda abstractions and applications
        for m in trace.moves().iter().rev().skip(1) {
            match m.player {
                Polarity::Opponent => match &m.payload {
                    LogicalPayload::AttackHypothesis { hyp_id } => {
                        term = Term::Lambda {
                            binder_name: format!("hyp_{}", hyp_id),
                            binder_type: Box::new(Term::Sort(Sort::Prop)),
                            body: Box::new(term),
                        };
                    }
                    LogicalPayload::InstantiateUniversal { term_repr } => {
                        term = Term::App {
                            fun: Box::new(term),
                            arg: Box::new(Term::Const {
                                name: term_repr.clone(),
                            }),
                        };
                    }
                    _ => {
                        if m.kind == MoveKind::Question {
                            term = Term::Lambda {
                                binder_name: format!("arg_{}", m.id),
                                binder_type: Box::new(Term::Sort(Sort::Prop)),
                                body: Box::new(term),
                            };
                        }
                    }
                },
                Polarity::Proponent => {
                    // Proponent intermediate assertions compose with existing terms
                }
            }
        }

        Ok(term)
    }

    /// Extract a CIC proof term from an arena dialogue play trace.
    pub fn extract(&self, dialogue: &ArenaDialogue) -> Result<Term, ExtractionError> {
        self.extract_trace(dialogue.trace())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bourbaki_ir::Move;

    #[test]
    fn test_term_extraction_smoke() {
        let mut dialogue = ArenaDialogue::new(Polarity::Proponent);
        let m0 = Move::root_goal("A -> A");
        let m1 = Move::question(
            1,
            Polarity::Opponent,
            0,
            LogicalPayload::AttackHypothesis { hyp_id: 0 },
        );
        let m2 = Move::answer(
            2,
            Polarity::Proponent,
            1,
            LogicalPayload::AxiomDischarge { premise_id: 0 },
        );
        dialogue.play_move(m0).unwrap();
        dialogue.play_move(m1).unwrap();
        dialogue.play_move(m2).unwrap();

        let extractor = TermExtractor::new();
        let term = extractor
            .extract(&dialogue)
            .expect("Extraction should succeed");

        match term {
            Term::Lambda {
                binder_name, body, ..
            } => {
                assert_eq!(binder_name, "hyp_0");
                assert_eq!(
                    *body,
                    Term::Const {
                        name: "hyp_0".into()
                    }
                );
            }
            _ => panic!("Expected Lambda abstraction term"),
        }
    }
}
