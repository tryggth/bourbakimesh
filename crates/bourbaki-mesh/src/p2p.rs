//! Decentralized Peer-to-Peer (P2P) proof search protocol using libp2p GossipSub and Kademlia DHT.

use crate::block::{BlockId, ProofBlock};
pub use crate::chunks::{ChunkGossipMessage, ModelChunk, TOPIC_CHUNKS};
use crate::consensus::{AttestationError, ProofAttestationEngine};
use crate::dag::ProofLedger;
use futures::StreamExt;
use libp2p::gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode};
use libp2p::identify;
use libp2p::kad::{self, store::MemoryStore};
use libp2p::ping;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{identity, Multiaddr, PeerId, Swarm};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

/// GossipSub topic for open theorem obligations and search tasks.
pub const TOPIC_TASKS: &str = "/bourbaki/1.0.0/tasks";

/// GossipSub topic for candidate ProofBlock strategy DAG broadcasts.
pub const TOPIC_PROOFS: &str = "/bourbaki/1.0.0/proofs";

/// Errors occurring during P2P network operations.
#[derive(Debug, Error)]
pub enum P2PError {
    #[error("Failed to generate identity or initialize transport: {0}")]
    Init(String),

    #[error("GossipSub subscription error: {0}")]
    GossipsubSubscription(#[from] gossipsub::SubscriptionError),

    #[error("GossipSub publish error: {0}")]
    GossipsubPublish(#[from] gossipsub::PublishError),

    #[error("Multiaddr parse or dial error: {0}")]
    Dial(#[from] libp2p::swarm::DialError),

    #[error("Transport error: {0}")]
    Transport(#[from] libp2p::TransportError<std::io::Error>),

    #[error("Serialization / Deserialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Attestation / Consensus error: {0}")]
    Attestation(#[from] AttestationError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Broadcast message payload for open proof search tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskGossipMessage {
    pub task_id: Uuid,
    pub theorem_name: String,
    pub goal_statement: String,
    pub difficulty_tier: u8,
    pub bounty_points: u64,
    pub proposer_peer_id: String,
    pub timestamp_secs: u64,
}

/// Broadcast message payload for candidate ProofBlocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofGossipMessage {
    pub block: ProofBlock,
    pub prover_peer_id: String,
    pub signature: Option<String>,
}

/// Unified network behaviour combining GossipSub, Kademlia DHT, Identify, and Ping.
#[derive(NetworkBehaviour)]
pub struct BourbakiBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
}

/// Configuration parameters for a P2P node.
#[derive(Debug, Clone)]
pub struct P2PConfig {
    pub listen_addr: Multiaddr,
    pub bootstrap_peers: Vec<(PeerId, Multiaddr)>,
    pub heartbeat_interval: Duration,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            listen_addr: "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
            bootstrap_peers: Vec::new(),
            heartbeat_interval: Duration::from_secs(1),
        }
    }
}

/// P2P Event notification emitted by the node loop.
#[derive(Debug, Clone)]
pub enum P2PEvent {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    PeerSubscribed { peer_id: PeerId, topic: String },
    TaskReceived(TaskGossipMessage),
    ProofReceived { block_id: BlockId, prover: String },
    ProofRejected { reason: String },
    ChunkReceived { chunk: ModelChunk, sender: String },
}

/// Decentralized P2P node participating in the Bourbaki proof discovery swarm.
pub struct P2PNode {
    pub local_peer_id: PeerId,
    pub swarm: Swarm<BourbakiBehaviour>,
    pub attestation_engine: Arc<ProofAttestationEngine>,
    pub tasks_topic: IdentTopic,
    pub proofs_topic: IdentTopic,
    pub chunks_topic: IdentTopic,
}

impl P2PNode {
    /// Initialize a new P2PNode with the specified configuration and attestation engine.
    pub fn new(
        config: P2PConfig,
        attestation_engine: Arc<ProofAttestationEngine>,
    ) -> Result<Self, P2PError> {
        let id_keys = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(id_keys.public());

        // 1. Configure GossipSub
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(config.heartbeat_interval)
            .validation_mode(ValidationMode::Permissive)
            .mesh_n(1)
            .mesh_n_low(1)
            .mesh_n_high(8)
            .mesh_outbound_min(0)
            .message_id_fn(message_id_fn)
            .build()
            .map_err(|e| P2PError::Init(e.to_string()))?;

        let mut gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(id_keys.clone()),
            gossipsub_config,
        )
        .map_err(|e| P2PError::Init(e.to_string()))?;

