//! Bourbaki IR: Core Game-Semantic Arena Intermediate Representation.
//!
//! Provides data structures for Hyland-Ong and Lorenzen dialogue games,
//! polarity-driven interaction nets, and play traces.

pub mod arena;
pub mod net;
pub mod polarity;

pub use arena::{ArenaDialogue, ArenaError, Move, MoveKind};
pub use net::{InteractionNet, NetAgent, Port};
pub use polarity::Polarity;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_ir() {
        let dialogue = ArenaDialogue::new(Polarity::Opponent);
        assert!(dialogue.is_empty());
    }
}
