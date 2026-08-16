//! Mesh coordinator node managing proof task dispatch and ledger integration.

use crate::block::ProofBlock;
use crate::dag::ProofLedger;
use crate::protocol::{WorkerCommand, WorkerResponse};
use bourbaki_kernel::{LeanEnvironment, StrategyExtractor, Term};
use std::collections::HashMap;
use uuid::Uuid;

/// Central coordinator orchestrating distributed proof search and ledger validation.
pub struct MeshCoordinator {
    pub ledger: ProofLedger,
    pub verifier: Option<LeanEnvironment>,
    pub active_tasks: HashMap<Uuid, String>,
}

impl MeshCoordinator {
    /// Create a new coordinator with an existing or genesis proof ledger.
    pub fn new(ledger: ProofLedger) -> Self {
        Self {
            ledger,
            verifier: None,
            active_tasks: HashMap::new(),
        }
    }

    /// Attach a Lean 4 verification environment gate.
    pub fn with_verifier(mut self, verifier: LeanEnvironment) -> Self {
        self.verifier = Some(verifier);
        self
    }

    /// Dispatch a new proof obligation to the active task pool.
    pub fn dispatch_task(&mut self, goal_statement: &str) -> Uuid {
        let task_id = Uuid::new_v4();
        self.active_tasks
            .insert(task_id, goal_statement.to_string());
        task_id
    }

    /// Synchronously process an RPC command from an edge worker.
    pub fn handle_command(&mut self, cmd: WorkerCommand) -> WorkerResponse {
        match cmd {
            WorkerCommand::Ping => WorkerResponse::Pong,
            WorkerCommand::Heartbeat { .. } => WorkerResponse::Acknowledged,
            WorkerCommand::ClaimTask {
                task_id,
                goal_statement,
                ..
            } => {
                self.active_tasks.insert(task_id, goal_statement.clone());
                WorkerResponse::TaskAssigned {
                    task_id,
                    goal_statement,
                }
            }
            WorkerCommand::StartSearch {
                task_id,
                goal_statement,
            } => {
                self.active_tasks.insert(task_id, goal_statement.clone());
                WorkerResponse::TaskAssigned {
                    task_id,
                    goal_statement,
                }
            }
            WorkerCommand::SubmitProof { task_id, strategy } => {
                let goal = self
                    .active_tasks
                    .get(&task_id)
                    .cloned()
                    .unwrap_or_else(|| "UnknownGoal".into());

                // 1. Lower strategy to CIC term
                let cic_term = match StrategyExtractor::compile_strategy(&strategy) {
                    Ok(term) => term,
                    Err(e) => {
                        return WorkerResponse::ProofRejected {
                            task_id,
                            reason: format!("Strategy extraction compiler error: {}", e),
                        };
                    }
                };

                // 2. If verifier is configured, execute Lean 4 operational gate
                let certified = true;
                if let Some(verifier) = &self.verifier {
                    let thm_name = format!("thm_{}", task_id.simple());
                    let prop_term = Term::var(&goal);
                    if let Err(err) = verifier.verify_term(&thm_name, &prop_term, &cic_term) {
                        return WorkerResponse::ProofRejected {
                            task_id,
                            reason: format!("Lean 4 kernel rejected proof: {}", err),
                        };
                    }
                }

                // 3. Commit to proof ledger DAG
                let parent_ids = vec![];
                let block = ProofBlock::new(
                    parent_ids,
                    format!("Theorem_{}", task_id.simple()),
                    goal,
                    Some(strategy),
                    Some(cic_term),
                    certified,
                    1700000000,
                );

                let block_id = match self.ledger.insert_block(block) {
                    Ok(id) => id,
                    Err(e) => {
                        return WorkerResponse::ProofRejected {
                            task_id,
                            reason: format!("Ledger insertion error: {}", e),
                        };
                    }
                };

                self.active_tasks.remove(&task_id);
                WorkerResponse::ProofAccepted { task_id, block_id }
            }
            WorkerCommand::SubmitRefutation { task_id, .. } => {
                self.active_tasks.remove(&task_id);
                WorkerResponse::Acknowledged
            }
            WorkerCommand::VerifyDialogue { task_id, dialogue } => {
                match StrategyExtractor::compile_trace(dialogue.trace()) {
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
