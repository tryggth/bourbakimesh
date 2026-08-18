//! Calculus of Inductive Constructions (CIC) Core AST & De Bruijn Terms.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Universe level for CIC Sorts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Level {
    /// Level 0 representing Prop (Sort 0).
    Zero,
    /// Successor universe level: Succ(l) represents l + 1 (e.g. Type 0 = Succ(Zero) = Sort 1).
    Succ(Box<Level>),
    /// Maximum of two universe levels: max(u, v).
    Max(Box<Level>, Box<Level>),
    /// Lean 4 impredicative universe maximum: imax(u, v) = if v == 0 then 0 else max(u, v).
    IMax(Box<Level>, Box<Level>),
    /// Named universe parameter (e.g. u, v).
    Param(String),
}

impl Level {
    /// Universe for Prop (Sort 0).
    pub fn zero() -> Self {
        Level::Zero
    }

    /// Universe for Type 0 (Sort 1 = Succ(Zero)).
    pub fn succ(self) -> Self {
        Level::Succ(Box::new(self))
    }

    /// Maximum universe level.
    pub fn max(l1: Level, l2: Level) -> Self {
        Level::Max(Box::new(l1), Box::new(l2))
    }

    /// Impredicative universe maximum.
    pub fn imax(l1: Level, l2: Level) -> Self {
        Level::IMax(Box::new(l1), Box::new(l2))
    }

    /// Normalizes universe level expression.
    pub fn normalize(&self) -> Level {
        match self {
            Level::Zero => Level::Zero,
            Level::Succ(l) => Level::Succ(Box::new(l.normalize())),
            Level::Max(l1, l2) => {
                let n1 = l1.normalize();
                let n2 = l2.normalize();
                if n1 == Level::Zero {
                    n2
                } else if n2 == Level::Zero {
                    n1
                } else if n1 == n2 {
                    n1
                } else {
                    Level::Max(Box::new(n1), Box::new(n2))
                }
            }
            Level::IMax(l1, l2) => {
                let n1 = l1.normalize();
                let n2 = l2.normalize();
                if n2 == Level::Zero {
                    Level::Zero
                } else if n1 == Level::Zero {
                    n2
                } else if n1 == n2 {
                    n1
                } else {
                    Level::IMax(Box::new(n1), Box::new(n2))
                }
            }
            Level::Param(p) => Level::Param(p.clone()),
        }
    }

    /// Instantiates universe parameters with concrete levels.
    pub fn instantiate_params(&self, subst: &HashMap<String, Level>) -> Level {
        match self {
            Level::Zero => Level::Zero,
            Level::Succ(l) => Level::Succ(Box::new(l.instantiate_params(subst))),
            Level::Max(l1, l2) => Level::Max(
                Box::new(l1.instantiate_params(subst)),
                Box::new(l2.instantiate_params(subst)),
            ),
            Level::IMax(l1, l2) => Level::IMax(
                Box::new(l1.instantiate_params(subst)),
                Box::new(l2.instantiate_params(subst)),
            ),
            Level::Param(p) => subst.get(p).cloned().unwrap_or_else(|| Level::Param(p.clone())),
        }
    }
}

/// Core Calculus of Inductive Constructions (CIC) dependent expressions.
///
/// Implements the 8-constructor Lean 4 core AST with 0-based De Bruijn indices for bound variables.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Expr {
    /// Bound variable with 0-based De Bruijn index.
    BVar(usize),
    /// Free variable or local hypothesis identifier.
    FVar(String),
    /// Universe sort: Sort(0) = Prop, Sort(1) = Type 0, Sort(u + 1) = Type u.
    Sort(Level),
    /// Global definition, axiom, or inductive constructor reference.
    Const(String, Vec<Level>),
    /// Function application: `(f a)`.
    App(Box<Expr>, Box<Expr>),
    /// Lambda abstraction: `λ (x : T), body`.
    Lam(String, Box<Expr>, Box<Expr>),
    /// Dependent Pi-type: `∀ (x : T), B(x)` or non-dependent `T → B`.
    ForallE(String, Box<Expr>, Box<Expr>),
    /// Local let binding: `let x : T := val in body`.
    LetE(String, Box<Expr>, Box<Expr>, Box<Expr>),
}

