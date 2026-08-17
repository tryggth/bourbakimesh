//! Bourbaki Kernel: Minimal Calculus of Inductive Constructions (CIC) Translation Layer.
//!
//! Provides the CIC Abstract Syntax Tree (AST), universal target emitters (Lean 4, Coq, Isabelle/HOL, Dedukti),
//! the deterministic Strategy Extraction Compiler ($\mathcal{E}(\sigma) \to \text{Term}_{\text{CIC}}$),
//! the term-to-strategy decompiler, batch corpus indexing, and the Zero-Trust Lean 4 verification harness bridge.

pub mod ast;
pub mod corpus;
pub mod decompiler;
pub mod emitter;
pub mod emitters;
pub mod extractor;
pub mod verifier;

pub use ast::{BinderInfo, MatchCase, Term, Universe};
pub use corpus::{CorpusDataset, CorpusDecompiler, CorpusError, DecompiledTheorem, RawTheorem};
pub use decompiler::{CICDecompiler, DecompileError};
pub use emitter::{EmissionError, ProofEmitter, ToLean};
pub use emitters::{CoqEmitter, DeduktiEmitter, IsabelleEmitter, LeanEmitter};
pub use extractor::{ExtractionError, StrategyExtractor, TermExtractor};
pub use verifier::{LeanEnvironment, VerificationError, VerificationReport};

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
