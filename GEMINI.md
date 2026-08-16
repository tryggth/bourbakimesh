# Agent Governance & Engineering Invariants: BourbakiMesh

This document defines the strict engineering constraints, architectural invariants, verification procedures, and contribution workflows for automated AI agents and human contributors interacting with the `bourbakimesh` repository.

---

## 1. Monorepo Map & Responsibilities

| Subsystem Path | Language / Toolchain | Core Responsibility |
| :--- | :--- | :--- |
| `crates/bourbaki-ir` | Rust (2021/2024 edition) | Game-semantic dialogue arena AST, Lorenzen/Hyland-Ong play traces, interaction nets, polarity algebra. |
| `crates/bourbaki-kernel` | Rust (2021/2024 edition) | Minimal Calculus of Inductive Constructions (CIC) AST, type-check primitives, deterministic proof-term extractor. |
| `crates/bourbaki-mesh` | Rust (2021/2024 edition) | Distributed edge worker node, RPC interfaces, cryptographic proof attestations, ledger synchronization. |
| `src/bourbakimesh` | Python 3.11+ (Torch, NetworkX, FastAPI) | Latent MCTS self-play search, neural policy/value models, dialogue graph dynamics, local worker orchestration API. |
| `lean_target/` | Lean 4 (Lake) | Reference CIC kernel verification harness and Mathlib interoperability target. |

---

## 2. Inviolable Architectural Invariants

Every agent modifying this codebase must adhere to the following invariants:

1. **Strict Dialogue Game Alternation**:
   - In `crates/bourbaki-ir` and `bourbakimesh.dynamics`, moves must strictly alternate between Opponent ($O$) and Proponent ($P$) unless explicitly designated as simultaneous game reductions.
   - Any move referencing prior justification must provide valid indices within bounds.

2. **Constructive Soundness Relative to CIC**:
   - Every extracted winning strategy for Proponent ($P$) in an arena dialogue must be deterministically translatable into a sound Calculus of Inductive Constructions (CIC) term.
   - Proof terms must not rely on `sorry`, `axiom`, or unverified constants unless explicitly flagged as open conjectures in test harnesses.

3. **Zero-Unverified Proof Axioms**:
   - Extracted proofs targeted at `lean_target/` must pass the Lean 4 kernel without admitting unverified axiom extensions.

4. **Self-Contained Worker State**:
   - Mesh worker RPC handlers in `crates/bourbaki-mesh` must be stateless or deterministically stateful against reproducible task IDs.

---

## 3. Standard Verification Loop

Before committing or reporting completion of any task, agents **must** execute and confirm the full verification loop:

```bash
# 1. Rust Workspace Check & Unit Tests
cargo check --workspace
cargo test --workspace

# 2. Python Test Suite & Type Verification
.venv/bin/pytest tests/
.venv/bin/python -c "import torch, networkx, pydantic, bourbakimesh; print('Python verification passed')"

# 3. Lean 4 Kernel Harness
cd lean_target && lake build && cd ..
```

---

## 4. Coding Style & Conventions

- **Rust**:
  - Idiomatic Rust with clear error types using `thiserror`.
  - Comprehensive docstrings with module-level descriptions.
  - Zero unhandled panics (`unwrap()` is prohibited in non-test production code paths; use `?` or explicit error handling).
  - Format with `cargo fmt`.

- **Python**:
  - Python 3.11+ type annotations (`from __future__ import annotations`, `typing` or built-in generics).
  - Pydantic v2 models for serialization/validation of IPC data structures.
  - PEP 8 compliant formatting.

- **Git & Commit Conventions**:
  - Follow Conventional Commits:
    - `feat:` New features or engine components.
    - `fix:` Bug fixes in IR, kernel, or dynamics.
    - `refactor:` Code restructuring without behavior changes.
    - `test:` Test additions or benchmarks.
    - `docs:` Documentation and architecture updates.
    - `chore:` Dependency bumps and tooling updates.