impl Expr {
    /// Construct Prop sort: `Sort(0)`.
    pub fn prop() -> Self {
        Expr::Sort(Level::Zero)
    }

    /// Construct Type 0 sort: `Sort(1)`.
    pub fn type_0() -> Self {
        Expr::Sort(Level::Succ(Box::new(Level::Zero)))
    }

    /// Construct a free variable reference.
    pub fn fvar(id: impl Into<String>) -> Self {
        Expr::FVar(id.into())
    }

    /// Construct a constant term with universe levels.
    pub fn const_term(name: impl Into<String>, levels: Vec<Level>) -> Self {
        Expr::Const(name.into(), levels)
    }

    /// Construct a function application term `(fun arg)`.
    pub fn app(fun: Expr, arg: Expr) -> Self {
        Expr::App(Box::new(fun), Box::new(arg))
    }

    /// Fold an application spine `f a1 a2 ... an`.
    pub fn mk_app(fun: Expr, args: Vec<Expr>) -> Self {
        args.into_iter().fold(fun, Expr::app)
    }

    /// Construct a lambda abstraction `λ (name : ty), body`.
    pub fn lam(name: impl Into<String>, ty: Expr, body: Expr) -> Self {
        Expr::Lam(name.into(), Box::new(ty), Box::new(body))
    }

    /// Construct a dependent Pi-type `∀ (name : ty), body`.
    pub fn forall_e(name: impl Into<String>, ty: Expr, body: Expr) -> Self {
        Expr::ForallE(name.into(), Box::new(ty), Box::new(body))
    }

    /// Construct a non-dependent function arrow `domain → codomain`.
    pub fn arrow(domain: Expr, codomain: Expr) -> Self {
        Expr::ForallE("_".into(), Box::new(domain), Box::new(codomain.lift(1, 0)))
    }

    /// Construct a let binding `let name : ty := val in body`.
    pub fn let_e(name: impl Into<String>, ty: Expr, val: Expr, body: Expr) -> Self {
        Expr::LetE(name.into(), Box::new(ty), Box::new(val), Box::new(body))
    }

    /// Shifts loose De Bruijn indices by `amount` above cutoff depth `cutoff`.
    pub fn lift(&self, amount: usize, cutoff: usize) -> Expr {
        match self {
            Expr::BVar(i) => {
                if *i >= cutoff {
                    Expr::BVar(i + amount)
                } else {
                    Expr::BVar(*i)
                }
            }
            Expr::App(f, a) => Expr::App(
                Box::new(f.lift(amount, cutoff)),
                Box::new(a.lift(amount, cutoff)),
            ),
            Expr::Lam(name, ty, body) => Expr::Lam(
                name.clone(),
                Box::new(ty.lift(amount, cutoff)),
                Box::new(body.lift(amount, cutoff + 1)),
            ),
            Expr::ForallE(name, ty, body) => Expr::ForallE(
                name.clone(),
                Box::new(ty.lift(amount, cutoff)),
                Box::new(body.lift(amount, cutoff + 1)),
            ),
            Expr::LetE(name, ty, val, body) => Expr::LetE(
                name.clone(),
                Box::new(ty.lift(amount, cutoff)),
                Box::new(val.lift(amount, cutoff)),
                Box::new(body.lift(amount, cutoff + 1)),
            ),
            _ => self.clone(),
        }
    }

    /// Substitutes term `val` for bound variable `BVar(depth)`.
    pub fn instantiate(&self, val: &Expr, depth: usize) -> Expr {
        match self {
            Expr::BVar(i) => {
                if *i == depth {
                    val.lift(depth, 0)
                } else if *i > depth {
                    Expr::BVar(i - 1)
                } else {
                    Expr::BVar(*i)
                }
            }
            Expr::App(f, a) => Expr::App(
                Box::new(f.instantiate(val, depth)),
                Box::new(a.instantiate(val, depth)),
            ),
            Expr::Lam(name, ty, body) => Expr::Lam(
                name.clone(),
                Box::new(ty.instantiate(val, depth)),
                Box::new(body.instantiate(val, depth + 1)),
            ),
            Expr::ForallE(name, ty, body) => Expr::ForallE(
                name.clone(),
                Box::new(ty.instantiate(val, depth)),
                Box::new(body.instantiate(val, depth + 1)),
            ),
            Expr::LetE(name, ty, v, body) => Expr::LetE(
                name.clone(),
                Box::new(ty.instantiate(val, depth)),
                Box::new(v.instantiate(val, depth)),
                Box::new(body.instantiate(val, depth + 1)),
            ),
            _ => self.clone(),
        }
    }

