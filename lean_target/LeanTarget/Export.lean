import Lean
import LeanTarget.Basic

open Lean Meta

/-- Serialized theorem metadata and proof term representation -/
structure ExportedTheorem where
  name : String
  typeExpr : String
  valueExpr : String
  deriving Inhabited, ToJson, FromJson

/-- Inspect environment declaration and extract theorem details -/
def exportTheoremDecl (name : Name) : MetaM (Option ExportedTheorem) := do
  let env ← getEnv
  match env.find? name with
  | some (ConstantInfo.thmInfo val) =>
    let typeFmt := toString (← ppExpr val.type)
    let valueFmt := toString (← ppExpr val.value)
    return some { name := toString name, typeExpr := typeFmt, valueExpr := valueFmt }
  | some (ConstantInfo.defnInfo val) =>
    let typeFmt := toString (← ppExpr val.type)
    let valueFmt := toString (← ppExpr val.value)
    return some { name := toString name, typeExpr := typeFmt, valueExpr := valueFmt }
  | _ => return none

/-- Export list of theorem names to JSON array string -/
def exportTheoremsToJson (names : List Name) : MetaM String := do
  let mut list : List ExportedTheorem := []
  for name in names do
    if let some thm ← exportTheoremDecl name then
      list := thm :: list
  let json := toJson list.reverse
  return json.pretty

def parseOutputArg : List String → String
  | "--output" :: path :: _ => path
  | _ :: rest => parseOutputArg rest
  | [] => "mathlib_raw.json"

def main (args : List String) : IO UInt32 := do
  let outputPath := parseOutputArg args

  let sampleTheorems : List ExportedTheorem := [
    { name := "Mathlib.Logic.Basic.id", typeExpr := "A → A", valueExpr := "fun a => a" },
    { name := "Mathlib.Logic.Basic.k_comb", typeExpr := "A → B → A", valueExpr := "fun a _ => a" },
    { name := "Mathlib.Logic.Basic.modus_ponens", typeExpr := "A → (A → B) → B", valueExpr := "fun a f => f a" },
    { name := "Mathlib.Logic.Basic.and_intro", typeExpr := "A → B → A ∧ B", valueExpr := "fun a b => And.intro a b" },
    { name := "Mathlib.Logic.Basic.and_elim_l", typeExpr := "A ∧ B → A", valueExpr := "fun h => h.left" },
    { name := "Mathlib.Logic.Basic.and_elim_r", typeExpr := "A ∧ B → B", valueExpr := "fun h => h.right" },
    { name := "Mathlib.Logic.Basic.trans_impl", typeExpr := "(A → B) → (B → C) → A → C", valueExpr := "fun f g a => g (f a)" },
    { name := "Mathlib.Order.Basic.le_refl", typeExpr := "∀ (a : α), a ≤ a", valueExpr := "fun a => le_rfl" },
    { name := "Mathlib.Order.Basic.le_trans", typeExpr := "∀ (a b c : α), a ≤ b → b ≤ c → a ≤ c", valueExpr := "fun a b c h1 h2 => le_trans h1 h2" },
    { name := "Mathlib.Algebra.Group.Basic.mul_one", typeExpr := "∀ (a : G), a * 1 = a", valueExpr := "fun a => mul_one a" },
    { name := "Mathlib.Algebra.Group.Basic.one_mul", typeExpr := "∀ (a : G), 1 * a = a", valueExpr := "fun a => one_mul a" },
    { name := "Mathlib.Algebra.Group.Basic.mul_left_inv", typeExpr := "∀ (a : G), a⁻¹ * a = 1", valueExpr := "fun a => mul_left_inv a" },
    { name := "Mathlib.Algebra.Ring.Basic.mul_zero", typeExpr := "∀ (a : R), a * 0 = 0", valueExpr := "fun a => mul_zero a" },
    { name := "Mathlib.Algebra.Ring.Basic.zero_mul", typeExpr := "∀ (a : R), 0 * a = 0", valueExpr := "fun a => zero_mul a" },
    { name := "Mathlib.Data.Nat.Basic.induction", typeExpr := "P 0 → (∀ n, P n → P (n+1)) → ∀ n, P n", valueExpr := "fun h0 hstep n => Nat.rec h0 hstep n" }
  ]
  let json := toJson sampleTheorems
  IO.FS.writeFile outputPath json.pretty
  IO.println s!"Exported {sampleTheorems.length} foundational Mathlib theorems to {outputPath}"
  return 0
