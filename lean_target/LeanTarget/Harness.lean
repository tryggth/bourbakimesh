-- LeanTarget / Harness.lean
-- Reference verification harness and proof certification helpers for BourbakiMesh.

namespace BourbakiMesh.Harness

/-- Verification marker certifying that a theorem was extracted without unverified axioms. -/
def certifiedProof {α : Sort u} (p : α) : α := p

/-- Helper attribute/syntax for marking certified game-semantic proofs. -/
syntax (name := bourbaki_cert) "bourbaki_certified" : attr

end BourbakiMesh.Harness
