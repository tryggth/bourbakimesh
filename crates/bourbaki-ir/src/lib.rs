//! Bourbaki IR: Core Game-Semantic Arena Intermediate Representation.
//!
//! Provides AST definitions and validation engines for Hyland-Ong and Lorenzen
//! dialogue games, P-view and O-view calculations, well-bracketing stack discipline,
//! and interaction net polarity reductions.

pub mod arena;
pub mod moves;
pub mod net;
pub mod polarity;
pub mod trace;
pub mod validator;

pub use arena::{ArenaDialogue, StrategyNode, StrategyTree};
pub use moves::{ConjunctionBranch, LogicalPayload, Move, MoveKind};
pub use net::{InteractionNet, NetAgent, Port};
pub use polarity::Polarity;
pub use trace::PlayTrace;
pub use validator::{verify_all, verify_next_move, ArenaValidationError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_ir() {
        let trace = PlayTrace::new();
        assert!(trace.is_empty());
        assert_eq!(trace.len(), 0);
    }
}
