//! Reduction engine (β, ζ, δ reductions) and definitional equality for CIC terms.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::cic::expr::Expr;

/// Global declaration in the typing environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstantInfo {
    pub name: String,
    pub level_params: Vec<String>,
    pub ty: Expr,
    pub value: Option<Expr>,
}

/// Global typing environment containing constants, axioms, and inductive types.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    constants: HashMap<String, ConstantInfo>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
        }
    }

    /// Add an axiom / constant declaration without a value.
    pub fn add_axiom(&mut self, name: impl Into<String>, ty: Expr) {
        let name = name.into();
        self.constants.insert(
            name.clone(),
            ConstantInfo {
                name,
                level_params: Vec::new(),
                ty,
                value: None,
            },
        );
    }

    /// Add a definition with an explicit value.
    pub fn add_def(&mut self, name: impl Into<String>, ty: Expr, value: Expr) {
        let name = name.into();
        self.constants.insert(
            name.clone(),
            ConstantInfo {
                name,
                level_params: Vec::new(),
                ty,
                value: Some(value),
            },
        );
    }

    /// Retrieve a declaration by name.
    pub fn get(&self, name: &str) -> Option<&ConstantInfo> {
        self.constants.get(name)
    }

    /// Pre-populates the environment with standard intuitionistic logic axioms.
    pub fn default_with_logic() -> Self {
        let mut env = Self::new();
        let prop = Expr::prop();
        let type_0 = Expr::type_0();

        // And : Prop → Prop → Prop
        let and_ty = Expr::arrow(prop.clone(), Expr::arrow(prop.clone(), prop.clone()));
        env.add_axiom("And", and_ty);

        // And.intro : ∀ (A : Prop) (B : Prop), A → B → And A B
        // depth 0: B, depth 1: A
        // under "a : A" (depth 0: a, depth 1: B, depth 2: A)
        // under "b : B" (depth 0: b, depth 1: a, depth 2: B, depth 3: A)
        let and_intro_ty = Expr::forall_e(
            "A",
            prop.clone(),
            Expr::forall_e(
                "B",
                prop.clone(),
                Expr::forall_e(
                    "a",
                    Expr::BVar(1),
                    Expr::forall_e(
                        "b",
                        Expr::BVar(1),
                        Expr::mk_app(
                            Expr::const_term("And", vec![]),
                            vec![Expr::BVar(3), Expr::BVar(2)],
                        ),
                    ),
                ),
            ),
        );
        env.add_axiom("And.intro", and_intro_ty);

        // And.left : ∀ (A : Prop) (B : Prop), And A B → A
        // depth 0: B, depth 1: A
        // under "h : And A B" -> depth 0: h, depth 1: B, depth 2: A
        let and_left_ty = Expr::forall_e(
            "A",
            prop.clone(),
            Expr::forall_e(
                "B",
                prop.clone(),
                Expr::forall_e(
                    "h",
                    Expr::mk_app(
                        Expr::const_term("And", vec![]),
                        vec![Expr::BVar(1), Expr::BVar(0)],
                    ),
                    Expr::BVar(2),
                ),
            ),
        );
        env.add_axiom("And.left", and_left_ty);

        // And.right : ∀ (A : Prop) (B : Prop), And A B → B
        // depth 0: B, depth 1: A
        // under "h : And A B" -> depth 0: h, depth 1: B, depth 2: A
        let and_right_ty = Expr::forall_e(
            "A",
            prop.clone(),
            Expr::forall_e(
                "B",
                prop.clone(),
                Expr::forall_e(
                    "h",
                    Expr::mk_app(
                        Expr::const_term("And", vec![]),
                        vec![Expr::BVar(1), Expr::BVar(0)],
                    ),
                    Expr::BVar(1),
                ),
            ),
        );
        env.add_axiom("And.right", and_right_ty);

        // Or : Prop → Prop → Prop
        let or_ty = Expr::arrow(prop.clone(), Expr::arrow(prop.clone(), prop.clone()));
        env.add_axiom("Or", or_ty);

        // Or.inl : ∀ (A : Prop) (B : Prop), A → Or A B
        let or_inl_ty = Expr::forall_e(
            "A",
            prop.clone(),
            Expr::forall_e(
                "B",
                prop.clone(),
                Expr::forall_e(
                    "a",
                    Expr::BVar(1),
                    Expr::mk_app(
                        Expr::const_term("Or", vec![]),
                        vec![Expr::BVar(2), Expr::BVar(1)],
                    ),
                ),
            ),
        );
        env.add_axiom("Or.inl", or_inl_ty);

        // Or.inr : ∀ (A : Prop) (B : Prop), B → Or A B
        let or_inr_ty = Expr::forall_e(
            "A",
            prop.clone(),
            Expr::forall_e(
                "B",
                prop.clone(),
                Expr::forall_e(
                    "b",
                    Expr::BVar(0),
                    Expr::mk_app(
                        Expr::const_term("Or", vec![]),
                        vec![Expr::BVar(2), Expr::BVar(1)],
                    ),
                ),
            ),
        );
        env.add_axiom("Or.inr", or_inr_ty);

        // False : Prop
        env.add_axiom("False", prop.clone());

        // False.elim : ∀ (C : Prop), False → C
        let false_elim_ty = Expr::forall_e(
            "C",
            prop.clone(),
            Expr::forall_e("h", Expr::const_term("False", vec![]), Expr::BVar(1)),
        );
        env.add_axiom("False.elim", false_elim_ty);

        // Eq : ∀ (A : Type 0), A → A → Prop
        let eq_ty = Expr::forall_e(
            "A",
            type_0.clone(),
            Expr::forall_e(
                "x",
                Expr::BVar(0),
                Expr::forall_e("y", Expr::BVar(1), prop.clone()),
            ),
        );
        env.add_axiom("Eq", eq_ty);

        // Eq.refl : ∀ (A : Type 0) (x : A), Eq A x x
        let eq_refl_ty = Expr::forall_e(
            "A",
            type_0.clone(),
            Expr::forall_e(
                "x",
                Expr::BVar(0),
                Expr::mk_app(
                    Expr::const_term("Eq", vec![]),
                    vec![Expr::BVar(1), Expr::BVar(0), Expr::BVar(0)],
                ),
            ),
        );
        env.add_axiom("Eq.refl", eq_refl_ty);

        env
    }
}

