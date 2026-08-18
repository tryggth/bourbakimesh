use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::ast::{DeductionStep, Expr, ProofStatus};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KernelError {
    #[error("Hypothesis not found: {0}")]
    HypothesisNotFound(String),
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },
    #[error("Invalid rule application: {0}")]
    InvalidRuleApplication(String),
    #[error("Proof is already closed")]
    ProofAlreadyClosed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofState {
    pub hyps: HashMap<String, Expr>,
    pub target: Expr,
    pub status: ProofStatus,
    next_hyp_idx: usize,
}

impl ProofState {
    pub fn new(initial_hyps: Vec<(String, Expr)>, target: Expr) -> Self {
        let mut hyps = HashMap::new();
        let mut max_idx = 0;
        for (id, expr) in initial_hyps {
            if let Some(num_str) = id.strip_prefix('h') {
                if let Ok(idx) = num_str.parse::<usize>() {
                    if idx >= max_idx {
                        max_idx = idx + 1;
                    }
                }
            }
            hyps.insert(id, expr);
        }
        Self {
            hyps,
            target,
            status: ProofStatus::Open,
            next_hyp_idx: max_idx,
        }
    }

    /// Evaluates a single deduction step and updates the hypothesis pool or closes the proof.
    pub fn apply_step(&mut self, step: &DeductionStep) -> Result<Option<String>, KernelError> {
        if self.status == ProofStatus::Proven {
            return Err(KernelError::ProofAlreadyClosed);
        }

        match step {
            DeductionStep::AndElimL { hyp } => {
                let expr = self.hyps.get(hyp).ok_or_else(|| KernelError::HypothesisNotFound(hyp.clone()))?;
                match expr {
                    Expr::And(left, _) => {
                        let new_id = format!("h{}", self.next_hyp_idx);
                        self.next_hyp_idx += 1;
                        self.hyps.insert(new_id.clone(), (**left).clone());
                        Ok(Some(new_id))
                    }
                    _ => Err(KernelError::TypeMismatch {
                        expected: "And(_, _)".to_string(),
                        found: format!("{:?}", expr),
                    }),
                }
            }
            DeductionStep::AndElimR { hyp } => {
                let expr = self.hyps.get(hyp).ok_or_else(|| KernelError::HypothesisNotFound(hyp.clone()))?;
                match expr {
                    Expr::And(_, right) => {
                        let new_id = format!("h{}", self.next_hyp_idx);
                        self.next_hyp_idx += 1;
                        self.hyps.insert(new_id.clone(), (**right).clone());
                        Ok(Some(new_id))
                    }
                    _ => Err(KernelError::TypeMismatch {
                        expected: "And(_, _)".to_string(),
                        found: format!("{:?}", expr),
                    }),
                }
            }
            DeductionStep::AndIntro { left, right } => {
                let left_expr = self.hyps.get(left).ok_or_else(|| KernelError::HypothesisNotFound(left.clone()))?;
                let right_expr = self.hyps.get(right).ok_or_else(|| KernelError::HypothesisNotFound(right.clone()))?;
                let new_expr = Expr::And(Box::new(left_expr.clone()), Box::new(right_expr.clone()));
                let new_id = format!("h{}", self.next_hyp_idx);
                self.next_hyp_idx += 1;
                self.hyps.insert(new_id.clone(), new_expr);
                Ok(Some(new_id))
            }
            DeductionStep::ModusPonens { r#impl, arg } => {
                let impl_expr = self.hyps.get(r#impl).ok_or_else(|| KernelError::HypothesisNotFound(r#impl.clone()))?;
                let arg_expr = self.hyps.get(arg).ok_or_else(|| KernelError::HypothesisNotFound(arg.clone()))?;
                match impl_expr {
                    Expr::Impl(antecedent, consequent) => {
                        if **antecedent == *arg_expr {
                            let new_id = format!("h{}", self.next_hyp_idx);
                            self.next_hyp_idx += 1;
                            self.hyps.insert(new_id.clone(), (**consequent).clone());
                            Ok(Some(new_id))
                        } else {
                            Err(KernelError::TypeMismatch {
                                expected: format!("{:?}", antecedent),
                                found: format!("{:?}", arg_expr),
                            })
                        }
                    }
                    _ => Err(KernelError::TypeMismatch {
                        expected: "Impl(_, _)".to_string(),
                        found: format!("{:?}", impl_expr),
                    }),
                }
            }
            DeductionStep::Exact { hyp } => {
                let expr = self.hyps.get(hyp).ok_or_else(|| KernelError::HypothesisNotFound(hyp.clone()))?;
                if *expr == self.target {
                    self.status = ProofStatus::Proven;
                    Ok(None)
                } else {
                    Err(KernelError::TypeMismatch {
                        expected: format!("{:?}", self.target),
                        found: format!("{:?}", expr),
                    })
                }
            }
            DeductionStep::Reflexivity { term } => {
                let expected_eq = Expr::Eq(term.clone(), term.clone());
                if expected_eq == self.target {
                    self.status = ProofStatus::Proven;
                    Ok(None)
                } else {
                    Err(KernelError::TypeMismatch {
                        expected: format!("{:?}", self.target),
                        found: format!("{:?}", expected_eq),
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modus_ponens_success_and_failure() {
        let p = Expr::Prop("P".to_string());
        let q = Expr::Prop("Q".to_string());
        let r = Expr::Prop("R".to_string());
        let imp = Expr::Impl(Box::new(p.clone()), Box::new(q.clone()));

        let mut state = ProofState::new(
            vec![
                ("h0".to_string(), imp.clone()),
                ("h1".to_string(), p.clone()),
                ("h2".to_string(), r.clone()),
            ],
            q.clone(),
        );

        // Valid MP: (P -> Q) with P -> h3: Q
        let h3 = state
            .apply_step(&DeductionStep::ModusPonens {
                r#impl: "h0".to_string(),
                arg: "h1".to_string(),
            })
            .unwrap()
            .unwrap();
        assert_eq!(h3, "h3");
        assert_eq!(state.hyps.get("h3"), Some(&q));

        // Close goal via Exact(h3)
        assert!(state
            .apply_step(&DeductionStep::Exact {
                hyp: "h3".to_string()
            })
            .is_ok());
        assert_eq!(state.status, ProofStatus::Proven);

        // Attempting to step on proven state returns ProofAlreadyClosed
        let err = state.apply_step(&DeductionStep::Exact {
            hyp: "h1".to_string(),
        });
        assert_eq!(err, Err(KernelError::ProofAlreadyClosed));
    }

    #[test]
    fn test_mp_type_mismatch_and_not_found() {
        let p = Expr::Prop("P".to_string());
        let q = Expr::Prop("Q".to_string());
        let r = Expr::Prop("R".to_string());
        let imp = Expr::Impl(Box::new(p.clone()), Box::new(q.clone()));

        let mut state = ProofState::new(
            vec![("h0".to_string(), imp), ("h1".to_string(), r)],
            q.clone(),
        );

        // Mismatched argument
        let err = state.apply_step(&DeductionStep::ModusPonens {
            r#impl: "h0".to_string(),
            arg: "h1".to_string(),
        });
        assert!(matches!(err, Err(KernelError::TypeMismatch { .. })));

        // Non-existent hypothesis
        let err2 = state.apply_step(&DeductionStep::ModusPonens {
            r#impl: "h99".to_string(),
            arg: "h1".to_string(),
        });
        assert_eq!(err2, Err(KernelError::HypothesisNotFound("h99".to_string())));
    }

    #[test]
    fn test_reflexivity_rule() {
        let target = Expr::Eq("x".to_string(), "x".to_string());
        let mut state = ProofState::new(vec![], target);

        assert_eq!(state.status, ProofStatus::Open);
        let res = state.apply_step(&DeductionStep::Reflexivity {
            term: "x".to_string(),
        });
        assert_eq!(res, Ok(None));
        assert_eq!(state.status, ProofStatus::Proven);
    }
}
