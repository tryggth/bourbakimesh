//! Calculus of Inductive Constructions (CIC) Core Module.
//!
//! Provides the foundational 8-constructor Lean 4 core AST, De Bruijn indices,
//! inductive families, primitive recursors, β/ζ/δ/ι reduction engines, and a bidirectional type checker.

pub mod expr;
pub mod inductive;
pub mod reduce;
pub mod typecheck;

pub use expr::{Expr, Level};
pub use inductive::{Constructor, InductiveType, Recursor, RecursorRule};
pub use reduce::{is_def_eq, unwind_app, whnf, ConstantInfo, Environment, LocalContext, LocalDecl};
pub use typecheck::{check_type, infer_type, TypeError};
