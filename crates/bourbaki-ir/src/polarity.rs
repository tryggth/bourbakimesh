//! Polarity and player duality in game-semantic dialogue arenas.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Player polarity in a Hyland-Ong / Lorenzen dialogue game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Polarity {
    /// Proponent (the prover asserting truth of the theorem, opens at step 0).
    Proponent,
    /// Opponent (the skeptic attempting to find counter-strategies).
    Opponent,
}

impl Polarity {
    /// Returns the opposite player polarity.
    pub fn dual(&self) -> Self {
        match self {
            Polarity::Proponent => Polarity::Opponent,
            Polarity::Opponent => Polarity::Proponent,
        }
    }

    /// Returns true if the player is Proponent.
    pub fn is_proponent(&self) -> bool {
        matches!(self, Polarity::Proponent)
    }

    /// Returns true if the player is Opponent.
    pub fn is_opponent(&self) -> bool {
        matches!(self, Polarity::Opponent)
    }
}

impl fmt::Display for Polarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Polarity::Proponent => write!(f, "P"),
            Polarity::Opponent => write!(f, "O"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polarity_duality_and_display() {
        assert_eq!(Polarity::Proponent.dual(), Polarity::Opponent);
        assert_eq!(Polarity::Opponent.dual(), Polarity::Proponent);
        assert_eq!(format!("{}", Polarity::Proponent), "P");
        assert_eq!(format!("{}", Polarity::Opponent), "O");
        assert!(Polarity::Proponent.is_proponent());
        assert!(!Polarity::Proponent.is_opponent());
        assert!(Polarity::Opponent.is_opponent());
        assert!(!Polarity::Opponent.is_proponent());
    }
}
