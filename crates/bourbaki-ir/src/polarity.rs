//! Dialogue game and interaction net polarities.

use serde::{Deserialize, Serialize};

/// Represents the player polarity in a game-semantic dialogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Polarity {
    /// Proponent (the prover, asserting truth of the theorem).
    Proponent,
    /// Opponent (the skeptic, attempting to find counter-strategies).
    Opponent,
}

impl Polarity {
    /// Returns the dual (opposite) polarity.
    pub fn dual(self) -> Self {
        match self {
            Polarity::Proponent => Polarity::Opponent,
            Polarity::Opponent => Polarity::Proponent,
        }
    }

    /// True if the current polarity is Proponent.
    pub fn is_proponent(self) -> bool {
        matches!(self, Polarity::Proponent)
    }

    /// True if the current polarity is Opponent.
    pub fn is_opponent(self) -> bool {
        matches!(self, Polarity::Opponent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polarity_duality() {
        assert_eq!(Polarity::Proponent.dual(), Polarity::Opponent);
        assert_eq!(Polarity::Opponent.dual(), Polarity::Proponent);
        assert_eq!(Polarity::Proponent.dual().dual(), Polarity::Proponent);
    }
}
