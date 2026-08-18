# Universal Multi-Target Compilers & Proof Extraction

BourbakiMesh decouples theorem search from target proof assistant syntaxes by using **game-semantic dialogue arenas** as an intermediate representation (IR) and deterministically compiling winning strategies into multiple interactive theorem provers.

---

## 1. The Strategy Extraction Pipeline $\mathcal{E}(\sigma)$

A winning strategy for Proponent ($P$) in an arena dialogue is a prefix-closed, deterministic subtree of valid moves $\sigma \subseteq \text{PlayTrace}$.

The extraction compiler $\mathcal{E}(\sigma)$ deterministically translates $\sigma$ into a typed Calculus of Inductive Constructions (CIC) term $\tau \in \text{Term}$:

```
                                          ┌──────────────┐
                                          │ Lean 4 (.lean)│
                                          └──────▲───────┘
                                                 │
┌─────────────────┐       ┌──────────┐    ┌──────┴───────┐    ┌─────────────────┐
│ Winning Strategy│  ==>  │ CIC AST  │ ==>│ ProofEmitter ├───►│ Coq Gallina (.v)│
│  Tree σ in IR   │ E(σ)  │  (Kernel)│    │  (Universal) ├───►│ Isabelle/HOL    │
└─────────────────┘       └──────────┘    └──────┬───────┘    │   (Isar .thy)   │
                                                 │            └─────────────────┘
                                          ┌──────▼───────┐
                                          │ Dedukti (.dk)│
                                          └──────────────┘
```

### Extraction Rules:
1. **Axiom / Hypothesis Leaf:** An answered question referencing hypothesis $x$ compiles to variable usage `x`.
2. **Implication Introduction ( $\lambda$-Abstraction):** Proponent asserting an implication hypothesis opens an opponent challenge, extracting `Term::Lambda(x, Ty, body)`.
3. **Modus Ponens (Application):** Proponent defending an attacked goal with an implication extracts `Term::App(f, arg)`.
4. **Conjunction Introduction (Pair Constructor):** Branching on conjunction components $A \wedge B$ extracts `Term::App(Term::App(Term::Const("And.intro"), pA), pB)`.
5. **Cut Lemma Let-Binding:** Intermediate game arena cuts extract to structured `Term::Let(name, ty, val, body)` bindings.

---

## 2. Pluggable `ProofEmitter` Targets

The `crates/bourbaki-kernel::emitters` module provides code generators for major interactive theorem provers:

### 1. Lean 4 Emitter (`LeanEmitter`)
Outputs canonical Lean 4 definitions checked directly by `lake env lean`:
```lean
theorem and_intro (A B : Prop) (hA : A) (hB : B) : A ∧ B :=
  And.intro hA hB
```

### 2. Coq / Gallina Emitter (`CoqEmitter`)
Outputs standard Coq Gallina definitions with `conj` pairing:
```coq
Theorem and_intro (A B : Prop) (hA : A) (hB : B) : A /\ B :=
  conj hA hB.
```

### 3. Isabelle/HOL Isar Emitter (`IsabelleEmitter`)
Outputs declarative Isar proof text:
```isabelle
theorem and_intro:
  fixes A B :: bool
  assumes hA: "A" and hB: "B"
  shows "A ∧ B"
  using assms by (rule conjI)
```

### 4. Dedukti Emitter (`DeduktiEmitter`)
Outputs higher-order rewrite signatures for the Dedukti logical framework:
```dedukti
thm_and_intro : A : Prop -> B : Prop -> Proof A -> Proof B -> Proof (and A B) :=
  a => b => ha => hb => and_intro a b ha hb.
```

---

## 3. Zero-Trust Soundness Invariant

- **Zero Unverified Axioms:** No emitted proof term may contain `sorry`, `axiom`, or unverified primitives.
- **Round-Trip Isomorphism:** Verified through Tier 3b testing:
  $$\text{Lean 4 / Mathlib AST} \xrightarrow{\text{Decompile}} \text{Arena Strategy} \xrightarrow{\mathcal{E}} \text{CIC Term} \xrightarrow{\text{Emit}} \text{Lean 4 Kernel Validated}$$
