//! WebSocket JSON-RPC 2.0 Server for Distributed Mesh Coordination.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::tungstenite::Message;
use kernel::ast::{DeductionStep, Expr, ProofStatus};
use crate::dag::{ProofDag, TaskQueue};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub worker_id: String,
    pub model: String,
    pub vram_limit_mb: u32,
    pub throughput_tok_s: f64,
    pub last_heartbeat: u64,
    pub tasks_completed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: Option<String>,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

pub struct CoordinatorState {
    pub dag: ProofDag,
    pub task_queue: TaskQueue,
    pub workers: HashMap<String, WorkerInfo>,
    pub total_tasks_resolved: u64,
}

impl CoordinatorState {
    pub fn new() -> Self {
        Self {
            dag: ProofDag::new(),
            task_queue: TaskQueue::new(),
            workers: HashMap::new(),
            total_tasks_resolved: 0,
        }
    }
}

pub struct MeshCoordinatorServer {
    state: Arc<RwLock<CoordinatorState>>,
    broadcast_tx: broadcast::Sender<String>,
}

impl MeshCoordinatorServer {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(256);
        Self {
            state: Arc::new(RwLock::new(CoordinatorState::new())),
            broadcast_tx,
        }
    }

    pub fn get_state(&self) -> Arc<RwLock<CoordinatorState>> {
        self.state.clone()
    }

    pub async fn run(&self, addr_str: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr: SocketAddr = addr_str.parse()?;
        let listener = TcpListener::bind(&addr).await?;
        println!("🚀 [Mesh Coordinator] Listening on ws://{}", addr);

        while let Ok((stream, client_addr)) = listener.accept().await {
            let state = self.state.clone();
            let btx = self.broadcast_tx.clone();
            let brx = btx.subscribe();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, client_addr, state, btx, brx).await {
                    eprintln!("[Mesh Coordinator] Connection error from {}: {:?}", client_addr, e);
                }
            });
        }

        Ok(())
    }
}

