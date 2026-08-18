# Top-Level Target Theorem Injection & Swarm Focus Protocol

The Top-Level Target Theorem Injection mechanism allows human researchers, operators, and automated agents to direct the collective proving power of the distributed BourbakiMesh swarm toward specific Lean 4 theorems or open conjectures.

---

## 1. Protocol Lifecycle

```
[Operator / User]
       │
       ├─► Web UI (TargetManager.tsx Modal)
       │    OR
       └─► CLI (bourbakimesh.api.target_cli)
       │
       ▼
[FastAPI Telemetry Hub (POST /api/target/set)]
       │
       ├─► 1. Parse & validate Lean 4 syntax / proposition signature
       ├─► 2. Update active target state in app.state.active_target
       ├─► 3. Generate initial root dialogue obligations
       │
       ▼
[WebSocket Broadcast (/ws/telemetry: "swarm_target_set")]
       │
       ├─► Standalone Daemons (crates/bourbaki-mesh)
       ├─► Python Training & Search Workers (src/bourbakimesh)
       └─► In-Browser Volunteer Provers (ui/src/workers/prover.worker.ts)
```

---

## 2. REST & WebSocket API Specification

### Endpoint: `POST /api/target/set`

Sets the active swarm target theorem.

**Request Payload:**
```json
{
  "name": "Mathlib.Algebra.Group.mul_left_inv",
  "proposition": "a⁻¹ * a = 1",
  "lean_code": "theorem mul_left_inv (a : G) : a⁻¹ * a = 1 := by sorry",
  "priority": 150
}
```

**Response (`200 OK`):**
```json
{
  "status": "target_updated",
  "target": {
    "name": "Mathlib.Algebra.Group.mul_left_inv",
    "proposition": "a⁻¹ * a = 1",
    "lean_code": "theorem mul_left_inv (a : G) : a⁻¹ * a = 1 := by sorry",
    "priority": 150,
    "status": "active",
    "dedicated_sims": 0,
    "open_subgoals": 1,
    "timestamp": 1723910000.0
  }
}
```

### Endpoint: `GET /api/target/current`

Retrieves the currently active swarm objective.

### WebSocket Broadcast Event: `swarm_target_set`

When a new target is set, the gateway immediately pushes the following event to all active WebSocket connections:

```json
{
  "type": "swarm_target_set",
  "timestamp": 1723910000.0,
  "data": {
    "name": "Mathlib.Algebra.Group.mul_left_inv",
    "proposition": "a⁻¹ * a = 1",
    "lean_code": "theorem mul_left_inv (a : G) : a⁻¹ * a = 1 := by sorry",
    "priority": 150,
    "timestamp": 1723910000.0
  }
}
```

---

## 3. Command-Line Interface Usage

Operators can set targets directly from the terminal using the `target_cli` tool:

```bash
# Inject with explicit proposition and Lean 4 code
.venv/bin/python -m bourbakimesh.api.target_cli \
  --name "Mathlib.Order.Basic.le_trans" \
  --prop "a ≤ b -> b ≤ c -> a ≤ c" \
  --priority 200

# Inject raw Lean 4 theorem (signature auto-extracted)
.venv/bin/python -m bourbakimesh.api.target_cli \
  --name "Mathlib.Logic.And.intro" \
  --lean-code "theorem and_intro (a : A) (b : B) : A ∧ B :=\n  And.intro a b"
```

---

## 4. Web UI Target Manager (`TargetManager.tsx`)

The dashboard features a persistent header banner indicating the active swarm theorem:
- Displays theorem name, proposition type, priority badge, and total dedicated simulations.
- **"Set Target Objective" Modal:** Provides an interactive form to paste Lean 4 theorem declarations, customize priority weights, and immediately update the swarm state.
