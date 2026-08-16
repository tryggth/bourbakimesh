//! Calculus of Inductive Constructions (CIC) Term AST.

use serde::{Deserialize, Serialize};

/// Universe level for CIC Sorts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Level {
    Zero,
    Succ(u32),
    Param(u32),
}

/// CIC Sorts (Prop, Type u).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sort {
    Prop,
    Type(Level),
}

/// Core CIC proof and type terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Term {
    /// Universe Sort.
    Sort(Sort),
    /// De Bruijn index or named variable.
    Var { index: usize, name: Option<String> },
    /// Global axiom or theorem constant reference.
    Const { name: String },
    /// Function abstraction: λ (x : T), body.
    Lambda {
        binder_name: String,
        binder_type: Box<Term>,
        body: Box<Term>,
    },
    /// Dependent function type: Π (x : T), body.
    Pi {
        binder_name: String,
        binder_type: Box<Term>,
        body: Box<Term>,
    },
    /// Function application: (f arg).
    App {
        fun: Box<Term>,
        arg: Box<Term>,
    },
    /// Let binding: let x : T := val in body.
    Let {
        binder_name: String,
        binder_type: Box<Term>,
        value: Box<Term>,
        body: Box<Term>,
    },
}

impl Term {
    /// Helper to construct a simple arrow type `A -> B` (non-dependent Pi).
    pub fn arrow(domain: Term, codomain: Term) -> Self {
        Term::Pi {
            binder_name: "_".into(),
            binder_type: Box::new(domain),
            body: Box::new(codomain),
        }
    }

    /// Helper to construct an application chain `f a1 a2 ... an`.
    pub fn mk_app(fun: Term, args: Vec<Term>) -> Self {
        args.into_iter().fold(fun, |acc, arg| Term::App {
            fun: Box::new(acc),
            arg: Box::new(arg),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cic_term_construction() {
        let prop = Term::Sort(Sort::Prop);
        let arrow = Term::arrow(prop.clone(), prop.clone());
        match arrow {
            Term::Pi { binder_type, body, .. } => {
                assert_eq!(*binder_type, Term::Sort(Sort::Prop));
                assert_eq!(*body, Term::Sort(Sort::Prop));
            }
            _ => panic!("Expected Pi term"),
        }
    }
}
