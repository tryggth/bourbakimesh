//! Pluggable universal target emitters for Lean 4, Coq, Isabelle/HOL, and Dedukti.

pub mod coq;
pub mod dedukti;
pub mod isabelle;
pub mod lean;

pub use coq::CoqEmitter;
pub use dedukti::DeduktiEmitter;
pub use isabelle::IsabelleEmitter;
pub use lean::LeanEmitter;
