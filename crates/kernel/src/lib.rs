//! Deterministic, microsecond-latency propositional proof kernel and state transition engine.

pub mod ast;
pub mod cic;
pub mod state;

pub use ast::{DeductionStep, Expr as PropExpr, ProofStatus};
pub use state::{KernelError, ProofState};
