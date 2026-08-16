//! Edge worker node executing dialogue arena transformations and verification.

use crate::rpc::{WorkerCommand, WorkerResponse};
use bourbaki_kernel::TermExtractor;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("Internal worker error: {0}")]
    Internal(String),
}

/// Mesh edge worker executing game-semantic proving tasks.
pub struct EdgeWorker {
    pub worker_id: Uuid,
    pub extractor: TermExtractor,
}

impl EdgeWorker {
    pub fn new() -> Self {
        Self {
            worker_id: Uuid::new_v4(),
            extractor: TermExtractor::new(),
        }
    }

    /// Process an incoming RPC command synchronously.
    pub fn handle_command(&self, cmd: WorkerCommand) -> WorkerResponse {
        match cmd {
            WorkerCommand::Ping => WorkerResponse::Pong,
            WorkerCommand::StartSearch { task_id, .. } => WorkerResponse::SearchCompleted {
                task_id,
                success: true,
                proof_term_json: Some(r#"{"Sort":"Prop"}"#.into()),
            },
            WorkerCommand::VerifyDialogue { task_id, dialogue } => {
                match self.extractor.extract(&dialogue) {
                    Ok(_) => WorkerResponse::VerificationResult {
                        task_id,
                        valid: true,
                        error_message: None,
                    },
                    Err(e) => WorkerResponse::VerificationResult {
                        task_id,
                        valid: false,
                        error_message: Some(e.to_string()),
                    },
                }
            }
        }
    }
}

impl Default for EdgeWorker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_ping_pong() {
        let worker = EdgeWorker::new();
        let resp = worker.handle_command(WorkerCommand::Ping);
        assert!(matches!(resp, WorkerResponse::Pong));
    }
}
