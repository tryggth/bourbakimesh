//! Bourbaki Mesh: Edge Worker Networking, RPC Interface, and Distributed Proof Ledger.

pub mod ledger;
pub mod rpc;
pub mod worker;

pub use ledger::{LedgerBlock, LedgerClient, ProofAttestation};
pub use rpc::{WorkerCommand, WorkerResponse};
pub use worker::{EdgeWorker, WorkerError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_mesh() {
        let worker = EdgeWorker::new();
        assert_eq!(worker.handle_command(WorkerCommand::Ping), WorkerResponse::Pong);
    }
}
