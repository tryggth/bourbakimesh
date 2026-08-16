# Contributing to BourbakiMesh

Thank you for your interest in contributing to **BourbakiMesh**!

BourbakiMesh is an open-source decentralized automated theorem proving ecosystem combining **polarized game-semantic dialogue arenas (Hyland-Ong / Lorenzen)**, **Calculus of Inductive Constructions (CIC) proof-term extraction**, **PyTorch Latent MCTS self-play dynamics**, and a **peer-to-peer proof DAG ledger**.

---

## 1. Project Philosophy & Core Invariants

Every contribution must respect the foundational engineering and theoretical invariants:

1. **Machine-First Constructive Proving:** Dialogue plays between Proponent ($P$) and Opponent ($O$) are the ground-truth search space. Winning strategies compile deterministically into verifiable CIC terms.
2. **Zero-Trust Verification:** Neural policies, heuristic searchers, and strategy extractors are untrusted. Proofs are only certified once validated by the official reference Lean 4 kernel without admitting `sorry` or unverified axioms.
3. **Strict Dialogue Alternation:** Moves strictly alternate between Proponent ($P$) and Opponent ($O$), with valid justification pointers respecting active P-views and well-bracketing stack discipline.
4. **Zero-Warning Compiler Policy:** All Rust code must compile cleanly under `cargo clippy --workspace -- -D warnings` with zero unhandled panics (`unwrap()` is prohibited in non-test production paths).

---

## 2. Communication Channels

- **[GitHub Discussions](https://github.com/tryggth/bourbakimesh/discussions):** Ideal for open-ended design discussions, mathematical formalization ideas, and game-theoretic explorations.
- **[RFC Process](rfcs/):** Required for substantial architectural shifts, new IR move types, kernel lowering rules, or network consensus changes. Use `rfcs/0000-template.md`.
- **[GitHub Issues](https://github.com/tryggth/bourbakimesh/issues):** For concrete bug reports, feature implementations, test additions, and performance profiling.

---

## 3. Local Development Setup

BourbakiMesh is a polyglot monorepo requiring **Rust**, **Python**, and **Lean 4**.

### Prerequisites

1. **Rust Toolchain (2021/2024 Edition):**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup component add clippy rustfmt
   ```

2. **Python Environment (3.11+):**
   ```bash
   python3 -m venv .venv
   source .venv/bin/activate
   pip install -e ".[dev]"
   ```

3. **Lean 4 & Lake (`elan`):**
   ```bash
   curl https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -sSf | sh
   source ~/.elan/env
   ```

---

## 4. Standard Verification Loop

Before submitting a Pull Request, contributors **must** execute and confirm the full monorepo verification loop:

```bash
# 1. Format & Lint Checks
cargo fmt --check
cargo check --workspace
cargo clippy --workspace -- -D warnings

# 2. Rust Unit & Integration Tests (including Tier 3a Proptest)
cargo test --workspace

# 3. Rust Criterion Micro-Benchmarks
cargo bench --workspace -- --test

# 4. Python Test Suite (Latent MCTS, Tableau Bootstrapping, Benchmarks)
.venv/bin/pytest tests/

# 5. Lean 4 Kernel Harness & MetaTheory Formalization
cd lean_target && lake build && cd ..
```

---

## 5. Git Commit Conventions

We enforce [Conventional Commits](https://www.conventionalcommits.org/):

- `feat(scope)`: New features or engine components (e.g., `feat(kernel): add conjunction extraction rule`).
- `fix(scope)`: Bug fixes in IR, kernel, mesh, or dynamics (e.g., `fix(ir): correct P-view bounds check`).
- `refactor(scope)`: Restructuring code without changing behavior.
- `test(scope)`: Adding new unit, proptest, or adversarial test suites.
- `perf(scope)`: Performance optimizations and benchmarking suites.
- `docs(scope)`: Documentation, architecture specs, or wiki updates.
- `chore(scope)`: Tooling, dependency, or workspace updates.

---

## 6. Pull Request Process

1. **Fork & Branch:** Create a feature branch off `main` (e.g., `feat/transformer-dynamics` or `fix/p-view-indexing`).
2. **Implement & Test:** Implement your changes with corresponding test coverage.
3. **Execute Verification Loop:** Ensure all 5 verification steps pass cleanly.
4. **Submit PR:** Open a Pull Request on GitHub referencing the related issue or RFC. Fill out the PR checklist completely.
5. **Code Review:** Address feedback from maintainers. Once approved and CI passes, your PR will be merged into `main`.
