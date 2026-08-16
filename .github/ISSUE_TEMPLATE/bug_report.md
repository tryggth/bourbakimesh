---
name: Bug Report
about: Report a bug, kernel type-check failure, IR panic, or MCTS search anomaly
title: "fix: "
labels: ["bug"]
assignees: ""
---

### 1. Bug Description
A clear and concise description of the bug.

### 2. Affected Subsystem
- [ ] `crates/bourbaki-ir` (Dialogue Arena AST & Views)
- [ ] `crates/bourbaki-kernel` (CIC AST & Strategy Extractor)
- [ ] `crates/bourbaki-mesh` (Tokio IPC & Proof DAG Ledger)
- [ ] `src/bourbakimesh` (Python ML & Latent MCTS)
- [ ] `lean_target/` (Lean 4 Verification Target & MetaTheory)

### 3. Steps to Reproduce
Steps to reproduce the behavior:
1. ...
2. ...
3. ...

### 4. Expected vs. Actual Behavior
- **Expected:** What should have happened.
- **Actual:** What actually occurred (include panic backtrace or Lean 4 error diagnostics).

### 5. Environment
- OS: [e.g. Linux x86_64, macOS ARM64]
- Rust Toolchain: [e.g. `rustc 1.80+`]
- Python Version: [e.g. `python 3.11+`]
- Lean Toolchain: [e.g. `leanprover/lean4:v4.33.0`]
