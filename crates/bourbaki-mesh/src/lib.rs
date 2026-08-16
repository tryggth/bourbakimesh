//! Bourbaki Mesh: Distributed edge worker node, proof DAG ledger, and RPC interfaces.

pub mod block;
pub mod dag;
pub mod ledger;
pub mod node;
pub mod protocol;
pub mod rpc;
pub mod worker;

pub use block::{BlockId, ProofBlock};
pub use dag::{LedgerError, ProofLedger};
pub use node::MeshCoordinator;
pub use protocol::{WorkerCommand, WorkerResponse};
pub use worker::EdgeWorker;

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
