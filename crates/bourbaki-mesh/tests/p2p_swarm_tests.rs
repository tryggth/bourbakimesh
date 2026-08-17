//! P2P swarm discovery, gossipsub broadcast, and Byzantine attestation integration tests.

use bourbaki_kernel::ast::Term;
use bourbaki_mesh::block::ProofBlock;
use bourbaki_mesh::consensus::ProofAttestationEngine;
use bourbaki_mesh::dag::ProofLedger;
use bourbaki_mesh::p2p::{P2PConfig, P2PEvent, P2PNode, TOPIC_PROOFS};
use futures::StreamExt;
use libp2p::Multiaddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::time::timeout;

/// Helper function to create an active node listening on loopback TCP with an ephemeral port.
async fn create_test_node() -> (P2PNode, Multiaddr) {
    let ledger = Arc::new(RwLock::new(ProofLedger::new()));
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

    (node, listen_addr)
}

#[tokio::test]
async fn test_p2p_swarm_discovery_and_ping() {
    let (mut node_a, addr_a) = create_test_node().await;
    let (mut node_b, _addr_b) = create_test_node().await;

    let peer_a_id = node_a.local_peer_id();
    let peer_b_id = node_b.local_peer_id();

    // Node B dials Node A
    node_b.dial(addr_a).expect("Node B dial failed");

    // Pump events on both nodes until connected
    let connected = timeout(Duration::from_secs(10), async {
        let mut a_saw_b = false;
        let mut b_saw_a = false;

        while !a_saw_b || !b_saw_a {
            tokio::select! {
                ev_a = node_a.step() => {
                    if let Ok(Some(P2PEvent::PeerConnected(p))) = ev_a {
                        if p == peer_b_id {
                            a_saw_b = true;
                        }
                    }
                }
                ev_b = node_b.step() => {
                    if let Ok(Some(P2PEvent::PeerConnected(p))) = ev_b {
                        if p == peer_a_id {
                            b_saw_a = true;
                        }
                    }
                }
            }
        }
    })
    .await;

    assert!(
        connected.is_ok(),
        "Node A and Node B failed to discover each other over libp2p"
    );
}

#[tokio::test]
async fn test_proof_block_gossip_and_attestation() {
    let (mut node_a, addr_a) = create_test_node().await;
    let (mut node_b, _addr_b) = create_test_node().await;

    // Insert genesis block into both ledgers
    let genesis = ProofBlock::genesis("True");
    node_a
        .ledger()
        .write()
        .unwrap()
        .insert_block(genesis.clone())
        .unwrap();
    node_b
        .ledger()
        .write()
        .unwrap()
        .insert_block(genesis.clone())
        .unwrap();

    node_b.dial(addr_a).expect("Dial failed");

    // Wait for connection and subscription sync
    let peer_b_id = node_b.local_peer_id();
    let _ = timeout(Duration::from_secs(5), async {
        loop {
            tokio::select! {
                ev_a = node_a.step() => {
                    if let Ok(Some(P2PEvent::PeerSubscribed { peer_id, topic })) = ev_a {
                        if peer_id == peer_b_id && topic == TOPIC_PROOFS {
                            break;
                        }
                    }
                }
                _ = node_b.step() => {}
            }
        }
    })
    .await;

    // Additional small pump to stabilize mesh
    for _ in 0..10 {
        let _ = tokio::time::timeout(Duration::from_millis(20), node_a.step()).await;
        let _ = tokio::time::timeout(Duration::from_millis(20), node_b.step()).await;
    }

    // Construct valid proof block for A -> A
    let identity_term = Term::lam("x", Term::var("A"), Term::var("x"));
    let valid_block = ProofBlock::new(
        vec![genesis.id],
        "Mathlib.Logic.Basic.id".into(),
        "∀ (A : Prop), A → A".into(),
        None,
        Some(identity_term),
        true,
        1700000001,
    );
    let expected_id = valid_block.id;

    // Broadcast proof block from Node A
    node_a
        .broadcast_proof(valid_block)
        .expect("Broadcast proof failed");

    // Pump events until Node B receives and attests the block
    let proof_received = timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                _ = node_a.step() => {}
                ev_b = node_b.step() => {
                    if let Ok(Some(P2PEvent::ProofReceived { block_id, .. })) = ev_b {
                        if block_id == expected_id {
                            return true;
                        }
                    }
                }
            }
        }
    })
    .await;

    assert!(
        proof_received.is_ok(),
        "Node B failed to receive and attest valid proof block"
    );

    // Verify Node B's local ledger contains the verified block
    assert!(
        node_b.ledger().read().unwrap().contains_block(&expected_id),
        "Block was not committed to Node B ledger"
    );
}

#[tokio::test]
async fn test_adversarial_byzantine_proof_rejection() {
    let (mut node_a, addr_a) = create_test_node().await;
    let (mut node_b, _addr_b) = create_test_node().await;

    let genesis = ProofBlock::genesis("True");
    node_a
        .ledger()
        .write()
        .unwrap()
        .insert_block(genesis.clone())
        .unwrap();
    node_b
        .ledger()
        .write()
        .unwrap()
        .insert_block(genesis.clone())
        .unwrap();

    node_b.dial(addr_a).expect("Dial failed");

    // Wait for connection and subscription sync
    let peer_b_id = node_b.local_peer_id();
    let _ = timeout(Duration::from_secs(5), async {
        loop {
            tokio::select! {
                ev_a = node_a.step() => {
                    if let Ok(Some(P2PEvent::PeerSubscribed { peer_id, topic })) = ev_a {
                        if peer_id == peer_b_id && topic == TOPIC_PROOFS {
                            break;
                        }
                    }
                }
                _ = node_b.step() => {}
            }
        }
    })
    .await;

    for _ in 0..10 {
        let _ = tokio::time::timeout(Duration::from_millis(20), node_a.step()).await;
        let _ = tokio::time::timeout(Duration::from_millis(20), node_b.step()).await;
    }

    // Construct forged adversarial proof block targeting False
    let fake_term = Term::var("magic_axiom");
    let forged_block = ProofBlock::new(
        vec![genesis.id],
        "Bourbaki.FalseProof".into(),
        "False".into(),
        None,
        Some(fake_term),
        true,
        1700000002,
    );
    let forged_id = forged_block.id;

    // Node A broadcasts forged block
    node_a
        .broadcast_proof(forged_block)
        .expect("Broadcast forged proof failed");

    // Node B must reject the proof during attestation
    let proof_rejected = timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                _ = node_a.step() => {}
                ev_b = node_b.step() => {
                    if let Ok(Some(P2PEvent::ProofRejected { reason })) = ev_b {
                        return reason;
                    }
                }
            }
        }
    })
    .await;

    assert!(
        proof_rejected.is_ok(),
        "Node B did not reject the forged Byzantine proof block"
    );
    let reason = proof_rejected.unwrap();
    assert!(
        reason.contains("Byzantine attack detected") || reason.contains("False"),
        "Unexpected rejection reason: {}",
        reason
    );

    // Ensure Node B ledger does NOT contain the forged block
    assert!(
        !node_b.ledger().read().unwrap().contains_block(&forged_id),
        "Forged block was wrongfully committed to Node B ledger"
    );
}
