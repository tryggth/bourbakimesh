//! Dialogue game invariant validation engine.

use crate::moves::{Move, MoveKind};
use crate::polarity::Polarity;
use crate::trace::PlayTrace;
use thiserror::Error;

/// Validation errors for game-semantic dialogue arena invariants.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ArenaValidationError {
    #[error("Step {step}: alternation violation. Expected {expected:?}, found {found:?}")]
    StrictAlternationViolation {
        step: usize,
        expected: Polarity,
        found: Polarity,
    },

    #[error("Step {step}: non-initial move missing justification pointer")]
    MissingJustificationPointer { step: usize },

    #[error("Step {step}: invalid pointer to {justifier} ({reason})")]
    InvalidJustificationPointer {
        step: usize,
        justifier: usize,
        reason: String,
    },

    #[error(
        "Step {step}: well-bracketing violation. Answered {attempted_target}, but active open question is {expected_target}"
    )]
    WellBracketingViolation {
        step: usize,
        attempted_target: usize,
        expected_target: usize,
    },

    #[error("Step {step}: justification pointer {justifier} is outside the player view")]
    JustificationOutsideView { step: usize, justifier: usize },
}

/// Standalone invariant validator for an entire PlayTrace.
pub fn verify_all(trace: &PlayTrace) -> Result<(), ArenaValidationError> {
    if trace.is_empty() {
        return Ok(());
    }

    let mut incremental = PlayTrace::new();
    for mv in trace.moves() {
        verify_next_move(&incremental, mv)?;
        incremental.moves_mut().push(mv.clone());
    }
    Ok(())
}

/// Verify that `mv` is a valid continuation of `trace`.
pub fn verify_next_move(trace: &PlayTrace, mv: &Move) -> Result<(), ArenaValidationError> {
    let step = trace.len();

    // 1. Check ID sequence
    if mv.id != step {
        return Err(ArenaValidationError::InvalidJustificationPointer {
            step,
            justifier: mv.id,
            reason: format!(
                "move id {} does not match current step index {}",
                mv.id, step
            ),
        });
    }

    // 2. Check Strict Alternation
    if step == 0 {
        if mv.player != Polarity::Proponent {
            return Err(ArenaValidationError::StrictAlternationViolation {
                step: 0,
                expected: Polarity::Proponent,
                found: mv.player,
            });
        }
    } else {
        let prev_player = trace.moves()[step - 1].player;
        let expected = prev_player.dual();
        if mv.player != expected {
            return Err(ArenaValidationError::StrictAlternationViolation {
                step,
                expected,
                found: mv.player,
            });
        }
    }

    // 3. Check Justification Pointers
    if step == 0 {
        if let Some(justifier) = mv.justifier {
            return Err(ArenaValidationError::InvalidJustificationPointer {
                step: 0,
                justifier,
                reason: "initial root move cannot have a justifier".into(),
            });
        }
    } else {
        let justifier = mv
            .justifier
            .ok_or(ArenaValidationError::MissingJustificationPointer { step })?;

        if justifier >= step {
            return Err(ArenaValidationError::InvalidJustificationPointer {
                step,
                justifier,
                reason: "justifier index must be strictly less than current step".into(),
            });
        }

        let justified_move = &trace.moves()[justifier];
        if justified_move.player == mv.player {
            return Err(ArenaValidationError::InvalidJustificationPointer {
                step,
                justifier,
                reason: format!(
                    "justifier player {:?} must have opposite polarity to move player {:?}",
                    justified_move.player, mv.player
                ),
            });
        }

        // Check View Inclusion
        let player_view = match mv.player {
            Polarity::Proponent => trace.get_p_view(),
            Polarity::Opponent => trace.get_o_view(),
        };

        if !player_view.contains(&justifier) {
            return Err(ArenaValidationError::JustificationOutsideView { step, justifier });
        }

        // 4. Check Well-Bracketing for Answers
        if mv.kind == MoveKind::Answer {
            if justified_move.kind != MoveKind::Question {
                return Err(ArenaValidationError::InvalidJustificationPointer {
                    step,
                    justifier,
                    reason: "answer move must target a question".into(),
                });
            }

            let active_question = trace.active_question_target(mv.player);
            match active_question {
                Some(expected_target) if expected_target == justifier => {
                    // Valid well-bracketed answer
                }
                Some(expected_target) => {
                    return Err(ArenaValidationError::WellBracketingViolation {
                        step,
                        attempted_target: justifier,
                        expected_target,
                    });
                }
                None => {
                    return Err(ArenaValidationError::WellBracketingViolation {
                        step,
                        attempted_target: justifier,
                        expected_target: usize::MAX,
                    });
                }
            }
        }
    }

    Ok(())
}
