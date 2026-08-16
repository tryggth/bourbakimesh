//! Worker RPC message payloads and networking protocol.

use crate::block::BlockId;
use bourbaki_ir::{ArenaDialogue, PlayTrace, StrategyTree};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// RPC command dispatched from/to mesh nodes and workers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerCommand {
    /// Worker claims or requests a proving task.
    ClaimTask {
        task_id: Uuid,
        goal_statement: String,
        max_simulations: usize,
    },
    /// Worker submits a candidate game-semantic winning strategy tree.
    SubmitProof {
        task_id: Uuid,
        strategy: StrategyTree,
    },
    /// Worker submits a refutation counter-trace proving refutation.
    SubmitRefutation {
        task_id: Uuid,
        counter_trace: PlayTrace,
    },
    /// Heartbeat signal indicating worker liveness.
    Heartbeat { worker_id: String },
    /// Ping signal for latency and connectivity checks.
    Ping,
    /// Initiate a search task on the worker.
    StartSearch {
        task_id: Uuid,
        goal_statement: String,
    },
    /// Direct verification request for an arena dialogue play.
    VerifyDialogue {
        task_id: Uuid,
        dialogue: ArenaDialogue,
    },
}

/// RPC response emitted by coordinator or worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerResponse {
    /// Proof search task assigned.
    TaskAssigned {
        task_id: Uuid,
        goal_statement: String,
    },
    /// Proof successfully compiled, certified, and merged into ledger DAG.
    ProofAccepted { task_id: Uuid, block_id: BlockId },
    /// Proof rejected due to invalid strategy or Lean kernel type failure.
    ProofRejected { task_id: Uuid, reason: String },
    /// Verification result for a dialogue play.
    VerificationResult {
        task_id: Uuid,
        valid: bool,
        error_message: Option<String>,
    },
    /// Search completed.
    SearchCompleted {
        task_id: Uuid,
        success: bool,
        proof_term_json: Option<String>,
    },
    /// General positive acknowledgement.
    Acknowledged,
    /// Response to Ping.
    Pong,
}
