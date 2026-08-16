//! Bourbaki Kernel: Minimal Calculus of Inductive Constructions (CIC) Translation Layer.
//!
//! Provides the CIC Abstract Syntax Tree (AST), the Lean 4 source code emitter,
//! and the deterministic Strategy Extraction Compiler ($\mathcal{E}(\sigma) \to \text{Term}_{\text{CIC}}$).

pub mod ast;
pub mod emitter;
pub mod extractor;

pub use ast::{BinderInfo, MatchCase, Term, Universe};
pub use emitter::ToLean;
pub use extractor::{ExtractionError, StrategyExtractor, TermExtractor};

/// Backwards compatibility module for CIC AST types.
pub mod cic {
    pub use crate::ast::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_kernel() {
        let u = Universe::prop();
        let term = Term::sort(u);
        assert_eq!(term, Term::Sort(Universe::Zero));
    }
}
