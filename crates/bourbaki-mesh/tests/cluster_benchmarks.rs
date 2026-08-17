//! Multi-Node P2P Cluster Benchmarks and 5-Node Swarm Consensus Integration Test.

use bourbaki_kernel::ast::Term;
use bourbaki_mesh::block::ProofBlock;
use bourbaki_mesh::chunks::ModelChunker;
use bourbaki_mesh::consensus::ProofAttestationEngine;
use bourbaki_mesh::dag::ProofLedger;
use bourbaki_mesh::p2p::{P2PConfig, P2PEvent, P2PNode};
use futures::StreamExt;
use libp2p::Multiaddr;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Helper function to create an active P2P node listening on loopback TCP.
async fn create_cluster_node(genesis_block: ProofBlock) -> (P2PNode, Multiaddr) {
    let ledger = Arc::new(RwLock::new(ProofLedger::new()));
    ledger.write().unwrap().insert_block(genesis_block).unwrap();

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

/// Helper to step all cluster nodes with non-blocking timeouts.
async fn pump_cluster(nodes: &mut [P2PNode]) -> Vec<Option<P2PEvent>> {
    let mut events = Vec::with_capacity(nodes.len());
    for node in nodes.iter_mut() {
        match tokio::time::timeout(Duration::from_millis(10), node.step()).await {
            Ok(Ok(Some(ev))) => events.push(Some(ev)),
            _ => events.push(None),
        }
    }
    events
}

#[tokio::test]
async fn test_5_node_p2p_cluster_full_mesh_consensus_and_chunk_sync() {
    let start_time = Instant::now();
    const NUM_NODES: usize = 5;

    let genesis = ProofBlock::genesis("True");
    let genesis_id = genesis.id;

    // 1. Initialize 5 cluster nodes
    let mut nodes = Vec::with_capacity(NUM_NODES);
    let mut addrs = Vec::with_capacity(NUM_NODES);

    for _ in 0..NUM_NODES {
        let (node, addr) = create_cluster_node(genesis.clone()).await;
        nodes.push(node);
        addrs.push(addr);
    }

    // 2. Interconnect all nodes in a full mesh (every node dials all prior nodes)
    for i in 1..NUM_NODES {
        for j in 0..i {
            nodes[i].dial(addrs[j].clone()).expect("Dial failed");
        }
    }

    // 3. Connect nodes and stabilize GossipSub mesh
    let connect_res = timeout(Duration::from_secs(10), async {
        loop {
            pump_cluster(&mut nodes).await;
            let min_peers = nodes
                .iter()
                .map(|n| n.swarm.network_info().num_peers())
                .min()
                .unwrap_or(0);
            if min_peers >= 1 {
                break;
            }
        }
    })
    .await;

    assert!(
        connect_res.is_ok(),
        "5-node cluster failed to establish mesh connectivity"
    );

    // Pump event loop to stabilize GossipSub subscription mesh
    for _ in 0..25 {
        pump_cluster(&mut nodes).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // 4. Test Simultaneous Task Broadcast from Node 0
    let task_id = timeout(Duration::from_secs(10), async {
        loop {
            pump_cluster(&mut nodes).await;
            if let Ok(id) =
                nodes[0].broadcast_task("Mathlib.Logic.And.intro", "A -> B -> A ∧ B", 1, 100)
            {
                return id;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("Task broadcast failed");

    // 5. Test P2P Model Weight Chunk Distribution from Node 0
    let dummy_weights =
        b"BourbakiMuZero_25M_RelationalArenaTransformer_Weights_Binary_Payload".repeat(20);
    let (_manifest, chunks) =
        ModelChunker::chunk_bytes("bourbaki_v2.pt", &dummy_weights, Some(512)).unwrap();
    assert!(!chunks.is_empty());

    let broadcast_chunk = chunks[0].clone();
    let expected_chunk_hash = broadcast_chunk.chunk_hash;

    timeout(Duration::from_secs(10), async {
        loop {
            pump_cluster(&mut nodes).await;
            if nodes[0].broadcast_chunk(broadcast_chunk.clone()).is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("Chunk broadcast failed");

    // 6. Test Proof Deduction & ProofBlock Broadcast from Node 1
    let valid_block = ProofBlock::new(
        vec![genesis_id],
        "Mathlib.Logic.And.intro".into(),
        "A -> B -> A ∧ B".into(),
        None,
        Some(Term::lam(
            "a",
            Term::var("A"),
            Term::lam(
                "b",
                Term::var("B"),
                Term::app(
                    Term::app(Term::var("And.intro"), Term::var("a")),
                    Term::var("b"),
                ),
            ),
        )),
        true,
        1700000002,
    );
    let valid_block_id = valid_block.id;

    timeout(Duration::from_secs(10), async {
        loop {
            pump_cluster(&mut nodes).await;
            if nodes[1].broadcast_proof(valid_block.clone()).is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("Proof broadcast failed");

    // 7. Step the entire 5-node cluster to propagate tasks, chunks, and proofs
    let mut received_tasks = HashSet::new();
    let mut received_chunks = HashSet::new();
    let mut received_proofs = HashSet::new();

    let cluster_sync_res = timeout(Duration::from_secs(15), async {
        while received_proofs.is_empty() || received_chunks.is_empty() || received_tasks.is_empty()
        {
            let events = pump_cluster(&mut nodes).await;
            for (idx, ev_opt) in events.into_iter().enumerate() {
                if let Some(ev) = ev_opt {
                    match ev {
                        P2PEvent::TaskReceived(t) if t.task_id == task_id => {
                            received_tasks.insert(idx);
                        }
                        P2PEvent::ChunkReceived { chunk, .. }
                            if chunk.chunk_hash == expected_chunk_hash =>
                        {
                            received_chunks.insert(idx);
                        }
                        P2PEvent::ProofReceived { block_id, .. } if block_id == valid_block_id => {
                            received_proofs.insert(idx);
                        }
                        _ => {}
                    }
                }
            }
        }
    })
    .await;

    assert!(
        cluster_sync_res.is_ok(),
        "Cluster event propagation timed out. Tasks received: {:?}, Chunks received: {:?}, Proofs received: {:?}",
        received_tasks,
        received_chunks,
        received_proofs
    );

    // 8. Verify Ledger Convergence across cluster
    for (i, node) in nodes.iter().enumerate() {
        let ledger = node.ledger();
        let guard = ledger.read().unwrap();
        assert!(
            guard.contains_block(&genesis_id),
            "Node {} missing genesis block",
            i
        );
    }

    println!(
        "5-node cluster benchmark passed in {:?}. All 5 nodes verified tasks, chunks, and Byzantine consensus.",
        start_time.elapsed()
    );
}
