//! Calculus of Inductive Constructions (CIC) Abstract Syntax Tree (AST).

use serde::{Deserialize, Serialize};

/// Universe level for CIC Sorts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Universe {
    /// Level 0 representing Prop.
    Zero,
    /// Successor universe level (e.g. Type 0 = Succ(Zero), Type 1 = Succ(Succ(Zero))).
    Succ(Box<Universe>),
    /// Named universe parameter (e.g. u, v).
    Param(String),
}

impl Universe {
    /// Universe for Prop.
    pub fn prop() -> Self {
        Universe::Zero
    }

    /// Universe for Type 0.
    pub fn type_0() -> Self {
        Universe::Succ(Box::new(Universe::Zero))
    }

    /// Construct successor universe level.
    pub fn succ(self) -> Self {
        Universe::Succ(Box::new(self))
    }

    /// Construct a named universe parameter.
    pub fn param(name: impl Into<String>) -> Self {
        Universe::Param(name.into())
    }
}

/// Binder annotations for parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinderInfo {
    /// Default explicit binder: (x : A).
    Default,
    /// Implicit binder: {x : A}.
    Implicit,
    /// Strict implicit binder: ⦃x : A⦄.
    StrictImplicit,
}

/// A pattern-matching branch in an inductive elimination match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchCase {
    /// Inductive constructor name (e.g. "And.intro", "Or.inl", "Nat.succ").
    pub constructor: String,
    /// Bound variable names for constructor arguments.
    pub bindings: Vec<String>,
    /// Branch consequence body.
    pub body: Box<Term>,
}

impl MatchCase {
    /// Create a new match case.
    pub fn new(constructor: impl Into<String>, bindings: Vec<String>, body: Term) -> Self {
        Self {
            constructor: constructor.into(),
            bindings,
            body: Box::new(body),
        }
    }
}

/// Core Calculus of Inductive Constructions (CIC) proof and type terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Term {
    /// Universe Sort (Prop or Type u).
    Sort(Universe),
    /// Local named variable or active hypothesis identifier.
    Var(String),
    /// Global axiom, theorem constant, or inductive type constructor reference.
    Const(String, Vec<Universe>),
    /// Function application: (f arg).
    App(Box<Term>, Box<Term>),
    /// Lambda abstraction: λ (x : A) => body.
    Lam(String, Box<Term>, Box<Term>),
    /// Dependent function type: Π (x : A) -> B.
    Pi(String, Box<Term>, Box<Term>),
    /// Local let binding: let x : A := val in body.
    Let(String, Box<Term>, Box<Term>, Box<Term>),
    /// Inductive case split / pattern matching.
    Match {
        discriminant: Box<Term>,
        motive: Option<Box<Term>>,
        cases: Vec<MatchCase>,
    },
    /// Uninstantiated axiom reference.
    Axiom(String),
}

impl Term {
    /// Construct a variable reference term.
    pub fn var(name: impl Into<String>) -> Self {
        Term::Var(name.into())
    }

    /// Construct a universe sort term.
    pub fn sort(u: Universe) -> Self {
        Term::Sort(u)
    }

    /// Construct Prop sort.
    pub fn prop() -> Self {
        Term::Sort(Universe::prop())
    }

    /// Construct Type 0 sort.
    pub fn type_0() -> Self {
        Term::Sort(Universe::type_0())
    }

    /// Construct a global constant term with universe instantiations.
    pub fn const_term(name: impl Into<String>, universes: Vec<Universe>) -> Self {
        Term::Const(name.into(), universes)
    }

    /// Construct a function application term `(fun arg)`.
    pub fn app(fun: Term, arg: Term) -> Self {
        Term::App(Box::new(fun), Box::new(arg))
    }

    /// Fold an application spine `f a1 a2 ... an`.
    pub fn mk_app(fun: Term, args: Vec<Term>) -> Self {
        args.into_iter().fold(fun, Term::app)
    }

    /// Construct a lambda abstraction `fun (binder : ty) => body`.
    pub fn lam(binder: impl Into<String>, ty: Term, body: Term) -> Self {
        Term::Lam(binder.into(), Box::new(ty), Box::new(body))
    }

    /// Construct a dependent product `(binder : ty) -> body`.
    pub fn pi(binder: impl Into<String>, ty: Term, body: Term) -> Self {
        Term::Pi(binder.into(), Box::new(ty), Box::new(body))
    }

    /// Construct a simple arrow type `domain -> codomain`.
    pub fn arrow(domain: Term, codomain: Term) -> Self {
        Term::Pi("_".into(), Box::new(domain), Box::new(codomain))
    }

    /// Construct a let binding `let name : ty := val in body`.
    pub fn let_in(name: impl Into<String>, ty: Term, val: Term, body: Term) -> Self {
        Term::Let(name.into(), Box::new(ty), Box::new(val), Box::new(body))
    }

    /// Construct a match expression on an inductive term.
    pub fn match_term(discriminant: Term, motive: Option<Term>, cases: Vec<MatchCase>) -> Self {
        Term::Match {
            discriminant: Box::new(discriminant),
            motive: motive.map(Box::new),
            cases,
        }
    }

    /// Construct an axiom term.
    pub fn axiom(name: impl Into<String>) -> Self {
        Term::Axiom(name.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_construction() {
        let a = Term::var("a");
        let prop = Term::prop();
        let id_type = Term::arrow(prop.clone(), prop.clone());
        let id_term = Term::lam("a", prop, a);

        match id_term {
            Term::Lam(name, ty, body) => {
                assert_eq!(name, "a");
                assert_eq!(*ty, Term::Sort(Universe::Zero));
                assert_eq!(*body, Term::Var("a".into()));
            }
            _ => panic!("Expected Lam"),
        }

        assert_eq!(id_type, Term::pi("_", Term::prop(), Term::prop()));
    }
}
