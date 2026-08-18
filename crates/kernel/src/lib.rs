//! Deterministic, microsecond-latency propositional proof kernel and state transition engine.

pub mod ast;
pub mod state;

pub use ast::{DeductionStep, Expr, ProofStatus};
pub use state::{KernelError, ProofState};
