//! Reduction engine (β, ζ, δ, ι reductions) and definitional equality for CIC terms.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::cic::expr::{Expr, Level};
use crate::cic::inductive::{InductiveType, Recursor};

/// Global declaration in the typing environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstantInfo {
    pub name: String,
    pub level_params: Vec<String>,
    pub ty: Expr,
    pub value: Option<Expr>,
}

impl ConstantInfo {
    /// Instantiates universe parameters in the constant's declared type.
    pub fn instantiate_type(&self, levels: &[Level]) -> Expr {
        if self.level_params.is_empty() {
            return self.ty.clone();
        }
        let mut subst = HashMap::new();
        for (i, param) in self.level_params.iter().enumerate() {
            let lvl = levels.get(i).cloned().unwrap_or(Level::Zero);
            subst.insert(param.clone(), lvl);
        }
        self.ty.instantiate_level_params(&subst)
    }
}

/// Global typing environment containing constants, axioms, and inductive types.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    constants: HashMap<String, ConstantInfo>,
    inductives: HashMap<String, InductiveType>,
    recursors: HashMap<String, Recursor>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            inductives: HashMap::new(),
            recursors: HashMap::new(),
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

    /// Add an axiom with universe level parameters.
    pub fn add_poly_axiom(&mut self, name: impl Into<String>, level_params: Vec<String>, ty: Expr) {
        let name = name.into();
        self.constants.insert(
            name.clone(),
            ConstantInfo {
                name,
                level_params,
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

    /// Add an inductive type family and its recursor into the environment.
    pub fn add_inductive(&mut self, ind: InductiveType, rec: Recursor) {
        // Register the inductive type itself
        self.add_poly_axiom(ind.name.clone(), ind.level_params.clone(), ind.ty.clone());

        // Register each constructor
        for ctor in &ind.constructors {
            self.add_poly_axiom(ctor.name.clone(), ind.level_params.clone(), ctor.ty.clone());
        }

        // Register the recursor
        self.recursors.insert(rec.name.clone(), rec.clone());
        self.inductives.insert(ind.name.clone(), ind);
    }

    /// Retrieve a declaration by name.
    pub fn get(&self, name: &str) -> Option<&ConstantInfo> {
        self.constants.get(name)
    }

    /// Retrieve an inductive type declaration by name.
    pub fn get_inductive(&self, name: &str) -> Option<&InductiveType> {
        self.inductives.get(name)
    }

    /// Retrieve a recursor by name.
    pub fn get_recursor(&self, name: &str) -> Option<&Recursor> {
        self.recursors.get(name)
    }

    /// Pre-populates the environment with canonical inductive types and standard logic axioms.
    pub fn default_with_logic() -> Self {
        let mut env = Self::new();
        let prop = Expr::prop();
        let type_0 = Expr::type_0();

        // 1. Register canonical inductive families
        let (bool_ind, bool_rec) = InductiveType::bool_type();
        env.add_inductive(bool_ind, bool_rec);

        let (nat_ind, nat_rec) = InductiveType::nat_type();
        env.add_inductive(nat_ind, nat_rec);

        let (and_ind, and_rec) = InductiveType::and_type();
        env.add_inductive(and_ind, and_rec);

        let (or_ind, or_rec) = InductiveType::or_type();
        env.add_inductive(or_ind, or_rec);

        let (eq_ind, eq_rec) = InductiveType::eq_type();
        env.add_inductive(eq_ind, eq_rec);

        let (list_ind, list_rec) = InductiveType::list_type();
        env.add_inductive(list_ind, list_rec);

        let (false_ind, false_rec) = InductiveType::false_type();
        env.add_inductive(false_ind, false_rec);

        // 2. Add projection helpers & aliases
        // And.left : ∀ (A B : Prop) (h : And A B), A
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

        // And.right : ∀ (A B : Prop) (h : And A B), B
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

        // And.symm : ∀ (A B : Prop) (h : And A B), And B A
        let and_symm_ty = Expr::forall_e(
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
                    Expr::mk_app(
                        Expr::const_term("And", vec![]),
                        vec![Expr::BVar(1), Expr::BVar(2)],
                    ),
                ),
            ),
        );
        env.add_axiom("And.symm", and_symm_ty);

        // Or.elim : ∀ (A B : Prop) (C : Sort u) (h : Or A B) (ha : A → C) (hb : B → C), C
        let or_elim_ty = Expr::forall_e(
            "A",
            prop.clone(),
            Expr::forall_e(
                "B",
                prop.clone(),
                Expr::forall_e(
                    "C",
                    Expr::Sort(Level::Param("u".into())),
                    Expr::forall_e(
                        "h",
                        Expr::mk_app(
                            Expr::const_term("Or", vec![]),
                            vec![Expr::BVar(2), Expr::BVar(1)],
                        ),
                        Expr::forall_e(
                            "ha",
                            Expr::arrow(Expr::BVar(3), Expr::BVar(1)),
                            Expr::forall_e(
                                "hb",
                                Expr::arrow(Expr::BVar(3), Expr::BVar(2)),
                                Expr::BVar(3),
                            ),
                        ),
                    ),
                ),
            ),
        );
        env.add_poly_axiom("Or.elim", vec!["u".into()], or_elim_ty);

        // Or.symm : ∀ (A B : Prop) (h : Or A B), Or B A
        let or_symm_ty = Expr::forall_e(
            "A",
            prop.clone(),
            Expr::forall_e(
                "B",
                prop.clone(),
                Expr::forall_e(
                    "h",
                    Expr::mk_app(
                        Expr::const_term("Or", vec![]),
                        vec![Expr::BVar(1), Expr::BVar(0)],
                    ),
                    Expr::mk_app(
                        Expr::const_term("Or", vec![]),
                        vec![Expr::BVar(1), Expr::BVar(2)],
                    ),
                ),
            ),
        );
        env.add_axiom("Or.symm", or_symm_ty);

        // False.elim : ∀ (C : Prop) (h : False), C
        let false_elim_ty = Expr::forall_e(
            "C",
            prop.clone(),
            Expr::forall_e("h", Expr::const_term("False", vec![]), Expr::BVar(1)),
        );
        env.add_axiom("False.elim", false_elim_ty);

        // rfl : ∀ (α : Type u) (a : α), Eq α a a (universe polymorphic)
        let rfl_ty = Expr::forall_e(
            "α",
            Expr::Sort(Level::Param("u".into())),
            Expr::forall_e(
                "a",
                Expr::BVar(0),
                Expr::mk_app(
                    Expr::const_term("Eq", vec![Level::Param("u".into())]),
                    vec![Expr::BVar(1), Expr::BVar(0), Expr::BVar(0)],
                ),
            ),
        );
        env.add_poly_axiom("rfl", vec!["u".into()], rfl_ty);

        // Eq.rec : ∀ (α : Type u) (a : α) (motive : ∀ (x : α) (h : Eq α a x), Sort u_1), motive a (rfl α a) → ∀ (b : α) (h : Eq α a b), motive b h
        let eq_rec_ty = Expr::forall_e(
            "α",
            Expr::Sort(Level::Param("u".into())),
            Expr::forall_e(
                "a",
                Expr::BVar(0),
                Expr::forall_e(
                    "motive",
                    Expr::forall_e(
                        "x",
                        Expr::BVar(1),
                        Expr::forall_e(
                            "h",
                            Expr::mk_app(
                                Expr::const_term("Eq", vec![Level::Param("u".into())]),
                                vec![Expr::BVar(2), Expr::BVar(1), Expr::BVar(0)],
                            ),
                            Expr::Sort(Level::Param("u_1".into())),
                        ),
                    ),
                    Expr::forall_e(
                        "minor",
                        Expr::mk_app(
                            Expr::BVar(0),
                            vec![
                                Expr::BVar(1),
                                Expr::mk_app(
                                    Expr::const_term("rfl", vec![Level::Param("u".into())]),
                                    vec![Expr::BVar(2), Expr::BVar(1)],
                                ),
                            ],
                        ),
                        Expr::forall_e(
                            "b",
                            Expr::BVar(3),
                            Expr::forall_e(
                                "h",
                                Expr::mk_app(
                                    Expr::const_term("Eq", vec![Level::Param("u".into())]),
                                    vec![Expr::BVar(4), Expr::BVar(3), Expr::BVar(0)],
                                ),
                                Expr::mk_app(
                                    Expr::BVar(3),
                                    vec![Expr::BVar(1), Expr::BVar(0)],
                                ),
                            ),
                        ),
                    ),
                ),
            ),
        );
        env.add_poly_axiom("Eq.rec", vec!["u_1".into(), "u".into()], eq_rec_ty);

        // Iff : Prop → Prop → Prop
        let iff_ty = Expr::arrow(prop.clone(), Expr::arrow(prop.clone(), prop.clone()));
        env.add_axiom("Iff", iff_ty);

        // Iff.intro : ∀ (A B : Prop) (mp : A → B) (mpr : B → A), Iff A B
        let iff_intro_ty = Expr::forall_e(
            "A",
            prop.clone(),
            Expr::forall_e(
                "B",
                prop.clone(),
                Expr::forall_e(
                    "mp",
                    Expr::arrow(Expr::BVar(1), Expr::BVar(0)),
                    Expr::forall_e(
                        "mpr",
                        Expr::arrow(Expr::BVar(1), Expr::BVar(2)),
                        Expr::mk_app(
                            Expr::const_term("Iff", vec![]),
                            vec![Expr::BVar(3), Expr::BVar(2)],
                        ),
                    ),
                ),
            ),
        );
        env.add_axiom("Iff.intro", iff_intro_ty);

        // HAdd.hAdd & OfNat for arithmetic
        let hadd_ty = Expr::forall_e(
            "α",
            type_0.clone(),
            Expr::forall_e(
                "β",
                type_0.clone(),
                Expr::forall_e(
                    "γ",
                    type_0.clone(),
                    Expr::arrow(
                        Expr::fvar("instHAdd"),
                        Expr::arrow(Expr::BVar(3), Expr::arrow(Expr::BVar(3), Expr::BVar(3))),
                    ),
                ),
            ),
        );
        env.add_axiom("HAdd.hAdd", hadd_ty);
        env.add_axiom("instHAdd", type_0.clone());
        env.add_axiom("instAddNat", type_0.clone());
        env.add_axiom("instOfNatNat", type_0.clone());
        env.add_axiom("OfNat.ofNat", type_0.clone());
        env.add_axiom("0", Expr::const_term("Nat", vec![]));

        env
    }
}

