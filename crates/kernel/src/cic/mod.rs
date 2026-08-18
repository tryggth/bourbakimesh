//! Calculus of Inductive Constructions (CIC) Core Module.
//!
//! Provides the foundational 8-constructor Lean 4 core AST, De Bruijn indices,
//! β/ζ/δ reduction engines, and a bidirectional type checker.

pub mod expr;
pub mod reduce;
pub mod typecheck;

pub use expr::{Expr, Level};
pub use reduce::{ConstantInfo, Environment, LocalContext, LocalDecl, is_def_eq, whnf};
pub use typecheck::{TypeError, check_type, infer_type};