/// Local declaration in a typing context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalDecl {
    pub fvar_id: String,
    pub user_name: String,
    pub ty: Expr,
    pub value: Option<Expr>,
}

/// Local typing context representing active free variables and hypothesis bindings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalContext {
    decls: Vec<LocalDecl>,
    lookup: HashMap<String, usize>,
}

impl LocalContext {
    pub fn new() -> Self {
        Self {
            decls: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    /// Extend context with an assumption `fvar_id : ty`.
    pub fn extend(&self, fvar_id: impl Into<String>, user_name: impl Into<String>, ty: Expr) -> Self {
        let mut ctx = self.clone();
        let id = fvar_id.into();
        let idx = ctx.decls.len();
        ctx.lookup.insert(id.clone(), idx);
        ctx.decls.push(LocalDecl {
            fvar_id: id,
            user_name: user_name.into(),
            ty,
            value: None,
        });
        ctx
    }

    /// Extend context with a let binding `fvar_id : ty := val`.
    pub fn extend_let(
        &self,
        fvar_id: impl Into<String>,
        user_name: impl Into<String>,
        ty: Expr,
        value: Expr,
    ) -> Self {
        let mut ctx = self.clone();
        let id = fvar_id.into();
        let idx = ctx.decls.len();
        ctx.lookup.insert(id.clone(), idx);
        ctx.decls.push(LocalDecl {
            fvar_id: id,
            user_name: user_name.into(),
            ty,
            value: Some(value),
        });
        ctx
    }

    /// Retrieve local declaration by free variable ID.
    pub fn get(&self, fvar_id: &str) -> Option<&LocalDecl> {
        self.lookup.get(fvar_id).and_then(|&idx| self.decls.get(idx))
    }

    /// Iterate over all declarations in context.
    pub fn iter(&self) -> impl Iterator<Item = &LocalDecl> {
        self.decls.iter()
    }
}

/// Computes the Weak Head Normal Form (WHNF) of an expression under environment and local context.
pub fn whnf(expr: &Expr, env: &Environment, ctx: &LocalContext) -> Expr {
    let mut current = expr.clone();
    loop {
        match current {
            Expr::LetE(_, _, val, body) => {
                // Zeta-reduction on local let
                current = body.instantiate(&val, 0);
            }
            Expr::FVar(ref id) => {
                if let Some(decl) = ctx.get(id) {
                    if let Some(ref val) = decl.value {
                        current = val.clone();
                        continue;
                    }
                }
                break;
            }
            Expr::Const(ref name, _) => {
                if let Some(info) = env.get(name) {
                    if let Some(ref val) = info.value {
                        current = val.clone();
                        continue;
                    }
                }
                break;
            }
            Expr::App(f, a) => {
                let f_whnf = whnf(&f, env, ctx);
                match f_whnf {
                    Expr::Lam(_, _, body) => {
                        // Beta-reduction: (λ x. body) a => body[x := a]
                        current = body.instantiate(&a, 0);
                    }
                    _ => {
                        current = Expr::App(Box::new(f_whnf), a);
                        break;
                    }
                }
            }
            _ => break,
        }
    }
    current
}

/// Checks if two CIC expressions are definitionally equal up to WHNF and α-equivalence.
pub fn is_def_eq(e1: &Expr, e2: &Expr, env: &Environment, ctx: &LocalContext) -> bool {
    if e1 == e2 {
        return true;
    }
    let n1 = whnf(e1, env, ctx);
    let n2 = whnf(e2, env, ctx);
    if n1 == n2 {
        return true;
    }

    match (&n1, &n2) {
        (Expr::Sort(l1), Expr::Sort(l2)) => l1.normalize() == l2.normalize(),
        (Expr::BVar(i1), Expr::BVar(i2)) => i1 == i2,
        (Expr::FVar(id1), Expr::FVar(id2)) => id1 == id2,
        (Expr::Const(name1, l1), Expr::Const(name2, l2)) => {
            name1 == name2
                && l1.len() == l2.len()
                && l1.iter().zip(l2).all(|(a, b)| a.normalize() == b.normalize())
        }
        (Expr::App(f1, a1), Expr::App(f2, a2)) => {
            is_def_eq(f1, f2, env, ctx) && is_def_eq(a1, a2, env, ctx)
        }
        (Expr::Lam(_, ty1, body1), Expr::Lam(_, ty2, body2)) => {
            is_def_eq(ty1, ty2, env, ctx) && is_def_eq(body1, body2, env, ctx)
        }
        (Expr::ForallE(_, ty1, body1), Expr::ForallE(_, ty2, body2)) => {
            is_def_eq(ty1, ty2, env, ctx) && is_def_eq(body1, body2, env, ctx)
        }
        _ => false,
    }
}