async fn handle_connection(
    stream: TcpStream,
    _addr: SocketAddr,
    state: Arc<RwLock<CoordinatorState>>,
    broadcast_tx: broadcast::Sender<String>,
    mut broadcast_rx: broadcast::Receiver<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    loop {
        tokio::select! {
            // Forward broadcast notifications to client
            Ok(msg_str) = broadcast_rx.recv() => {
                if ws_tx.send(Message::Text(msg_str)).await.is_err() {
                    break;
                }
            }

            // Receive client message
            Some(msg_res) = ws_rx.next() => {
                match msg_res {
                    Ok(Message::Text(text)) => {
                        let resp = process_json_rpc(&text, &state, &broadcast_tx).await;
                        let resp_json = serde_json::to_string(&resp)?;
                        ws_tx.send(Message::Text(resp_json)).await?;
                    }
                    Ok(Message::Ping(p)) => {
                        ws_tx.send(Message::Pong(p)).await?;
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
            else => break,
        }
    }

    Ok(())
}

async fn process_json_rpc(
    raw_json: &str,
    state_lock: &Arc<RwLock<CoordinatorState>>,
    broadcast_tx: &broadcast::Sender<String>,
) -> JsonRpcResponse {
    let req: JsonRpcRequest = match serde_json::from_str(raw_json) {
        Ok(r) => r,
        Err(e) => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(serde_json::json!({ "code": -32700, "message": format!("Parse error: {}", e) })),
            };
        }
    };

    let id = req.id.clone();
    let method = req.method.as_str();
    let params = req.params.unwrap_or(serde_json::Value::Null);

    match method {
        "mesh_register_worker" => {
            let worker_id = params.get("worker_id").and_then(|v| v.as_str()).unwrap_or("worker-anon").to_string();
            let model = params.get("model").and_then(|v| v.as_str()).unwrap_or("gemma-4-edge").to_string();
            let vram_limit_mb = params.get("vram_limit_mb").and_then(|v| v.as_u64()).unwrap_or(4096) as u32;
            let throughput = params.get("throughput_tok_s").and_then(|v| v.as_f64()).unwrap_or(45.0);

            let mut state = state_lock.write().await;
            state.workers.insert(
                worker_id.clone(),
                WorkerInfo {
                    worker_id: worker_id.clone(),
                    model,
                    vram_limit_mb,
                    throughput_tok_s: throughput,
                    last_heartbeat: now_secs(),
                    tasks_completed: 0,
                },
            );

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                    "registered": true,
                    "worker_id": worker_id,
                    "active_workers": state.workers.len(),
                    "network_epoch": 1
                })),
                error: None,
            }
        }

        "mesh_pull_task" => {
            let worker_id = params.get("worker_id").and_then(|v| v.as_str()).unwrap_or("worker-anon");
            let mut state = state_lock.write().await;
            let task_opt = state.task_queue.lease_next_task(worker_id, 30);

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::to_value(task_opt).unwrap_or(serde_json::Value::Null)),
                error: None,
            }
        }

        "mesh_submit_result" => {
            let task_id = params.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
            let worker_id = params.get("worker_id").and_then(|v| v.as_str()).unwrap_or("");
            let step_ast_val = params.get("step_ast");
            let genrm_score = params.get("genrm_score").and_then(|v| v.as_f64()).unwrap_or(0.5);

            let step_ast: Option<DeductionStep> = step_ast_val
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            if step_ast.is_none() {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(serde_json::json!({ "code": -32602, "message": "Invalid step_ast payload" })),
                };
            }

            let step = step_ast.unwrap();
            let mut state = state_lock.write().await;

            let task = match state.task_queue.complete_task(task_id) {
                Some(t) => t,
                None => {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(serde_json::json!({ "code": -32001, "message": "Task not found or expired" })),
                    };
                }
            };

            // Ground truth validation via kernel
            match state.dag.apply_step_and_branch(&task.node_id, &step) {
                Ok((next_node_id, status)) => {
                    state.total_tasks_resolved += 1;
                    if let Some(w) = state.workers.get_mut(worker_id) {
                        w.tasks_completed += 1;
                    }

                    // If not proven, push sub-task to queue
                    if status != ProofStatus::Proven {
                        if let Some(child_node) = state.dag.get_node(&next_node_id).cloned() {
                            state.task_queue.push_task(
                                child_node.id,
                                task.theorem_name.clone(),
                                child_node.hyps,
                                child_node.target,
                                task.priority,
                            );
                        }
                    }

                    // Broadcast DAG sync event to mesh
                    let notification = serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "mesh_dag_updated",
                        "params": {
                            "node_id": task.node_id,
                            "next_node_id": next_node_id,
                            "status": format!("{:?}", status),
                            "step_applied": step,
                            "genrm_score": genrm_score,
                            "total_resolved": state.total_tasks_resolved
                        }
                    });
                    let _ = broadcast_tx.send(notification.to_string());

                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(serde_json::json!({
                            "accepted": true,
                            "status": format!("{:?}", status),
                            "next_node_id": next_node_id,
                            "total_resolved": state.total_tasks_resolved
                        })),
                        error: None,
                    }
                }
                Err(err_msg) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(serde_json::json!({ "code": -32002, "message": err_msg })),
                },
            }
        }

        "mesh_post_goal" => {
            let theorem_name = params.get("theorem_name").and_then(|v| v.as_str()).unwrap_or("goal_anon");
            let hyps_val = params.get("hyps");
            let target_val = params.get("target");

            let hyps: HashMap<String, Expr> = hyps_val
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let target: Expr = match target_val.and_then(|v| serde_json::from_value(v.clone()).ok()) {
                Some(t) => t,
                None => {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(serde_json::json!({ "code": -32602, "message": "Missing target expression" })),
                    };
                }
            };

            let mut state = state_lock.write().await;
            let root_id = state.dag.insert_root(
                theorem_name,
                hyps.clone().into_iter().collect(),
                target.clone(),
            );

            let task_id = state.task_queue.push_task(
                root_id.clone(),
                theorem_name.to_string(),
                hyps,
                target,
                100,
            );

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                    "posted": true,
                    "root_id": root_id,
                    "task_id": task_id
                })),
                error: None,
            }
        }

        "mesh_heartbeat" => {
            let worker_id = params.get("worker_id").and_then(|v| v.as_str()).unwrap_or("worker-anon");
            let mut state = state_lock.write().await;
            if let Some(w) = state.workers.get_mut(worker_id) {
                w.last_heartbeat = now_secs();
            }

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                    "status": "ok",
                    "timestamp": now_secs()
                })),
                error: None,
            }
        }

        "mesh_get_dag" => {
            let state = state_lock.read().await;
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::to_value(&state.dag).unwrap_or(serde_json::Value::Null)),
                error: None,
            }
        }

        "mesh_get_telemetry" => {
            let state = state_lock.read().await;
            let proven_count = state.dag.nodes.values().filter(|n| n.status == ProofStatus::Proven).count();

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                    "active_workers": state.workers.len(),
                    "total_nodes": state.dag.nodes.len(),
                    "tasks_in_queue": state.task_queue.len(),
                    "proven_nodes": proven_count,
                    "total_tasks_resolved": state.total_tasks_resolved
                })),
                error: None,
            }
        }

        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(serde_json::json!({ "code": -32601, "message": format!("Method not found: {}", method) })),
        },
    }
}
