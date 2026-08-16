-- LeanTarget / MetaTheory / CIC.lean
-- Deep embedding of Calculus of Inductive Constructions (CIC) terms and typing judgments.

namespace BourbakiMesh.MetaTheory

/-- Deep embedding of minimal Calculus of Inductive Constructions (CIC) terms. -/
inductive CICTerm where
  | sort : Nat → CICTerm
  | var  : String → CICTerm
  | lam  : String → CICTerm → CICTerm → CICTerm
  | pi   : String → CICTerm → CICTerm → CICTerm
  | app  : CICTerm → CICTerm → CICTerm
  | letE : String → CICTerm → CICTerm → CICTerm → CICTerm
  deriving Repr, DecidableEq

/-- Helper constructor for non-dependent arrow types A -> B. -/
def CICTerm.arrow (A B : CICTerm) : CICTerm :=
  CICTerm.pi "_" A B

/-- Typing context associating variable identifiers with types. -/
abbrev Context := List (String × CICTerm)

/-- Inductive typing judgment $\Gamma \vdash t : T$ in minimal CIC. -/
inductive Typing : Context → CICTerm → CICTerm → Prop where
  | sort (Γ : Context) (u : Nat) :
      Typing Γ (CICTerm.sort u) (CICTerm.sort (u + 1))
  | var (Γ : Context) (x : String) (A : CICTerm)
      (h_mem : (x, A) ∈ Γ) :
      Typing Γ (CICTerm.var x) A
  | lam (Γ : Context) (x : String) (A B t : CICTerm)
      (h_body : Typing ((x, A) :: Γ) t B) :
      Typing Γ (CICTerm.lam x A t) (CICTerm.pi x A B)
  | app (Γ : Context) (f a : CICTerm) (x : String) (A B : CICTerm)
      (h_fun : Typing Γ f (CICTerm.pi x A B))
      (h_arg : Typing Γ a A) :
      Typing Γ (CICTerm.app f a) B
  | let_bind (Γ : Context) (x : String) (A v t B : CICTerm)
      (h_val : Typing Γ v A)
      (h_body : Typing ((x, A) :: Γ) t B) :
      Typing Γ (CICTerm.letE x A v t) B

end BourbakiMesh.MetaTheory
