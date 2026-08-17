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
