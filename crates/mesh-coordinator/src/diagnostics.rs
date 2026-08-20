//! Diagnostic Failure Attribution Engine for Proof Search and Kernel Validation.

use kernel::cic::typecheck::TypeError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailureClass {
    MalformedJson(String),
    UnboundDeBruijnIndex { index: usize, max_depth: usize },
    TypeMismatch { expected: String, inferred: String },
    RecursorArgMismatch { recursor: String, details: String },
    TimeoutOrStall,
}

impl FailureClass {
    pub fn from_type_error(err: &TypeError) -> Self {
        match err {
            TypeError::LooseBVar(idx) => FailureClass::UnboundDeBruijnIndex {
                index: *idx,
                max_depth: 0,
            },
            TypeError::TypeMismatch { expected, got } => FailureClass::TypeMismatch {
                expected: format!("{:?}", expected),
                inferred: format!("{:?}", got),
            },
            TypeError::UniverseMismatch(l1, l2) => FailureClass::TypeMismatch {
                expected: format!("{:?}", l1),
                inferred: format!("{:?}", l2),
            },
            TypeError::UnknownConst(name) => FailureClass::RecursorArgMismatch {
                recursor: name.clone(),
                details: format!(
                    "Unknown constant or missing recursor in environment: {}",
                    name
                ),
            },
            TypeError::UnknownFVar(name) => {
                FailureClass::MalformedJson(format!("Unknown free variable: {}", name))
            }
            TypeError::NotAFunction(expr) => FailureClass::TypeMismatch {
                expected: "Function (Arrow or ForallE)".to_string(),
                inferred: format!("{:?}", expr),
            },
            TypeError::NotASort(expr) => FailureClass::TypeMismatch {
                expected: "Sort".to_string(),
                inferred: format!("{:?}", expr),
            },
        }
    }
}
