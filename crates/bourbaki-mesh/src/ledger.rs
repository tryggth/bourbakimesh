//! Distributed proof ledger client and block attestations.

pub use crate::block::{BlockId, ProofBlock};
pub use crate::dag::{LedgerError, ProofLedger};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A verified proof attestation recorded in the distributed proof ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofAttestation {
    pub attestation_id: Uuid,
    pub theorem_name: String,
    pub proof_hash: String,
    pub worker_id: Uuid,
    pub timestamp_ms: u64,
}

/// A block of proof attestations committed to the Bourbaki ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerBlock {
    pub block_index: u64,
    pub previous_hash: String,
    pub attestations: Vec<ProofAttestation>,
    pub block_hash: String,
}

/// Client interacting with the Bourbaki distributed proof ledger network.
#[derive(Debug, Clone, Default)]
pub struct LedgerClient {
    pub node_id: Uuid,
}

impl LedgerClient {
    pub fn new() -> Self {
        Self {
            node_id: Uuid::new_v4(),
        }
    }

    /// Submit a proof attestation to the ledger pool.
    pub fn submit_attestation(
        &self,
        theorem_name: impl Into<String>,
        proof_hash: impl Into<String>,
    ) -> ProofAttestation {
        ProofAttestation {
            attestation_id: Uuid::new_v4(),
            theorem_name: theorem_name.into(),
            proof_hash: proof_hash.into(),
            worker_id: self.node_id,
            timestamp_ms: 1_700_000_000_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ledger_client_submission() {
        let client = LedgerClient::new();
        let att = client.submit_attestation("theorem_fermat_little", "0xdeadbeef1234");
        assert_eq!(att.theorem_name, "theorem_fermat_little");
        assert_eq!(att.worker_id, client.node_id);
    }
}
