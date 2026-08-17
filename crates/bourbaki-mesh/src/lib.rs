//! Bourbaki Mesh: Distributed edge worker node, proof DAG ledger, libp2p GossipSub/Kademlia P2P protocol, and RPC interfaces.

pub mod block;
pub mod consensus;
pub mod dag;
pub mod ipc_server;
pub mod ledger;
pub mod node;
pub mod p2p;
pub mod protocol;
pub mod rpc;
pub mod worker;

pub use block::{BlockId, ProofBlock};
pub use consensus::{AttestationError, ProofAttestationEngine};
pub use dag::{LedgerError, ProofLedger};
pub use ipc_server::{IpcError, MeshIpcServer};
pub use node::MeshCoordinator;
pub use p2p::{
    BourbakiBehaviour, P2PConfig, P2PError, P2PEvent, P2PNode, ProofGossipMessage,
    TaskGossipMessage, TOPIC_PROOFS, TOPIC_TASKS,
};
pub use protocol::{WorkerCommand, WorkerResponse};
pub use worker::{EdgeWorker, MeshWorkerDaemon, WorkerDaemonEvent, WorkerError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_mesh() {
        let genesis = ProofBlock::genesis("True");
        assert_eq!(genesis.theorem_name, "Bourbaki.Genesis");
        assert!(genesis.certified);
    }
}