        let tasks_topic = IdentTopic::new(TOPIC_TASKS);
        let proofs_topic = IdentTopic::new(TOPIC_PROOFS);
        let chunks_topic = IdentTopic::new(TOPIC_CHUNKS);
        gossipsub.subscribe(&tasks_topic)?;
        gossipsub.subscribe(&proofs_topic)?;
        gossipsub.subscribe(&chunks_topic)?;

        // 2. Configure Kademlia DHT
        let store = MemoryStore::new(local_peer_id);
        let kad_config = kad::Config::default();
        let mut kademlia = kad::Behaviour::with_config(local_peer_id, store, kad_config);
        kademlia.set_mode(Some(kad::Mode::Server));

        // 3. Configure Identify
        let identify = identify::Behaviour::new(identify::Config::new(
            "/bourbaki/1.0.0".into(),
            id_keys.public(),
        ));

        // 4. Configure Ping
        let ping = ping::Behaviour::new(ping::Config::default());

        let behaviour = BourbakiBehaviour {
            gossipsub,
            kademlia,
            identify,
            ping,
        };

        // 5. Build Swarm with TCP + Noise + Yamux
        let swarm = libp2p::SwarmBuilder::with_existing_identity(id_keys)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|e| P2PError::Init(e.to_string()))?
            .with_behaviour(|_| behaviour)
            .map_err(|e| P2PError::Init(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        let mut node = Self {
            local_peer_id,
            swarm,
            attestation_engine,
            tasks_topic,
            proofs_topic,
            chunks_topic,
        };

        node.swarm.listen_on(config.listen_addr)?;

        for (peer_id, addr) in config.bootstrap_peers {
            node.swarm
                .behaviour_mut()
                .kademlia
                .add_address(&peer_id, addr);
        }

        Ok(node)
    }

    /// Convenience constructor from raw ledger and optional LeanEnvironment.
    pub fn from_ledger(
        config: P2PConfig,
        ledger: Arc<RwLock<ProofLedger>>,
        lean_env: Option<bourbaki_kernel::verifier::LeanEnvironment>,
    ) -> Result<Self, P2PError> {
        let engine = Arc::new(ProofAttestationEngine::new(ledger, lean_env));
        Self::new(config, engine)
    }

    /// Return the local node's PeerId.
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Return reference to the local ProofLedger.
    pub fn ledger(&self) -> Arc<RwLock<ProofLedger>> {
        self.attestation_engine.ledger()
    }

    /// List active listener addresses.
    pub fn listen_addrs(&self) -> Vec<Multiaddr> {
        self.swarm.listeners().cloned().collect()
    }

    /// Dial another peer via Multiaddr.
    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), P2PError> {
        self.swarm.dial(addr)?;
        Ok(())
    }

    /// Broadcast an open theorem task obligation to the swarm.
    pub fn broadcast_task(
        &mut self,
        theorem_name: &str,
        goal_statement: &str,
        difficulty_tier: u8,
        bounty_points: u64,
    ) -> Result<Uuid, P2PError> {
        let task_id = Uuid::new_v4();
        let msg = TaskGossipMessage {
            task_id,
            theorem_name: theorem_name.to_string(),
            goal_statement: goal_statement.to_string(),
            difficulty_tier,
            bounty_points,
            proposer_peer_id: self.local_peer_id.to_string(),
            timestamp_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let encoded = serde_json::to_vec(&msg)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.tasks_topic.clone(), encoded)?;
        Ok(task_id)
    }

