//! Asynchronous IPC socket server for coordinator-worker communication.

use crate::node::MeshCoordinator;
use crate::protocol::{WorkerCommand, WorkerResponse};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, UnixListener};
use tokio::sync::{oneshot, Mutex};

/// Errors arising during IPC server operations.
#[derive(Debug, Error)]
pub enum IpcError {
    #[error("IO error in IPC transport: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IPC server error: {0}")]
    ServerError(String),
}

/// Tokio-based asynchronous IPC server for BourbakiMesh coordinator.
pub struct MeshIpcServer {
    pub local_addr: String,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl MeshIpcServer {
    /// Bind and serve over TCP.
    pub async fn bind_tcp(
        addr: &str,
        coordinator: Arc<Mutex<MeshCoordinator>>,
    ) -> Result<(Self, String), IpcError> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?.to_string();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let coord_clone = coordinator.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        break;
                    }
                    accept_res = listener.accept() => {
                        if let Ok((mut socket, _)) = accept_res {
                            let coord = coord_clone.clone();
                            tokio::spawn(async move {
                                let (reader, mut writer) = socket.split();
                                let mut lines = BufReader::new(reader).lines();

                                while let Ok(Some(line)) = lines.next_line().await {
                                    let trimmed = line.trim();
                                    if trimmed.is_empty() {
                                        continue;
                                    }

                                    let resp = match serde_json::from_str::<WorkerCommand>(trimmed) {
                                        Ok(cmd) => {
                                            let mut guard = coord.lock().await;
                                            guard.handle_command(cmd)
                                        }
                                        Err(err) => WorkerResponse::ProofRejected {
                                            task_id: uuid::Uuid::nil(),
                                            reason: format!("Malformed command JSON: {}", err),
                                        },
                                    };

                                    if let Ok(mut serialized) = serde_json::to_string(&resp) {
                                        serialized.push('\n');
                                        if writer.write_all(serialized.as_bytes()).await.is_err() {
                                            break;
                                        }
                                        let _ = writer.flush().await;
                                    }
                                }
                            });
                        }
                    }
                }
            }
        });

        let server = Self {
            local_addr: local_addr.clone(),
            shutdown_tx: Some(shutdown_tx),
        };

        Ok((server, local_addr))
    }

    /// Bind and serve over Unix Domain Sockets (UDS).
    pub async fn bind_uds(
        socket_path: impl AsRef<Path>,
        coordinator: Arc<Mutex<MeshCoordinator>>,
    ) -> Result<Self, IpcError> {
        let path = socket_path.as_ref().to_path_buf();
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }

        let listener = UnixListener::bind(&path)?;
        let local_addr = path.to_string_lossy().to_string();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let coord_clone = coordinator.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        break;
                    }
                    accept_res = listener.accept() => {
                        if let Ok((mut socket, _)) = accept_res {
                            let coord = coord_clone.clone();
                            tokio::spawn(async move {
                                let (reader, mut writer) = socket.split();
                                let mut lines = BufReader::new(reader).lines();

                                while let Ok(Some(line)) = lines.next_line().await {
                                    let trimmed = line.trim();
                                    if trimmed.is_empty() {
                                        continue;
                                    }

                                    let resp = match serde_json::from_str::<WorkerCommand>(trimmed) {
                                        Ok(cmd) => {
                                            let mut guard = coord.lock().await;
                                            guard.handle_command(cmd)
                                        }
                                        Err(err) => WorkerResponse::ProofRejected {
                                            task_id: uuid::Uuid::nil(),
                                            reason: format!("Malformed command JSON: {}", err),
                                        },
                                    };

                                    if let Ok(mut serialized) = serde_json::to_string(&resp) {
                                        serialized.push('\n');
                                        if writer.write_all(serialized.as_bytes()).await.is_err() {
                                            break;
                                        }
                                        let _ = writer.flush().await;
                                    }
                                }
                            });
                        }
                    }
                }
            }
        });

        let server = Self {
            local_addr,
            shutdown_tx: Some(shutdown_tx),
        };

        Ok(server)
    }

    /// Stop the server listener.
    pub fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
