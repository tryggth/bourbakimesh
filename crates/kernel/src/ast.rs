use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Expr {
    Prop(String),
    And(Box<Expr>, Box<Expr>),
    Impl(Box<Expr>, Box<Expr>),
    Eq(String, String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule")]
pub enum DeductionStep {
    AndElimL { hyp: String },
    AndElimR { hyp: String },
    AndIntro { left: String, right: String },
    ModusPonens { r#impl: String, arg: String },
    Exact { hyp: String },
    Reflexivity { term: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStatus {
    Open,
    Proven,
}
