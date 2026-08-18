use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Expr {
    Prop(String),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Impl(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    False,
    Eq(String, String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule")]
pub enum DeductionStep {
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
    Reflexivity { term: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStatus {
    Open,
    Proven,
}
