//! Lean 4 syntax emitter converting CIC terms into valid Lean 4 code.

use crate::ast::{MatchCase, Term, Universe};

/// Trait for emitting Lean 4 source syntax.
pub trait ToLean {
    /// Emit Lean 4 source code representation.
    fn to_lean_string(&self) -> String;
}

impl ToLean for Universe {
    fn to_lean_string(&self) -> String {
        match self {
            Universe::Zero => "0".into(),
            Universe::Succ(inner) => {
                if let Universe::Zero = **inner {
                    "1".into()
                } else {
                    format!("({} + 1)", inner.to_lean_string())
                }
            }
            Universe::Param(name) => name.clone(),
        }
    }
}

impl ToLean for Term {
    fn to_lean_string(&self) -> String {
        format_term(self, 0)
    }
}

impl ToLean for MatchCase {
    fn to_lean_string(&self) -> String {
        if self.bindings.is_empty() {
            format!("| {} => {}", self.constructor, self.body.to_lean_string())
        } else {
            format!(
                "| {} {} => {}",
                self.constructor,
                self.bindings.join(" "),
                self.body.to_lean_string()
            )
        }
    }
}

/// Helper formatting terms with precedence levels to minimize redundant parentheses.
/// Precedence levels:
/// 0: Top level (Let, Match, Lam)
/// 1: Arrow / Pi (->)
/// 2: Application (f arg)
/// 3: Atomic (Var, Const, Sort, (expr))
fn format_term(term: &Term, prec: u8) -> String {
    match term {
        Term::Sort(u) => match u {
            Universe::Zero => "Prop".into(),
            Universe::Succ(inner) if **inner == Universe::Zero => "Type".into(),
            Universe::Succ(_) => format!("Type {}", u.to_lean_string()),
            Universe::Param(p) => format!("Type {}", p),
        },
        Term::Var(name) => name.clone(),
        Term::Const(name, _) => name.clone(),
        Term::Axiom(name) => name.clone(),
        Term::Lam(binder, ty, body) => {
            let res = format!(
                "fun ({} : {}) => {}",
                binder,
                format_term(ty, 0),
                format_term(body, 0)
            );
            if prec > 0 {
                format!("({})", res)
            } else {
                res
            }
        }
        Term::Pi(binder, domain, codomain) => {
            let res = if binder == "_" {
                let dom_str = format_term(domain, 2);
                let codom_str = format_term(codomain, 1);
                format!("{} -> {}", dom_str, codom_str)
            } else {
                format!(
                    "({} : {}) -> {}",
                    binder,
                    format_term(domain, 0),
                    format_term(codomain, 1)
                )
            };
            if prec > 1 {
                format!("({})", res)
            } else {
                res
            }
        }
        Term::App(fun, arg) => {
            let fun_str = format_term(fun, 2);
            let arg_str = format_term(arg, 3);
            let res = format!("{} {}", fun_str, arg_str);
            if prec >= 3 {
                format!("({})", res)
            } else {
                res
            }
        }
        Term::Let(name, ty, val, body) => {
            let res = format!(
                "let {} : {} := {}; {}",
                name,
                format_term(ty, 0),
                format_term(val, 0),
                format_term(body, 0)
            );
            if prec > 0 {
                format!("({})", res)
            } else {
                res
            }
        }
        Term::Match {
            discriminant,
            cases,
            ..
        } => {
            let cases_str = cases
                .iter()
                .map(|c| c.to_lean_string())
                .collect::<Vec<_>>()
                .join(" ");
            let res = format!("match {} with {}", format_term(discriminant, 0), cases_str);
            if prec > 0 {
                format!("({})", res)
            } else {
                res
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emitter_formatting() {
        // Identity: fun (x : A) => x
        let id_term = Term::lam("x", Term::var("A"), Term::var("x"));
        assert_eq!(id_term.to_lean_string(), "fun (x : A) => x");

        // Arrow: A -> B -> A
        let k_type = Term::arrow(Term::var("A"), Term::arrow(Term::var("B"), Term::var("A")));
        assert_eq!(k_type.to_lean_string(), "A -> B -> A");

        // Application: f a (g b)
        let app_term = Term::mk_app(
            Term::var("f"),
            vec![Term::var("a"), Term::app(Term::var("g"), Term::var("b"))],
        );
        assert_eq!(app_term.to_lean_string(), "f a (g b)");

        // Match: match v with | And.intro a b => a
        let match_term = Term::match_term(
            Term::var("v"),
            None,
            vec![MatchCase::new(
                "And.intro",
                vec!["a".into(), "b".into()],
                Term::var("a"),
            )],
        );
        assert_eq!(
            match_term.to_lean_string(),
            "match v with | And.intro a b => a"
        );
    }
}
