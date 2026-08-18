//! Bidirectional type checker (infer_type and check_type) for the Calculus of Inductive Constructions (CIC).

use std::sync::atomic::{AtomicUsize, Ordering};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::cic::expr::{Expr, Level};
use crate::cic::reduce::{is_def_eq, whnf, Environment, LocalContext};

static FRESH_VAR_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn fresh_fvar_id(prefix: &str) -> String {
    let count = FRESH_VAR_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}_{}", prefix, count)
}

/// Errors raised during CIC type inference and type checking.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum TypeError {
    #[error("Loose bound variable BVar({0}) encountered during type checking")]
    LooseBVar(usize),

    #[error("Unknown free variable: {0}")]
    UnknownFVar(String),

    #[error("Unknown constant / axiom: {0}")]
    UnknownConst(String),

    #[error("Expected function / Pi-type, got: {0:?}")]
    NotAFunction(Expr),

    #[error("Expected type sort, got: {0:?}")]
    NotASort(Expr),

    #[error("Type mismatch: expected {expected:?}, got {got:?}")]
    TypeMismatch {
        expected: Expr,
        got: Expr,
    },

    #[error("Universe level mismatch: {0:?} vs {1:?}")]
    UniverseMismatch(Level, Level),
}

/// Infers the type of a CIC expression under an environment and local context.
pub fn infer_type(expr: &Expr, env: &Environment, ctx: &LocalContext) -> Result<Expr, TypeError> {
    match expr {
        Expr::BVar(i) => Err(TypeError::LooseBVar(*i)),

        Expr::FVar(id) => {
            if let Some(decl) = ctx.get(id) {
                Ok(decl.ty.clone())
            } else {
                Err(TypeError::UnknownFVar(id.clone()))
            }
        }

        Expr::Sort(lvl) => {
            // Sort(u) : Sort(u + 1)
            Ok(Expr::Sort(Level::Succ(Box::new(lvl.clone()))))
        }

        Expr::Const(name, _levels) => {
            if let Some(info) = env.get(name) {
                Ok(info.ty.clone())
            } else {
                Err(TypeError::UnknownConst(name.clone()))
            }
        }

        Expr::ForallE(name, domain, codomain) => {
            // 1. Ensure domain is a sort
            let d_ty = infer_type(domain, env, ctx)?;
            let d_ty_whnf = whnf(&d_ty, env, ctx);
            let u1 = match d_ty_whnf {
                Expr::Sort(lvl) => lvl,
                _ => return Err(TypeError::NotASort(d_ty)),
            };

            // 2. Extend context with fresh fvar for parameter
            let fvar = fresh_fvar_id(name);
            let ext_ctx = ctx.extend(&fvar, name, *domain.clone());

            // 3. Instantiate codomain and infer its type
            let codomain_inst = codomain.instantiate_fvar(&fvar, 0);
            let c_ty = infer_type(&codomain_inst, env, &ext_ctx)?;
            let c_ty_whnf = whnf(&c_ty, env, &ext_ctx);
            let u2 = match c_ty_whnf {
                Expr::Sort(lvl) => lvl,
                _ => return Err(TypeError::NotASort(c_ty)),
            };

            // 4. Return Sort(imax(u1, u2))
            Ok(Expr::Sort(Level::imax(u1, u2).normalize()))
        }

        Expr::Lam(name, domain, body) => {
            // 1. Ensure domain is a valid type (its type is a Sort)
            let d_ty = infer_type(domain, env, ctx)?;
            let d_ty_whnf = whnf(&d_ty, env, ctx);
            if !matches!(d_ty_whnf, Expr::Sort(_)) {
                return Err(TypeError::NotASort(d_ty));
            }

            // 2. Extend context with parameter binding
            let fvar = fresh_fvar_id(name);
            let ext_ctx = ctx.extend(&fvar, name, *domain.clone());

            // 3. Instantiate body with the free variable
            let body_inst = body.instantiate_fvar(&fvar, 0);
            let b_ty = infer_type(&body_inst, env, &ext_ctx)?;

            // 4. Abstract the free variable back into De Bruijn BVar(0)
            let b_ty_abstract = b_ty.abstract_fvar(&fvar, 0);

            Ok(Expr::ForallE(
                name.clone(),
                domain.clone(),
                Box::new(b_ty_abstract),
            ))
        }

        Expr::App(f, arg) => {
            // 1. Infer type of function
            let f_ty = infer_type(f, env, ctx)?;
            let f_ty_whnf = whnf(&f_ty, env, ctx);

            // 2. Ensure f is a Pi-type: ∀ (x : domain), codomain
            let (domain, codomain) = match f_ty_whnf {
                Expr::ForallE(_, domain, codomain) => (domain, codomain),
                _ => return Err(TypeError::NotAFunction(f_ty)),
            };

            // 3. Infer type of argument and check against domain
            let arg_ty = infer_type(arg, env, ctx)?;
            if !is_def_eq(&arg_ty, &domain, env, ctx) {
                return Err(TypeError::TypeMismatch {
                    expected: *domain,
                    got: arg_ty,
                });
            }

            // 4. Return codomain with BVar(0) instantiated with arg
            Ok(codomain.instantiate(arg, 0))
        }

        Expr::LetE(name, ty, val, body) => {
            // 1. Check ty is a valid sort
            let sort_ty = infer_type(ty, env, ctx)?;
            if !matches!(whnf(&sort_ty, env, ctx), Expr::Sort(_)) {
                return Err(TypeError::NotASort(sort_ty));
            }

            // 2. Check val has type ty
            check_type(val, ty, env, ctx)?;

            // 3. Extend context with let-value
            let fvar = fresh_fvar_id(name);
            let ext_ctx = ctx.extend_let(&fvar, name, *ty.clone(), *val.clone());

            // 4. Infer type of body
            let body_inst = body.instantiate_fvar(&fvar, 0);
            let b_ty = infer_type(&body_inst, env, &ext_ctx)?;

            // 5. Abstract fvar back
            Ok(b_ty.abstract_fvar(&fvar, 0).instantiate(val, 0))
        }
    }
}

/// Checks that a term has the expected type under an environment and local context.
pub fn check_type(
    term: &Expr,
    expected: &Expr,
    env: &Environment,
    ctx: &LocalContext,
) -> Result<(), TypeError> {
    let inferred = infer_type(term, env, ctx)?;
    if is_def_eq(&inferred, expected, env, ctx) {
        Ok(())
    } else {
        Err(TypeError::TypeMismatch {
            expected: expected.clone(),
            got: inferred,
        })
    }
}
