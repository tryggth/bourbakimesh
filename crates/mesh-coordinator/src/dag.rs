//! Global Proof DAG and Distributed Task Queue for BourbakiMesh.

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use kernel::ast::{DeductionStep, Expr as PropExpr, ProofStatus};
use kernel::cic::expr::Expr as CicExpr;
use kernel::state::ProofState;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    pub id: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub hyps: HashMap<String, PropExpr>,
    pub target: PropExpr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cic_target: Option<CicExpr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cic_proof_term: Option<CicExpr>,
    pub goal_repr: String,
    pub status: ProofStatus,
    pub tactic_applied: Option<DeductionStep>,
    pub assigned_worker: Option<String>,
    pub lease_expires: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofDag {
    pub nodes: HashMap<String, DagNode>,
    pub roots: Vec<String>,
}

impl ProofDag {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            roots: Vec::new(),
        }
    }

    pub fn insert_root(&mut self, theorem_name: &str, initial_hyps: Vec<(String, PropExpr)>, target: PropExpr) -> String {
        let root_id = format!("dag-root-{}", uuid::Uuid::new_v4());
        let hyps_map: HashMap<String, PropExpr> = initial_hyps.into_iter().collect();
        let goal_repr = format!("{} ⊢ {:?}", theorem_name, target);

        let root_node = DagNode {
            id: root_id.clone(),
            parent: None,
            children: Vec::new(),
            hyps: hyps_map,
            target,
            cic_target: None,
            cic_proof_term: None,
            goal_repr,
            status: ProofStatus::Open,
            tactic_applied: None,
            assigned_worker: None,
            lease_expires: None,
        };

        self.nodes.insert(root_id.clone(), root_node);
        self.roots.push(root_id.clone());
        root_id
    }

    pub fn insert_cic_root(&mut self, theorem_name: &str, cic_target: CicExpr) -> String {
        let root_id = format!("dag-cic-root-{}", uuid::Uuid::new_v4());
        let goal_repr = format!("{} : {:?}", theorem_name, cic_target);

        let root_node = DagNode {
            id: root_id.clone(),
            parent: None,
            children: Vec::new(),
            hyps: HashMap::new(),
            target: PropExpr::Prop(theorem_name.to_string()),
            cic_target: Some(cic_target),
            cic_proof_term: None,
            goal_repr,
            status: ProofStatus::Open,
            tactic_applied: None,
            assigned_worker: None,
            lease_expires: None,
        };

        self.nodes.insert(root_id.clone(), root_node);
        self.roots.push(root_id.clone());
        root_id
    }

    pub fn get_node(&self, node_id: &str) -> Option<&DagNode> {
        self.nodes.get(node_id)
    }

    pub fn mark_cic_proven(&mut self, node_id: &str, proof_term: CicExpr) -> Result<(), String> {
        let node = self.nodes.get_mut(node_id).ok_or_else(|| "Node not found".to_string())?;
        node.status = ProofStatus::Proven;
        node.cic_proof_term = Some(proof_term);

        // Backpropagate proven status to parents if any
        let mut curr_id = node.parent.clone();
        while let Some(pid) = curr_id {
            if let Some(pnode) = self.nodes.get_mut(&pid) {
                pnode.status = ProofStatus::Proven;
                curr_id = pnode.parent.clone();
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Evaluates step AST with kernel and branches child node or marks proven.
    pub fn apply_step_and_branch(
        &mut self,
        node_id: &str,
        step: &DeductionStep,
    ) -> Result<(String, ProofStatus), String> {
        let node = self.nodes.get_mut(node_id).ok_or_else(|| "Node not found".to_string())?;

        let mut kernel_state = ProofState::new(
            node.hyps.clone().into_iter().collect(),
            node.target.clone(),
        );

        match kernel_state.apply_step(step) {
            Ok(_) => {
                let is_proven = kernel_state.status == ProofStatus::Proven
                    || kernel_state.hyps.values().any(|h| *h == kernel_state.target);
                let status = if is_proven { ProofStatus::Proven } else { ProofStatus::Open };

                node.status = status.clone();
                node.tactic_applied = Some(step.clone());

                if is_proven {
                    // Backpropagate proven status
                    let mut curr_id = node.parent.clone();
                    while let Some(pid) = curr_id {
                        if let Some(pnode) = self.nodes.get_mut(&pid) {
                            pnode.status = ProofStatus::Proven;
                            curr_id = pnode.parent.clone();
                        } else {
                            break;
                        }
                    }
                    Ok((node_id.to_string(), ProofStatus::Proven))
                } else {
                    // Create child node representing new hypothesis context
                    let child_id = format!("dag-node-{}", uuid::Uuid::new_v4());
                    let child_node = DagNode {
                        id: child_id.clone(),
                        parent: Some(node_id.to_string()),
                        children: Vec::new(),
                        hyps: kernel_state.hyps,
                        target: kernel_state.target.clone(),
                        cic_target: None,
                        cic_proof_term: None,
                        goal_repr: format!("⊢ {:?}", kernel_state.target),
                        status: ProofStatus::Open,
                        tactic_applied: None,
                        assigned_worker: None,
                        lease_expires: None,
                    };

                    node.children.push(child_id.clone());
                    self.nodes.insert(child_id.clone(), child_node);
                    Ok((child_id, ProofStatus::Open))
                }
            }
            Err(err) => Err(format!("Kernel validation error: {:?}", err)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    pub node_id: String,
    pub theorem_name: String,
    pub hyps: HashMap<String, PropExpr>,
    pub target: PropExpr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cic_target: Option<CicExpr>,
    pub priority: u32,
    pub created_at: u64,
    pub lease_worker: Option<String>,
    pub lease_expires: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct TaskQueue {
    tasks: HashMap<String, Task>,
    pending_queue: VecDeque<String>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            pending_queue: VecDeque::new(),
        }
    }

    pub fn push_task(
        &mut self,
        node_id: String,
        theorem_name: String,
        hyps: HashMap<String, PropExpr>,
        target: PropExpr,
        priority: u32,
    ) -> String {
        let task_id = format!("task-{}", uuid::Uuid::new_v4());
        let task = Task {
            task_id: task_id.clone(),
            node_id,
            theorem_name,
            hyps,
            target,
            cic_target: None,
            priority,
            created_at: now_secs(),
            lease_worker: None,
            lease_expires: None,
        };

        self.tasks.insert(task_id.clone(), task);
        self.pending_queue.push_back(task_id.clone());
        task_id
    }

    pub fn push_cic_task(
        &mut self,
        node_id: String,
        theorem_name: String,
        cic_target: CicExpr,
        priority: u32,
    ) -> String {
        let task_id = format!("task-cic-{}", uuid::Uuid::new_v4());
        let task = Task {
            task_id: task_id.clone(),
            node_id,
            theorem_name: theorem_name.clone(),
            hyps: HashMap::new(),
            target: PropExpr::Prop(theorem_name),
            cic_target: Some(cic_target),
            priority,
            created_at: now_secs(),
            lease_worker: None,
            lease_expires: None,
        };

        self.tasks.insert(task_id.clone(), task);
        self.pending_queue.push_back(task_id.clone());
        task_id
    }

    pub fn lease_next_task(&mut self, worker_id: &str, lease_secs: u64) -> Option<Task> {
        self.reclaim_expired_leases();

        while let Some(task_id) = self.pending_queue.pop_front() {
            if let Some(task) = self.tasks.get_mut(&task_id) {
                let expires = now_secs() + lease_secs;
                task.lease_worker = Some(worker_id.to_string());
                task.lease_expires = Some(expires);
                return Some(task.clone());
            }
        }
        None
    }

    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    pub fn complete_task(&mut self, task_id: &str) -> Option<Task> {
        self.tasks.remove(task_id)
    }

    pub fn reclaim_expired_leases(&mut self) {
        let now = now_secs();
        for (task_id, task) in &mut self.tasks {
            if let Some(exp) = task.lease_expires {
                if now > exp {
                    task.lease_worker = None;
                    task.lease_expires = None;
                    self.pending_queue.push_back(task_id.clone());
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }
}