/// Unwinds an application spine: `App(App(f, a1), a2)` -> `(f, [a1, a2])`.
pub fn unwind_app(expr: &Expr) -> (Expr, Vec<Expr>) {
    let mut args = Vec::new();
    let mut curr = expr;
    while let Expr::App(f, a) = curr {
        args.push((**a).clone());
        curr = f;
    }
    args.reverse();
    (curr.clone(), args)
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
///
/// Implements β (lambda reduction), ζ (let reduction), δ (constant unfolding),
/// and ι (primitive recursor / constructor matching) reductions.
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
            Expr::App(..) => {
                let (head, args) = unwind_app(&current);
                let head_whnf = whnf(&head, env, ctx);

                // 1. Beta-reduction: (λ x. body) a => body[x := a]
                if let Expr::Lam(_, _, body) = head_whnf {
                    let mut res = body.instantiate(&args[0], 0);
                    for arg in &args[1..] {
                        res = Expr::app(res, arg.clone());
                    }
                    current = res;
                    continue;
                }

                // 2. Delta-reduction on defined constant head
                if let Expr::Const(ref name, _) = head_whnf {
                    if let Some(info) = env.get(name) {
                        if let Some(ref val) = info.value {
                            let mut res = val.clone();
                            for arg in &args {
                                res = Expr::app(res, arg.clone());
                            }
                            current = res;
                            continue;
                        }
                    }
                }

                // 3. Iota-reduction: Recursor applied to constructor term
                if let Expr::Const(ref rec_name, _) = head_whnf {
                    if let Some(rec) = env.get_recursor(rec_name) {
                        let major_idx = rec.num_params + rec.num_motives + rec.num_minors + rec.num_indices;
                        if args.len() > major_idx {
                            let major_whnf = whnf(&args[major_idx], env, ctx);
                            let (major_head, major_args) = unwind_app(&major_whnf);
                            if let Expr::Const(ref ctor_name, _) = major_head {
                                if let Some(rule_idx) = rec.rules.iter().position(|r| &r.ctor_name == ctor_name) {
                                    let minor = &args[rec.num_params + rec.num_motives + rule_idx];

                                    // Constructor field arguments (skipping inductive type parameters)
                                    let fields = if major_args.len() >= rec.num_params {
                                        &major_args[rec.num_params..]
                                    } else {
                                        &[]
                                    };

                                    // Compute reduced body
                                    let mut reduced = if rec.ind_name == "Nat" && ctor_name == "Nat.succ" && !fields.is_empty() {
                                        let n = &fields[0];
                                        let mut rec_call_args = Vec::new();
                                        for i in 0..(rec.num_params + rec.num_motives + rec.num_minors) {
                                            rec_call_args.push(args[i].clone());
                                        }
                                        rec_call_args.push(n.clone());
                                        let rec_call = Expr::mk_app(Expr::const_term(rec.name.clone(), vec![]), rec_call_args);
                                        Expr::mk_app(minor.clone(), vec![n.clone(), rec_call])
                                    } else if rec.ind_name == "List" && ctor_name == "List.cons" && fields.len() >= 2 {
                                        let head = &fields[0];
                                        let tail = &fields[1];
                                        let mut rec_call_args = Vec::new();
                                        for i in 0..(rec.num_params + rec.num_motives + rec.num_minors) {
                                            rec_call_args.push(args[i].clone());
                                        }
                                        rec_call_args.push(tail.clone());
                                        let rec_call = Expr::mk_app(Expr::const_term(rec.name.clone(), vec![]), rec_call_args);
                                        Expr::mk_app(minor.clone(), vec![head.clone(), tail.clone(), rec_call])
                                    } else {
                                        Expr::mk_app(minor.clone(), fields.to_vec())
                                    };

                                    // Apply any trailing arguments past the major premise
                                    if args.len() > major_idx + 1 {
                                        reduced = Expr::mk_app(reduced, args[major_idx + 1..].to_vec());
                                    }

                                    current = reduced;
                                    continue;
                                }
                            }
                        }
                    }
                }

                // Reconstruct spine if no reduction fired
                let mut res = head_whnf;
                for a in args {
                    res = Expr::app(res, a);
                }
                current = res;
                break;
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
        (Expr::Sort(l1), Expr::Sort(l2)) => {
            l1.normalize() == l2.normalize()
                || matches!(l1, Level::Param(_))
                || matches!(l2, Level::Param(_))
        }
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
