use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Term {
    Var(String),
    Const(String),
    Func(String, Vec<Term>),
}

impl Term {
    pub fn substitute(&self, var: &str, replacement: &Term) -> Term {
        match self {
            Term::Var(v) if v == var => replacement.clone(),
            Term::Var(v) => Term::Var(v.clone()),
            Term::Const(c) => Term::Const(c.clone()),
            Term::Func(name, args) => Term::Func(
                name.clone(),
                args.iter().map(|a| a.substitute(var, replacement)).collect(),
            ),
        }
    }

    pub fn replace_term(&self, target: &Term, replacement: &Term) -> Term {
        if self == target {
            return replacement.clone();
        }
        match self {
            Term::Func(name, args) => Term::Func(
                name.clone(),
                args.iter().map(|a| a.replace_term(target, replacement)).collect(),
            ),
            _ => self.clone(),
        }
    }

    pub fn contains_var(&self, var: &str) -> bool {
        match self {
            Term::Var(v) => v == var,
            Term::Const(_) => false,
            Term::Func(_, args) => args.iter().any(|a| a.contains_var(var)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Expr {
    Prop(String),
    Pred(String, Vec<Term>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Impl(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    False,
    Eq(Term, Term),
    Forall { var: String, body: Box<Expr> },
    Exists { var: String, body: Box<Expr> },
}

impl Expr {
    pub fn substitute(&self, var: &str, replacement: &Term) -> Expr {
        match self {
            Expr::Prop(p) => Expr::Prop(p.clone()),
            Expr::Pred(p, terms) => Expr::Pred(
                p.clone(),
                terms.iter().map(|t| t.substitute(var, replacement)).collect(),
            ),
            Expr::And(left, right) => Expr::And(
                Box::new(left.substitute(var, replacement)),
                Box::new(right.substitute(var, replacement)),
            ),
            Expr::Or(left, right) => Expr::Or(
                Box::new(left.substitute(var, replacement)),
                Box::new(right.substitute(var, replacement)),
            ),
            Expr::Impl(ant, con) => Expr::Impl(
                Box::new(ant.substitute(var, replacement)),
                Box::new(con.substitute(var, replacement)),
            ),
            Expr::Not(inner) => Expr::Not(Box::new(inner.substitute(var, replacement))),
            Expr::False => Expr::False,
            Expr::Eq(t1, t2) => Expr::Eq(
                t1.substitute(var, replacement),
                t2.substitute(var, replacement),
            ),
            Expr::Forall { var: v, body } => {
                if v == var {
                    Expr::Forall {
                        var: v.clone(),
                        body: body.clone(),
                    }
                } else {
                    Expr::Forall {
                        var: v.clone(),
                        body: Box::new(body.substitute(var, replacement)),
                    }
                }
            }
            Expr::Exists { var: v, body } => {
                if v == var {
                    Expr::Exists {
                        var: v.clone(),
                        body: body.clone(),
                    }
                } else {
                    Expr::Exists {
                        var: v.clone(),
                        body: Box::new(body.substitute(var, replacement)),
                    }
                }
            }
        }
    }

    pub fn replace_term(&self, target: &Term, replacement: &Term) -> Expr {
        match self {
            Expr::Prop(p) => Expr::Prop(p.clone()),
            Expr::Pred(p, terms) => Expr::Pred(
                p.clone(),
                terms.iter().map(|t| t.replace_term(target, replacement)).collect(),
            ),
            Expr::And(left, right) => Expr::And(
                Box::new(left.replace_term(target, replacement)),
                Box::new(right.replace_term(target, replacement)),
            ),
            Expr::Or(left, right) => Expr::Or(
                Box::new(left.replace_term(target, replacement)),
                Box::new(right.replace_term(target, replacement)),
            ),
            Expr::Impl(ant, con) => Expr::Impl(
                Box::new(ant.replace_term(target, replacement)),
                Box::new(con.replace_term(target, replacement)),
            ),
            Expr::Not(inner) => Expr::Not(Box::new(inner.replace_term(target, replacement))),
            Expr::False => Expr::False,
            Expr::Eq(t1, t2) => Expr::Eq(
                t1.replace_term(target, replacement),
                t2.replace_term(target, replacement),
            ),
            Expr::Forall { var, body } => Expr::Forall {
                var: var.clone(),
                body: Box::new(body.replace_term(target, replacement)),
            },
            Expr::Exists { var, body } => Expr::Exists {
                var: var.clone(),
                body: Box::new(body.replace_term(target, replacement)),
            },
        }
    }

    pub fn contains_free_var(&self, var: &str) -> bool {
        match self {
            Expr::Prop(_) => false,
            Expr::Pred(_, terms) => terms.iter().any(|t| t.contains_var(var)),
            Expr::And(l, r) | Expr::Or(l, r) | Expr::Impl(l, r) => {
                l.contains_free_var(var) || r.contains_free_var(var)
            }
            Expr::Not(inner) => inner.contains_free_var(var),
            Expr::False => false,
            Expr::Eq(t1, t2) => t1.contains_var(var) || t2.contains_var(var),
            Expr::Forall { var: v, body } | Expr::Exists { var: v, body } => {
                if v == var {
                    false
                } else {
                    body.contains_free_var(var)
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule")]
pub enum DeductionStep {
    // Propositional Primitives
    AndElimL { hyp: String },
    AndElimR { hyp: String },
    AndIntro { left: String, right: String },
    OrIntroL { hyp: String, right: Expr },
    OrIntroR { left: Expr, hyp: String },
    OrElim { hyp_or: String, left_impl: String, right_impl: String },
    Contradiction { pos_hyp: String, neg_hyp: String },
    FalseElim { hyp_false: String },
    ModusPonens { r#impl: String, arg: String },
    Exact { hyp: String },
    Reflexivity { term: Term },

    // First-Order Logic Primitives
    ForallElim { hyp: String, term: Term },
    ExistsIntro { hyp: String, var: String, body: Expr },
    ExistsElim { hyp_exists: String, hyp_impl: String },
    Rewrite { eq_hyp: String, target_hyp: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStatus {
    Open,
    Proven,
}
