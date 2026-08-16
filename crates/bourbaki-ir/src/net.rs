//! Interaction net and proof net structures for game-semantic arena reduction.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Port on an interaction net agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Port {
    pub agent_id: Uuid,
    pub port_index: usize,
}

/// Principal or auxiliary agent node in an interaction net.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetAgent {
    pub id: Uuid,
    pub symbol: String,
    pub arity: usize,
}

/// Interaction net representing cut-elimination / dialogue game normalization.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionNet {
    agents: Vec<NetAgent>,
    connections: Vec<(Port, Port)>,
}

impl InteractionNet {
    /// Creates an empty interaction net.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an agent to the net.
    pub fn add_agent(&mut self, symbol: impl Into<String>, arity: usize) -> Uuid {
        let id = Uuid::new_v4();
        self.agents.push(NetAgent {
            id,
            symbol: symbol.into(),
            arity,
        });
        id
    }

    /// Connect two ports.
    pub fn connect(&mut self, p1: Port, p2: Port) {
        self.connections.push((p1, p2));
    }

    /// Total number of agents in the net.
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Total number of wires/connections in the net.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_net_creation() {
        let mut net = InteractionNet::new();
        let a1 = net.add_agent("Delta", 2);
        let a2 = net.add_agent("Epsilon", 1);
        net.connect(
            Port {
                agent_id: a1,
                port_index: 0,
            },
            Port {
                agent_id: a2,
                port_index: 0,
            },
        );

        assert_eq!(net.agent_count(), 2);
        assert_eq!(net.connection_count(), 1);
    }
}
