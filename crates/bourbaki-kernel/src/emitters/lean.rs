//! Lean 4 proof emitter.

use crate::ast::Term;
use crate::emitter::{EmissionError, ProofEmitter, ToLean};

/// Proof emitter for Lean 4.
#[derive(Debug, Clone, Default)]
pub struct LeanEmitter;

impl LeanEmitter {
    /// Create a new Lean 4 emitter.
    pub fn new() -> Self {
        Self
    }
}

impl ProofEmitter for LeanEmitter {
    fn emit_term(&self, term: &Term) -> Result<String, EmissionError> {
        Ok(term.to_lean_string())
    }

    fn emit_theorem(&self, name: &str, ty: &Term, proof: &Term) -> Result<String, EmissionError> {
        let ty_str = self.emit_term(ty)?;
        let proof_str = self.emit_term(proof)?;
        Ok(format!("theorem {} : {} :=\n  {}", name, ty_str, proof_str))
    }

    fn target_name(&self) -> &'static str {
        "Lean 4"
    }

    fn file_extension(&self) -> &'static str {
        "lean"
    }
}