    /// Broadcast a candidate ProofBlock to the swarm.
    pub fn broadcast_proof(&mut self, block: ProofBlock) -> Result<BlockId, P2PError> {
        let block_id = block.id;
        let msg = ProofGossipMessage {
            block,
            prover_peer_id: self.local_peer_id.to_string(),
            signature: None,
        };

        let encoded = serde_json::to_vec(&msg)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.proofs_topic.clone(), encoded)?;
        Ok(block_id)
    }

    /// Broadcast a ModelChunk to the swarm.
    pub fn broadcast_chunk(&mut self, chunk: ModelChunk) -> Result<[u8; 32], P2PError> {
        let chunk_hash = chunk.chunk_hash;
        let msg = ChunkGossipMessage {
            chunk,
            sender_peer_id: self.local_peer_id.to_string(),
        };

        let encoded = serde_json::to_vec(&msg)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.chunks_topic.clone(), encoded)?;
        Ok(chunk_hash)
    }

    /// Step the swarm event loop and handle incoming GossipSub/Kademlia/Ping messages.
    pub async fn step(&mut self) -> Result<Option<P2PEvent>, P2PError> {
        tokio::select! {
            event = self.swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        tracing::debug!("Node listening on {:?}", address);
                        Ok(None)
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        self.swarm.behaviour_mut().kademlia.add_address(&peer_id, "/ip4/127.0.0.1/tcp/0".parse().unwrap());
                        Ok(Some(P2PEvent::PeerConnected(peer_id)))
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        Ok(Some(P2PEvent::PeerDisconnected(peer_id)))
                    }
                    SwarmEvent::Behaviour(BourbakiBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed {
                        peer_id,
                        topic,
                    })) => {
                        Ok(Some(P2PEvent::PeerSubscribed {
                            peer_id,
                            topic: topic.to_string(),
                        }))
                    }
                    SwarmEvent::Behaviour(BourbakiBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        message,
                        ..
                    })) => {
                        if message.topic == self.tasks_topic.hash() {
                            if let Ok(task_msg) = serde_json::from_slice::<TaskGossipMessage>(&message.data) {
                                return Ok(Some(P2PEvent::TaskReceived(task_msg)));
                            }
                        } else if message.topic == self.proofs_topic.hash() {
                            if let Ok(proof_msg) = serde_json::from_slice::<ProofGossipMessage>(&message.data) {
                                match self.attestation_engine.verify_and_commit(proof_msg.block) {
                                    Ok(block_id) => {
                                        return Ok(Some(P2PEvent::ProofReceived {
                                            block_id,
                                            prover: proof_msg.prover_peer_id,
                                        }));
                                    }
                                    Err(err) => {
                                        return Ok(Some(P2PEvent::ProofRejected {
                                            reason: err.to_string(),
                                        }));
                                    }
                                }
                            }
                        } else if message.topic == self.chunks_topic.hash() {
                            if let Ok(chunk_msg) = serde_json::from_slice::<ChunkGossipMessage>(&message.data) {
                                if chunk_msg.chunk.verify_hash() {
                                    return Ok(Some(P2PEvent::ChunkReceived {
                                        chunk: chunk_msg.chunk,
                                        sender: chunk_msg.sender_peer_id,
                                    }));
                                }
                            }
                        }
                        Ok(None)
                    }
                    SwarmEvent::Behaviour(BourbakiBehaviourEvent::Identify(identify::Event::Received {
                        peer_id,
                        info,
                    })) => {
                        for addr in info.listen_addrs {
                            self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                        }
                        Ok(None)
                    }
                    _ => Ok(None),
                }
            }
        }
    }
}
