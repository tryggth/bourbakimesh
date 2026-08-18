//! Integration tests for automated P2P model weight announcement, chunk synchronization, and hot-reloading.

use bourbaki_mesh::block::ProofBlock;
use bourbaki_mesh::chunks::ModelChunker;
use bourbaki_mesh::consensus::ProofAttestationEngine;
use bourbaki_mesh::dag::ProofLedger;
use bourbaki_mesh::p2p::{P2PConfig, P2PNode};
use bourbaki_mesh::worker::{MeshWorkerDaemon, WorkerDaemonEvent};
use futures::StreamExt;
use libp2p::Multiaddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::time::timeout;

/// Helper function to create an active worker daemon node listening on loopback TCP.
async fn create_test_daemon() -> (MeshWorkerDaemon, Multiaddr) {
    let ledger = Arc::new(RwLock::new(ProofLedger::new()));
    let genesis = ProofBlock::genesis("True");
    ledger.write().unwrap().insert_block(genesis).unwrap();

    let engine = Arc::new(ProofAttestationEngine::new(ledger, None));
    let config = P2PConfig {
        listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
        bootstrap_peers: Vec::new(),
        heartbeat_interval: Duration::from_millis(50),
    };

    let mut node = P2PNode::new(config, engine).expect("Failed to create P2PNode");

    // Pump events until NewListenAddr is received
    let listen_addr = timeout(Duration::from_secs(5), async {
        loop {
            if let Some(libp2p::swarm::SwarmEvent::NewListenAddr { address, .. }) =
                node.swarm.next().await
            {
                return address;
            }
        }
    })
    .await
    .expect("Timed out waiting for listen address");

    let daemon = MeshWorkerDaemon::new(node, 50);
    (daemon, listen_addr)
}

#[tokio::test]
async fn test_automated_p2p_model_weight_sync_and_hot_reload() {
    let (mut daemon_a, addr_a) = create_test_daemon().await;
    let (mut daemon_b, _addr_b) = create_test_daemon().await;

    // Node B dials Node A
    daemon_b.node_mut().dial(addr_a).expect("Daemon B dial failed");

    // 1. Wait for connection establishment & GossipSub subscription
    let connected = timeout(Duration::from_secs(10), async {
        let mut a_subscribed = false;
        let mut b_subscribed = false;

        while !a_subscribed || !b_subscribed {
            tokio::select! {
                ev_a = daemon_a.step() => {
                    if let Ok(Some(WorkerDaemonEvent::PeerSubscribed { topic, .. })) = ev_a {
                        if topic.contains("models") || topic.contains("chunks") {
                            a_subscribed = true;
                        }
                    }
                }
                ev_b = daemon_b.step() => {
                    if let Ok(Some(WorkerDaemonEvent::PeerSubscribed { topic, .. })) = ev_b {
                        if topic.contains("models") || topic.contains("chunks") {
                            b_subscribed = true;
                        }
                    }
                }
            }
        }
    })
    .await;

    assert!(connected.is_ok(), "Timed out waiting for GossipSub peer subscription");

    // Give GossipSub mesh time to stabilize
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 2. Prepare new model weight binary (e.g. 64KB synthetic weight buffer)
    let fake_weights = (0..65536).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
    let (manifest, chunks) = ModelChunker::chunk_bytes("bourbaki_v2.1.pt", &fake_weights, Some(16 * 1024))
        .expect("Failed to chunk model data");

    assert_eq!(manifest.total_chunks, 4);
    assert!(manifest.verify_merkle_root());

    // 3. Node A announces new model version "v2.1.0-alpha"
    daemon_a
        .announce_model_update("v2.1.0-alpha", "bourbaki_v2.1.pt", &manifest)
        .expect("Failed to broadcast model announcement");

    // 4. Pump nodes until Node B receives model announcement
    let announced = timeout(Duration::from_secs(5), async {
        loop {
            tokio::select! {
                _ = daemon_a.step() => {}
                ev_b = daemon_b.step() => {
                    if let Ok(Some(WorkerDaemonEvent::ModelUpgradeAnnounced { version, root_hash, total_chunks })) = ev_b {
                        assert_eq!(version, "v2.1.0-alpha");
                        assert_eq!(root_hash, manifest.root_hash);
                        assert_eq!(total_chunks, 4);
                        return true;
                    }
                }
            }
        }
    })
    .await;

    assert!(announced.is_ok(), "Timed out waiting for ModelUpgradeAnnounced event on Node B");

    // 5. Node A broadcasts all chunks
    for chunk in chunks {
        daemon_a
            .node_mut()
            .broadcast_chunk(chunk)
            .expect("Failed to broadcast chunk");
    }

    // 6. Pump nodes until Node B receives all chunks and hot-reloads the model
    let reloaded = timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                _ = daemon_a.step() => {}
                ev_b = daemon_b.step() => {
                    if let Ok(Some(WorkerDaemonEvent::ModelHotReloaded { version, root_hash, total_bytes })) = ev_b {
                        assert_eq!(version, "v2.1.0-alpha");
                        assert_eq!(root_hash, manifest.root_hash);
                        assert_eq!(total_bytes, 65536);
                        return true;
                    }
                }
            }
        }
    })
    .await;

    assert!(reloaded.is_ok(), "Timed out waiting for ModelHotReloaded event on Node B");

    // 7. Verify Node B's active in-memory model state
    assert_eq!(daemon_b.active_model_version, "v2.1.0-alpha");
    assert_eq!(daemon_b.active_model_hash, Some(manifest.root_hash));
    assert_eq!(daemon_b.active_model_bytes.as_deref(), Some(fake_weights.as_slice()));

    // 8. Ensure Node B can execute theorem proving seamlessly after hot-reload
    let (strategy, term) = daemon_b.solve_goal("Mathlib.Logic.Identity", "P -> P");
    assert!(strategy.root.is_some());
    assert!(matches!(term, bourbaki_kernel::ast::Term::Lam { .. }));
}
