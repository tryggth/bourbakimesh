//! Integration tests for standalone MeshWorkerDaemon loopback claiming, proving, and gossip attestation.

use bourbaki_kernel::ast::Term;
use bourbaki_mesh::block::ProofBlock;
use bourbaki_mesh::consensus::ProofAttestationEngine;
use bourbaki_mesh::dag::ProofLedger;
use bourbaki_mesh::p2p::{P2PConfig, P2PNode, TOPIC_TASKS};
use bourbaki_mesh::worker::{MeshWorkerDaemon, WorkerDaemonEvent};
use futures::StreamExt;
use libp2p::Multiaddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::time::timeout;

/// Helper function to create an active worker daemon listening on loopback TCP.
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

    let daemon = MeshWorkerDaemon::new(node, 100);
    (daemon, listen_addr)
}

#[tokio::test]
async fn test_daemon_solve_goal_logic() {
    let (daemon, _addr) = create_test_daemon().await;

    // 1. Solve True
    let (strat_true, term_true) = daemon.solve_goal("thm_true", "True");
    assert!(strat_true.root.is_some());
    assert_eq!(term_true, Term::var("True.intro"));

    // 2. Solve Implication P -> P
    let (strat_imp, term_imp) = daemon.solve_goal("thm_id", "P -> P");
    assert!(strat_imp.root.is_some());
    assert!(matches!(term_imp, Term::Lam { .. }));
}

#[tokio::test]
async fn test_daemon_loopback_task_claiming_and_proof_gossip() {
    let (mut daemon_a, addr_a) = create_test_daemon().await;
    let (mut daemon_b, _addr_b) = create_test_daemon().await;

    let peer_a_id = daemon_a.local_peer_id();
    let peer_b_id = daemon_b.local_peer_id();

    // Node B dials Node A
    daemon_b.node_mut().dial(addr_a).expect("Dial failed");

    // Connect peers and exchange subscriptions
    let connected = timeout(Duration::from_secs(10), async {
        let mut a_saw_b = false;
        let mut b_saw_a = false;
        let mut a_saw_b_sub_tasks = false;
        let mut b_saw_a_sub_tasks = false;

        while !a_saw_b || !b_saw_a || !a_saw_b_sub_tasks || !b_saw_a_sub_tasks {
            tokio::select! {
                ev_a = daemon_a.step() => {
                    match ev_a {
                        Ok(Some(WorkerDaemonEvent::PeerConnected(p))) if p == peer_b_id => a_saw_b = true,
                        Ok(Some(WorkerDaemonEvent::PeerSubscribed { peer_id, topic })) if peer_id == peer_b_id && topic == TOPIC_TASKS => a_saw_b_sub_tasks = true,
                        _ => {}
                    }
                }
                ev_b = daemon_b.step() => {
                    match ev_b {
                        Ok(Some(WorkerDaemonEvent::PeerConnected(p))) if p == peer_a_id => b_saw_a = true,
                        Ok(Some(WorkerDaemonEvent::PeerSubscribed { peer_id, topic })) if peer_id == peer_a_id && topic == TOPIC_TASKS => b_saw_a_sub_tasks = true,
                        _ => {}
                    }
                }
            }
        }
    })
    .await;

    assert!(connected.is_ok(), "Daemon A and Daemon B connection / subscription timed out");

    // Wait for GossipSub mesh stabilization
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Daemon A broadcasts a theorem task obligation
    let task_id = daemon_a
        .node_mut()
        .broadcast_task("thm_identity", "P -> P", 1, 500)
        .expect("Broadcast task failed");

    // Daemon B should receive the task, claim it, solve it, and gossip back a ProofBlock to Daemon A
    let proof_exchanged = timeout(Duration::from_secs(10), async {
        let mut b_published_proof = false;
        let mut a_received_proof = false;

        while !b_published_proof || !a_received_proof {
            tokio::select! {
                ev_a = daemon_a.step() => {
                    if let Ok(Some(WorkerDaemonEvent::ProofReceived { .. })) = ev_a {
                        a_received_proof = true;
                    }
                }
                ev_b = daemon_b.step() => {
                    if let Ok(Some(WorkerDaemonEvent::ProofPublished { task_id: tid, .. })) = ev_b {
                        if tid == task_id {
                            b_published_proof = true;
                        }
                    }
                }
            }
        }
    })
    .await;

    assert!(
        proof_exchanged.is_ok(),
        "Failed to claim, solve, and gossip proof block across daemons"
    );

    // Verify both ledgers now contain the new proof block for thm_identity
    let ledger_a = daemon_a.ledger();
    let ledger_b = daemon_b.ledger();

    let a_guard = ledger_a.read().unwrap();
    let b_guard = ledger_b.read().unwrap();

    assert_eq!(a_guard.len(), 2, "Ledger A should contain Genesis + Proven Block");
    assert_eq!(b_guard.len(), 2, "Ledger B should contain Genesis + Proven Block");

    let proven_block_b = b_guard
        .get_block_ids()
        .into_iter()
        .find(|id| id != &ProofBlock::genesis("True").id)
        .expect("Missing proven block in B");

    let block_data_a = a_guard.get_block(&proven_block_b).expect("Missing block in A");
    assert_eq!(block_data_a.theorem_name, "thm_identity");
    assert_eq!(block_data_a.statement, "P -> P");
    assert!(block_data_a.certified);
}
