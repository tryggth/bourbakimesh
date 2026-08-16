---
name: Architectural Design Proposal
about: Propose a major architectural change, new game-semantic IR structure, or kernel subsystem
title: "rfc: "
labels: ["enhancement"]
assignees: ""
---

### 1. Proposal Overview
A clear, concise summary of the proposed architectural change.

### 2. Motivation & Game-Semantic Justification
- What problem does this proposal address?
- How does it align with dialogue game semantics, constructive CIC extraction, or decentralized proof ledger invariants?

### 3. Proposed Design
- Proposed data structures, traits, or state machines.
- Expected API changes across Rust (`crates/`), Python (`src/bourbakimesh`), or Lean 4 (`lean_target/`).

### 4. Soundness & Invariant Impact
- [ ] Preserves strict Proponent / Opponent move alternation
- [ ] Retains valid justification pointer and well-bracketing stack discipline
- [ ] Fully translatable to constructive CIC proof terms without `sorry`
- [ ] No unhandled panics or unwraps in production code paths

### 5. Alternatives Considered
What alternative approaches were evaluated and why was this selected?
