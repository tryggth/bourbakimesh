//! RPC messages and protocol definitions for mesh workers.

use bourbaki_ir::ArenaDialogue;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Commands received by mesh workers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerCommand {
    /// Request to initiate self-play or proof search on a theorem goal.
    StartSearch {
        task_id: Uuid,
        theorem_statement: String,
        budget_seconds: u64,
    },
    /// Request to verify an extracted dialogue play trace.
    VerifyDialogue {
        task_id: Uuid,
        dialogue: ArenaDialogue,
    },
    /// Ping message for health & heartbeat.
    Ping,
}

/// Responses sent by mesh workers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerResponse {
    SearchCompleted {
        task_id: Uuid,
        success: bool,
        proof_term_json: Option<String>,
    },
    VerificationResult {
        task_id: Uuid,
        valid: bool,
        error_message: Option<String>,
    },
    Pong,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_serialization() {
        let cmd = WorkerCommand::Ping;
        let serialized = serde_json::to_string(&cmd).expect("Serialization failed");
        let deserialized: WorkerCommand =
            serde_json::from_str(&serialized).expect("Deserialization failed");
        matches!(deserialized, WorkerCommand::Ping);
    }
}
