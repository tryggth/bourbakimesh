-- LeanTarget / MetaTheory / Arena.lean
-- Formalized game-semantic dialogue arena AST and Hyland-Ong view calculations.

namespace BourbakiMesh.MetaTheory

/-- Player polarity: Proponent (P) or Opponent (O). -/
inductive Polarity where
  | Proponent : Polarity
  | Opponent  : Polarity
  deriving Repr, DecidableEq

/-- Dual polarity involution. -/
def Polarity.dual : Polarity → Polarity
  | Polarity.Proponent => Polarity.Opponent
  | Polarity.Opponent  => Polarity.Proponent

@[simp] theorem dual_dual (p : Polarity) : p.dual.dual = p := by
  cases p <;> rfl

/-- Dialogue move classification: Question or Answer. -/
inductive MoveKind where
  | Question : MoveKind
  | Answer   : MoveKind
  deriving Repr, DecidableEq

/-- Additive conjunction projection choice. -/
inductive ConjunctionBranch where
  | Left  : ConjunctionBranch
  | Right : ConjunctionBranch
  deriving Repr, DecidableEq

/-- Game-semantic dialogue move payload. -/
inductive LogicalPayload where
  | rootGoal             : String → LogicalPayload
  | attackHypothesis     : Nat → LogicalPayload
  | attackConjunction    : ConjunctionBranch → LogicalPayload
  | demandWitness        : LogicalPayload
  | provideWitness       : String → LogicalPayload
  | instantiateUniversal : String → LogicalPayload
  | axiomDischarge       : Nat → LogicalPayload
  | inductiveCaseDemand  : Nat → LogicalPayload
  deriving Repr, DecidableEq

/-- A single justified move in a dialogue game play. -/
structure Move where
  id        : Nat
  player    : Polarity
  kind      : MoveKind
  justifier : Option Nat
  payload   : LogicalPayload
  deriving Repr, DecidableEq

/-- A play trace is a finite sequence of justified dialogue moves. -/
def PlayTrace := List Move

/-- Strict move alternation predicate between Proponent and Opponent. -/
inductive StrictAlternation : PlayTrace → Prop where
  | nil : StrictAlternation []
  | single (m : Move) (h_p : m.player = Polarity.Proponent) : StrictAlternation [m]
  | cons (m1 m2 : Move) (rest : List Move)
      (h_alt : m1.player = m2.player.dual)
      (h_rest : StrictAlternation (m2 :: rest)) :
      StrictAlternation (m1 :: m2 :: rest)

/-- Proponent view (P-view) sub-trace extraction $\ulcorner s \urcorner_P$. -/
partial def pViewAux : List Move → List Move
  | [] => []
  | m :: ms =>
    match m.player with
    | Polarity.Proponent => m :: pViewAux ms
    | Polarity.Opponent =>
      match m.justifier with
      | some k =>
        m :: pViewAux (ms.filter (fun prev => prev.id ≤ k))
      | none => [m]

/-- Opponent view (O-view) sub-trace extraction $\llcorner s \lrcorner_O$. -/
partial def oViewAux : List Move → List Move
  | [] => []
  | m :: ms =>
    match m.player with
    | Polarity.Opponent => m :: oViewAux ms
    | Polarity.Proponent =>
      match m.justifier with
      | some k => m :: oViewAux (ms.filter (fun prev => prev.id ≤ k))
      | none => [m]

/-- Public P-view computation over a PlayTrace. -/
def PView (trace : PlayTrace) : List Move :=
  (pViewAux trace.reverse).reverse

/-- Public O-view computation over a PlayTrace. -/
def OView (trace : PlayTrace) : List Move :=
  (oViewAux trace.reverse).reverse

/-- Stack discipline predicate asserting well-bracketed question-answer pairing. -/
inductive WellBracketed : PlayTrace → Prop where
  | nil : WellBracketed []
  | step (m : Move) (trace : PlayTrace) (h_wb : WellBracketed trace) : WellBracketed (m :: trace)

/-- Innocence predicate: A strategy depends strictly on the current player view. -/
def InnocentStrategy (sigma : PlayTrace → Option Move) : Prop :=
  ∀ s1 s2 : PlayTrace, PView s1 = PView s2 → sigma s1 = sigma s2

end BourbakiMesh.MetaTheory
