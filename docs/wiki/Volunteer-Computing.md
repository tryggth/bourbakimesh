# In-Browser Volunteer Computing & WebGPU Prover Architecture

BourbakiMesh features a decentralized, zero-install volunteer computing engine enabling web users to donate idle GPU and CPU cycles to formal mathematics directly from any modern web browser.

---

## 1. Architectural Overview

```
+-----------------------------------------------------------------------------------+
| Browser Main Thread (ui/src/)                                                     |
|                                                                                   |
|  [VolunteerPanel.tsx] <-----> [cryptoIdentity.ts]                                 |
|         |                           |                                             |
|         | (PostMessage)             | (ECDSA P-256 Keypair)                       |
|         v                           v                                             |
|  [prover.worker.ts] --------> [signProof()]                                       |
|   - ONNX WebGPU / Wasm SIMD         |                                             |
|   - BrowserLatentMCTS (10-500 sims) |                                             |
|   - IndexedDB Weight Cache          v                                             |
+------------------------------[Signed ProofBlock Payload]--------------------------+
                                      |
                                      | WebSocket (/ws/telemetry)
                                      v
+-----------------------------------------------------------------------------------+
| BourbakiMesh Gateway Bridge (src/bourbakimesh/api/server.py)                      |
|                                                                                   |
|  1. Worker Registration (Capabilities & Hardware)                                 |
|  2. Atomic Obligation / Task Claiming                                             |
|  3. Cryptographic Signature Validation                                            |
|  4. Zero-Trust Verification (Lean 4 / CIC Kernel Gate)                            |
|  5. ProofLedger DAG Commit & Real-Time Swarm Broadcast                            |
+-----------------------------------------------------------------------------------+
```

---

## 2. In-Browser Execution Engine (`prover.worker.ts`)

- **Dedicated Web Worker Isolation:**
  MCTS tree searches execute inside a background `Worker` to maintain a silky 60 FPS UI on the main thread.
- **Hardware Acceleration:**
  - **Primary Provider:** ONNX Runtime `webgpu` for direct GPU matrix operations.
  - **Fallback Provider:** WebAssembly SIMD (`wasm`) with multi-threading where WebGPU is unavailable.
- **IndexedDB Weight Caching (`idb-keyval`):**
  Model weights (`.onnx`) are cached locally in the browser's IndexedDB. Subsequent page loads initialize the neural sessions instantly without re-downloading multi-megabyte checkpoints.

---

## 3. Cryptographic Node Identity (`cryptoIdentity.ts`)

Every browser session operates as an attested node in the proof mesh:
1. **Ephemeral Key Generation:**
   On startup, the browser generates an ECDSA keypair using the W3C Web Crypto standard:
   ```typescript
   crypto.subtle.generateKey(
     { name: "ECDSA", namedCurve: "P-256" },
     false, // Non-extractable private key
     ["sign", "verify"]
   )
   ```
2. **Deterministic Peer ID:**
   A unique, content-addressed `peer_id` is derived from the SHA-256 hash of the exported public key:
   `browser-12D3KooW...`
3. **Signed Proof Attestation:**
   When a winning strategy tree $\sigma$ is discovered, the worker extracts the CIC proof term $\mathcal{E}(\sigma)$, hashes the theorem statement and term, and signs the digest:
   ```typescript
   const signature = await signProof({
     theorem_name: "Mathlib.Logic.Identity",
     proposition: "P -> P",
     extracted_term: "fun (p : P) => p",
     prover_peer_id: identity.peerId,
   });
   ```

---

## 4. Resource Throttling & Hardware Safety

To respect user hardware and mobile battery constraints, the volunteer engine implements multi-level safeguards:

| Power Profile | MCTS Simulations / Move | Use Case |
| :--- | :---: | :--- |
| **Eco** | 25 sims | Low-power mobile devices, laptops on battery, background tabs. |
| **Balanced** | 100 sims | Standard desktop browsing. |
| **Max** | 250+ sims | Dedicated workstation proving or idle charging mode. |

### Battery & Thermal Auto-Pause
- **Battery API Hook (`navigator.getBattery`):** Automatically pauses search if the device is discharging and battery drops below 20%.
- **Visibility API Hook (`document.visibilityState`):** Automatically throttles or pauses worker execution when the tab is backgrounded (with an explicit override toggle).

---

## 5. Gateway Bridge & Verification Loop

1. **Worker Registration:** Browser workers send `{ "type": "register_worker", "peer_id": "...", "capabilities": {...} }` over `/ws/telemetry`.
2. **Task Claiming:** Workers request obligations via `{ "type": "claim_task", "peer_id": "..." }`. The gateway atomically reserves open subgoals.
3. **Proof Submission & Verification:** Candidate proof blocks sent via `{ "type": "submit_proof", ... }` are verified in the zero-trust `LeanEnvironment`.
4. **DAG Commit:** Certified blocks are committed to the topological `ProofLedger` and broadcasted in real time to all connected dashboards.