    /// Replaces occurrences of `FVar(fvar_id)` with `BVar(depth)`.
    pub fn abstract_fvar(&self, fvar_id: &str, depth: usize) -> Expr {
        match self {
            Expr::FVar(id) if id == fvar_id => Expr::BVar(depth),
            Expr::App(f, a) => Expr::App(
                Box::new(f.abstract_fvar(fvar_id, depth)),
                Box::new(a.abstract_fvar(fvar_id, depth)),
            ),
            Expr::Lam(name, ty, body) => Expr::Lam(
                name.clone(),
                Box::new(ty.abstract_fvar(fvar_id, depth)),
                Box::new(body.abstract_fvar(fvar_id, depth + 1)),
            ),
            Expr::ForallE(name, ty, body) => Expr::ForallE(
                name.clone(),
                Box::new(ty.abstract_fvar(fvar_id, depth)),
                Box::new(body.abstract_fvar(fvar_id, depth + 1)),
            ),
            Expr::LetE(name, ty, val, body) => Expr::LetE(
                name.clone(),
                Box::new(ty.abstract_fvar(fvar_id, depth)),
                Box::new(val.abstract_fvar(fvar_id, depth)),
                Box::new(body.abstract_fvar(fvar_id, depth + 1)),
            ),
            _ => self.clone(),
        }
    }

    /// Instantiates `BVar(depth)` with `FVar(fvar_id)`.
    pub fn instantiate_fvar(&self, fvar_id: &str, depth: usize) -> Expr {
        self.instantiate(&Expr::FVar(fvar_id.to_string()), depth)
    }

    /// Checks if expression contains any loose bound variables at or above `cutoff`.
    pub fn has_loose_bvars(&self, cutoff: usize) -> bool {
        match self {
            Expr::BVar(i) => *i >= cutoff,
            Expr::App(f, a) => f.has_loose_bvars(cutoff) || a.has_loose_bvars(cutoff),
            Expr::Lam(_, ty, body) | Expr::ForallE(_, ty, body) => {
                ty.has_loose_bvars(cutoff) || body.has_loose_bvars(cutoff + 1)
            }
            Expr::LetE(_, ty, val, body) => {
                ty.has_loose_bvars(cutoff)
                    || val.has_loose_bvars(cutoff)
                    || body.has_loose_bvars(cutoff + 1)
            }
            _ => false,
        }
    }

    /// Instantiates universe parameters across the entire expression.
    pub fn instantiate_level_params(&self, subst: &HashMap<String, Level>) -> Expr {
        if subst.is_empty() {
            return self.clone();
        }
        match self {
            Expr::Sort(lvl) => Expr::Sort(lvl.instantiate_params(subst)),
            Expr::Const(name, levels) => Expr::Const(
                name.clone(),
                levels.iter().map(|l| l.instantiate_params(subst)).collect(),
            ),
            Expr::App(f, a) => Expr::App(
                Box::new(f.instantiate_level_params(subst)),
                Box::new(a.instantiate_level_params(subst)),
            ),
            Expr::Lam(name, ty, body) => Expr::Lam(
                name.clone(),
                Box::new(ty.instantiate_level_params(subst)),
                Box::new(body.instantiate_level_params(subst)),
            ),
            Expr::ForallE(name, ty, body) => Expr::ForallE(
                name.clone(),
                Box::new(ty.instantiate_level_params(subst)),
                Box::new(body.instantiate_level_params(subst)),
            ),
            Expr::LetE(name, ty, val, body) => Expr::LetE(
                name.clone(),
                Box::new(ty.instantiate_level_params(subst)),
                Box::new(val.instantiate_level_params(subst)),
                Box::new(body.instantiate_level_params(subst)),
            ),
            _ => self.clone(),
        }
    }
}
