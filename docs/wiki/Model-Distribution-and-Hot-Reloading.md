# Decentralized Model Distribution & Swarm Hot-Reloading

BourbakiMesh implements decentralized peer-to-peer neural model distribution and zero-downtime hot-reloading across edge daemon nodes and browser workers.

---

## 1. Distribution & Hot-Reload Architecture

```
[Training Pipeline / Checkpoint Promoter]
                   │
                   ▼
  1. Export ONNX & PyTorch Checkpoints
  2. Compute SHA-256 Merkle Root & Chunks (ModelChunker)
                   │
                   ▼
[P2P GossipSub Topic: /bourbaki/1.0.0/models]
                   │
    ┌──────────────┴──────────────┐
    │                             │
    ▼                             ▼
[Standalone Rust Daemon]      [FastAPI Telemetry Hub]
 - Pull missing chunks         - Broadcast "model_upgraded"
 - Verify Merkle tree          - Push event to WebSockets
 - Hot-swap in-memory session  - Invalidate browser cache
    │                             │
    ▼                             ▼
[Embedded MCTS Worker]        [In-Browser Web Worker]
 - Zero downtime switch        - OnnxRuntime session recreate
 - Continuous rollouts         - idb-keyval update
```

---

## 2. GossipSub Protocol (`/bourbaki/1.0.0/models`)

When a newly trained checkpoint passes tournament champion gating (e.g. `bourbaki_v2.pt`), the model announcement is broadcast across the P2P swarm:

```rust
pub struct ModelAnnouncementMessage {
    pub model_name: String,
    pub version: u64,
    pub total_size_bytes: usize,
    pub chunk_size_bytes: usize,
    pub total_chunks: usize,
    pub merkle_root: [u8; 32],
    pub elo_rating: f64,
}
```

### Merkle Chunking & Direct Swarm Pulling
1. **Chunking (`ModelChunker`):** Weights are sliced into deterministic $512\text{ KB}$ chunks (`ModelChunk`).
2. **Content Verification:** Each chunk's hash is verified against the Merkle root before acceptance.
3. **P2P Streaming:** Missing chunks are pulled in parallel from connected Kademlia DHT peers.

---

## 3. Zero-Downtime Daemon Hot-Reloading (`MeshWorkerDaemon`)

In `crates/bourbaki-mesh`, `MeshWorkerDaemon` implements dynamic model session replacement:

```rust
pub fn hot_reload_model(&mut self, model_name: &str, model_bytes: &[u8]) -> Result<(), String> {
    // 1. Instantiate candidate neural runtime from in-memory buffer
    let new_session = NeuralEvaluator::load_from_bytes(model_bytes)?;
    
    // 2. Atomically swap active evaluator
    self.evaluator = Arc::new(new_session);
    self.active_model_name = model_name.to_string();
    
    // 3. Emit ModelHotReloaded telemetry event
    println!("🔥 Zero-downtime hot-reload completed: {}", model_name);
    Ok(())
}
```

Active MCTS searches complete without process restarts, memory leaks, or dropped RPC connections.

---

## 4. In-Browser Web Worker Hot-Swapping (`prover.worker.ts`)

When a `model_upgraded` or `swarm_sync` event is received by the frontend:
1. The main thread sends an `UPGRADE_MODEL` message to `prover.worker.ts` with the new URL / buffer.
2. The worker invalidates the existing IndexedDB cache entry (`idb-keyval`).
3. The ONNX Runtime session is cleanly released and re-initialized with the new weights.
4. Subsequent MCTS rollouts immediately evaluate against the upgraded policy and value heads.
