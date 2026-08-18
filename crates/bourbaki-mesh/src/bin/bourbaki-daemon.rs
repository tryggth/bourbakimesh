//! BourbakiMesh Standalone P2P Daemon CLI.
//!
//! Joins the libp2p GossipSub network, claims open Mathlib theorem obligations,
//! executes Latent MCTS proof search, verifies proof terms in Lean 4, and broadcasts attested ProofBlocks.

use bourbaki_kernel::verifier::LeanEnvironment;
use bourbaki_mesh::block::ProofBlock;
use bourbaki_mesh::dag::ProofLedger;
use bourbaki_mesh::p2p::{P2PConfig, P2PNode};
use bourbaki_mesh::worker::{MeshWorkerDaemon, WorkerDaemonEvent};
use clap::Parser;
use libp2p::{Multiaddr, PeerId};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Standalone P2P worker node daemon for the BourbakiMesh theorem proving network.
#[derive(Parser, Debug)]
#[command(
    name = "bourbaki-daemon",
    version = "0.1.0",
    about = "BourbakiMesh P2P Prover Node Daemon"
)]
pub struct DaemonCliArgs {
    /// P2P swarm listener port.
    #[arg(long, default_value_t = 9001)]
    pub peer_port: u16,

    /// Path to trained neural model checkpoint.
    #[arg(long, default_value = "checkpoints/bourbaki_v2.pt")]
    pub model_path: PathBuf,

    /// Number of MCTS simulations per dialogue move.
    #[arg(long, default_value_t = 100)]
    pub simulations: usize,

    /// IPC bridge socket address for Python MCTS / worker communication.
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub ipc_addr: String,

    /// Bootstrap peer multiaddresses to connect on startup.
    #[arg(long = "bootstrap-nodes", num_args = 0..)]
    pub bootstrap_nodes: Vec<Multiaddr>,

    /// Path to Lean 4 target verification harness.
    #[arg(long, default_value = "lean_target")]
    pub lean_target: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = DaemonCliArgs::parse();

    // Initialize structured logging
    tracing_subscriber::fmt::init();

    println!("===============================================================");
    println!("🚀 BourbakiMesh Standalone P2P Prover Daemon");
    println!("===============================================================");
    println!("  Peer Port:       {}", args.peer_port);
    println!("  Model Path:      {:?}", args.model_path);
    println!("  Simulations:     {}", args.simulations);
    println!("  IPC Bridge:      {}", args.ipc_addr);
    println!("  Lean 4 Target:   {:?}", args.lean_target);
    println!("  Bootstrap Nodes: {:?}", args.bootstrap_nodes);

    // 1. Initialize ProofLedger with Genesis Block
    let ledger = Arc::new(RwLock::new(ProofLedger::new()));
    let genesis = ProofBlock::genesis("True");
    ledger.write().unwrap().insert_block(genesis)?;

    // 2. Initialize LeanEnvironment if path exists
    let lean_env = if args.lean_target.exists() {
        Some(LeanEnvironment::new(&args.lean_target))
    } else {
        None
    };

    // 3. Configure P2P Network
    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", args.peer_port).parse()?;
    let bootstrap_peers = args
        .bootstrap_nodes
        .into_iter()
        .filter_map(|_addr| {
            // If multiaddr contains peer ID, extract it
            None::<(PeerId, Multiaddr)>
        })
        .collect();

    let p2p_config = P2PConfig {
        listen_addr,
        bootstrap_peers,
        heartbeat_interval: Duration::from_millis(500),
    };

    // 4. Initialize P2PNode and MeshWorkerDaemon
    let node = P2PNode::from_ledger(p2p_config, ledger.clone(), lean_env)?;
    let local_peer_id = node.local_peer_id();
    println!("  Local Peer ID:   {}", local_peer_id);
    println!("===============================================================");
    println!("🟢 Node joined swarm. Listening for task obligations...");

    let mut daemon = MeshWorkerDaemon::new(node, args.simulations)
        .with_model_path(args.model_path.to_string_lossy().to_string());

    // 5. Main event loop with graceful shutdown
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\n🛑 Interrupted by user. Gracefully shutting down BourbakiMesh daemon...");
        }
        _ = async {
            loop {
                match daemon.step().await {
                    Ok(Some(WorkerDaemonEvent::TaskClaimed { task_id, theorem_name, goal_statement })) => {
                        println!("📥 [CLAIM] Task {}: {} ⊢ {}", task_id, theorem_name, goal_statement);
                    }
                    Ok(Some(WorkerDaemonEvent::ProofPublished { task_id, block_id, theorem_name })) => {
                        println!("✨ [PROVEN] Task {}: {} certified → BlockId({})", task_id, theorem_name, &block_id.to_hex()[..12]);
                    }
                    Ok(Some(WorkerDaemonEvent::ProofReceived { block_id, prover })) => {
                        println!("📦 [ATTESTED] Received ProofBlock({}) from peer {}", &block_id.to_hex()[..12], prover);
                    }
                    Ok(Some(WorkerDaemonEvent::PeerConnected(peer_id))) => {
                        println!("🤝 [PEER] Connected to peer {}", peer_id);
                    }
                    Ok(Some(WorkerDaemonEvent::PeerDisconnected(peer_id))) => {
                        println!("👋 [PEER] Disconnected from peer {}", peer_id);
                    }
                    Ok(Some(WorkerDaemonEvent::PeerSubscribed { peer_id, topic })) => {
                        println!("📢 [SUB] Peer {} subscribed to {}", peer_id, topic);
                    }
                    Ok(Some(WorkerDaemonEvent::ChunkReceived { chunk, sender })) => {
                        println!("🧩 [CHUNK] Received chunk {}/{} for {} from peer {}", chunk.chunk_index + 1, chunk.total_chunks, chunk.model_name, sender);
                    }
                    Ok(Some(WorkerDaemonEvent::ModelUpgradeAnnounced { version, root_hash, total_chunks })) => {
                        println!("🚀 [MODEL UPGRADE] Swarm announced model {} (Merkle root: {}, total chunks: {})", version, &root_hash.to_hex()[..12], total_chunks);
                    }
                    Ok(Some(WorkerDaemonEvent::ModelHotReloaded { version, root_hash, total_bytes })) => {
                        println!("🎉 [HOT-RELOAD] Successfully hot-reloaded model {} ({} bytes, Merkle root: {}) into memory!", version, total_bytes, &root_hash.to_hex()[..12]);
                    }
                    Ok(None) => {}
                    Err(err) => {
                        eprintln!("⚠️  [P2P ERROR] {}", err);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        } => {}
    }

    Ok(())
}
