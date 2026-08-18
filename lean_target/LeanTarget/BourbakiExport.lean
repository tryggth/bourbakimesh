import Lean
import LeanTarget.Basic

open Lean Meta Elab Command

def levelToJson (l : Lean.Level) : Json :=
  match l with
  | .zero => Json.str "Zero"
  | .succ l' => Json.mkObj [("Succ", levelToJson l')]
  | .max l1 l2 => Json.mkObj [("Max", Json.arr #[levelToJson l1, levelToJson l2])]
  | .imax l1 l2 => Json.mkObj [("IMax", Json.arr #[levelToJson l1, levelToJson l2])]
  | .param n => Json.mkObj [("Param", Json.str n.toString)]
  | .mvar n => Json.mkObj [("Param", Json.str n.name.toString)]

def exprToJson (e : Lean.Expr) : Json :=
  match e with
  | .bvar idx => Json.mkObj [("BVar", Json.num idx)]
  | .fvar id => Json.mkObj [("FVar", Json.str id.name.toString)]
  | .mvar id => Json.mkObj [("FVar", Json.str id.name.toString)]
  | .sort lvl => Json.mkObj [("Sort", levelToJson lvl)]
  | .const name lvls => Json.mkObj [("Const", Json.arr #[Json.str name.toString, Json.arr (lvls.toArray.map levelToJson)])]
  | .app f a => Json.mkObj [("App", Json.arr #[exprToJson f, exprToJson a])]
  | .lam n t b _ => Json.mkObj [("Lam", Json.arr #[Json.str n.toString, exprToJson t, exprToJson b])]
  | .forallE n t b _ => Json.mkObj [("ForallE", Json.arr #[Json.str n.toString, exprToJson t, exprToJson b])]
  | .letE n t v b _ => Json.mkObj [("LetE", Json.arr #[Json.str n.toString, exprToJson t, exprToJson v, exprToJson b])]
  | .mdata _ e' => exprToJson e'
  | .lit (.natVal n) => Json.mkObj [("Const", Json.arr #[Json.str (toString n), Json.arr #[]])]
  | .lit (.strVal s) => Json.mkObj [("Const", Json.arr #[Json.str s, Json.arr #[]])]
  | .proj typeName idx struct =>
    Json.mkObj [("App", Json.arr #[Json.mkObj [("Const", Json.arr #[Json.str s!"{typeName}.proj_{idx}", Json.arr #[]])], exprToJson struct])]

syntax (name := exportBourbaki) "#export_bourbaki " ident : command

@[command_elab exportBourbaki]
def elabExportBourbaki : CommandElab := fun stx => do
  let id := stx[1].getId
  let env ← getEnv
  match env.find? id with
  | some (.thmInfo val) =>
    let payload := Json.mkObj [
      ("name", Json.str id.toString),
      ("type", exprToJson val.type),
      ("value", exprToJson val.value)
    ]
    IO.FS.writeFile s!"../artifacts/exported_{id.toString}.json" (payload.pretty)
    logInfo s!"Exported theorem {id} to artifacts/exported_{id.toString}.json"
  | some (.defnInfo val) =>
    let payload := Json.mkObj [
      ("name", Json.str id.toString),
      ("type", exprToJson val.type),
      ("value", exprToJson val.value)
    ]
    IO.FS.writeFile s!"../artifacts/exported_{id.toString}.json" (payload.pretty)
    logInfo s!"Exported definition {id} to artifacts/exported_{id.toString}.json"
  | _ => throwError s!"Declaration {id} not found or unsupported"

-- Foundational explicit proofs for direct CIC type checking
theorem And.swap {A B : Prop} (h : And A B) : And B A :=
  And.intro h.right h.left

theorem Or.swap {A B : Prop} (h : Or A B) : Or B A :=
  Or.elim h (fun a => Or.inr a) (fun b => Or.inl b)

-- Export foundational declarations
#export_bourbaki id_prop
#export_bourbaki k_comb
#export_bourbaki modus_ponens_thm
#export_bourbaki and_intro_thm
#export_bourbaki trans_impl_thm
#export_bourbaki And.swap
#export_bourbaki Or.swap
#export_bourbaki Eq.symm
