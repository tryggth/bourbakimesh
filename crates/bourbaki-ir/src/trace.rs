//! PlayTrace data structure and Hyland-Ong view slicing.

use crate::moves::{Move, MoveKind};
use crate::polarity::Polarity;
use crate::validator::{verify_next_move, ArenaValidationError};
use serde::{Deserialize, Serialize};

/// A sequence of justified dialogue moves representing a game play trace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayTrace {
    moves: Vec<Move>,
}

impl PlayTrace {
    /// Create a new empty play trace.
    pub fn new() -> Self {
        Self { moves: Vec::new() }
    }

    /// Number of moves in the trace.
    pub fn len(&self) -> usize {
        self.moves.len()
    }

    /// True if the trace has no moves.
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// Read-only slice of moves in the trace.
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// Mutable access to internal move storage (for internal building).
    pub fn moves_mut(&mut self) -> &mut Vec<Move> {
        &mut self.moves
    }

    /// Retrieve a move by its 0-indexed position.
    pub fn get(&self, index: usize) -> Option<&Move> {
        self.moves.get(index)
    }

    /// Validate and append a move to the play trace.
    pub fn push(&mut self, mv: Move) -> Result<(), ArenaValidationError> {
        verify_next_move(self, &mv)?;
        self.moves.push(mv);
        Ok(())
    }

    /// Compute the Proponent view (P-view) indices $\ulcorner s \urcorner_P$.
    pub fn get_p_view(&self) -> Vec<usize> {
        compute_p_view(&self.moves, self.moves.len())
    }

    /// Compute the Opponent view (O-view) indices $\llcorner s \lrcorner_O$.
    pub fn get_o_view(&self) -> Vec<usize> {
        compute_o_view(&self.moves, self.moves.len())
    }

    /// Compute the P-view for a specific prefix length.
    pub fn get_p_view_prefix(&self, prefix_len: usize) -> Vec<usize> {
        let bounded_len = prefix_len.min(self.moves.len());
        compute_p_view(&self.moves, bounded_len)
    }

    /// Compute the O-view for a specific prefix length.
    pub fn get_o_view_prefix(&self, prefix_len: usize) -> Vec<usize> {
        let bounded_len = prefix_len.min(self.moves.len());
        compute_o_view(&self.moves, bounded_len)
    }

    /// Return indices of all open (unanswered) questions in the specified player's view.
    pub fn open_questions_in_view(&self, player: Polarity) -> Vec<usize> {
        let view = match player {
            Polarity::Proponent => self.get_p_view(),
            Polarity::Opponent => self.get_o_view(),
        };

        let mut open_questions = Vec::new();
        for &idx in &view {
            if idx < self.moves.len() {
                let m = &self.moves[idx];
                match m.kind {
                    MoveKind::Question => {
                        open_questions.push(idx);
                    }
                    MoveKind::Answer => {
                        if let Some(target) = m.justifier {
                            if let Some(pos) = open_questions.iter().rposition(|&q| q == target) {
                                open_questions.remove(pos);
                            }
                        }
                    }
                }
            }
        }
        open_questions
    }

    /// Return the active question target (the most recently posed unanswered question in view).
    pub fn active_question_target(&self, player: Polarity) -> Option<usize> {
        self.open_questions_in_view(player).last().copied()
    }
}

/// Recursive computation of the Proponent view (P-view).
fn compute_p_view(moves: &[Move], prefix_len: usize) -> Vec<usize> {
    if prefix_len == 0 || prefix_len > moves.len() {
        return Vec::new();
    }
    let last_idx = prefix_len - 1;
    let last_mv = &moves[last_idx];
    match last_mv.player {
        Polarity::Proponent => {
            let mut view = compute_p_view(moves, last_idx);
            view.push(last_idx);
            view
        }
        Polarity::Opponent => match last_mv.justifier {
            Some(k) if k < last_idx => {
                let mut view = compute_p_view(moves, k + 1);
                view.push(last_idx);
                view
            }
            _ => {
                vec![last_idx]
            }
        },
    }
}

/// Recursive computation of the Opponent view (O-view).
fn compute_o_view(moves: &[Move], prefix_len: usize) -> Vec<usize> {
    if prefix_len == 0 || prefix_len > moves.len() {
        return Vec::new();
    }
    let last_idx = prefix_len - 1;
    let last_mv = &moves[last_idx];
    match last_mv.player {
        Polarity::Opponent => {
            let mut view = compute_o_view(moves, last_idx);
            view.push(last_idx);
            view
        }
        Polarity::Proponent => match last_mv.justifier {
            Some(k) if k < last_idx => {
                let mut view = compute_o_view(moves, k + 1);
                view.push(last_idx);
                view
            }
            _ => {
                vec![last_idx]
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::LogicalPayload;

    #[test]
    fn test_trace_view_computation() {
        let mut trace = PlayTrace::new();
        // Step 0: P root
        trace
            .push(Move::root_goal("A -> B -> A"))
            .expect("Step 0 failed");
        assert_eq!(trace.get_p_view(), vec![0]);
        assert_eq!(trace.get_o_view(), vec![0]);

        // Step 1: O attacks hypothesis A (justifier 0)
        trace
            .push(Move::question(
                1,
                Polarity::Opponent,
                0,
                LogicalPayload::AttackHypothesis { hyp_id: 0 },
            ))
            .expect("Step 1 failed");
        assert_eq!(trace.get_p_view(), vec![0, 1]);
        assert_eq!(trace.get_o_view(), vec![0, 1]);

        // Step 2: P responds by challenging hypothesis B (justifier 1)
        trace
            .push(Move::question(
                2,
                Polarity::Proponent,
                1,
                LogicalPayload::AttackHypothesis { hyp_id: 1 },
            ))
            .expect("Step 2 failed");
        assert_eq!(trace.get_p_view(), vec![0, 1, 2]);
        assert_eq!(trace.get_o_view(), vec![0, 1, 2]);

        // Step 3: O branches back to challenge step 0 directly
        trace
            .push(Move::question(
                3,
                Polarity::Opponent,
                0,
                LogicalPayload::AttackHypothesis { hyp_id: 2 },
            ))
            .expect("Step 3 failed");
        // For P-view: O points to 0, so P-view drops steps 1 & 2!
        assert_eq!(trace.get_p_view(), vec![0, 3]);
        // For O-view: O retains its own history [0, 1, 2, 3]
        assert_eq!(trace.get_o_view(), vec![0, 1, 2, 3]);
    }
}
