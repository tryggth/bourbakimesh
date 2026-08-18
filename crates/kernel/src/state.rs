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
            DeductionStep::OrIntroL { hyp, right } => {
                let expr = self.hyps.get(hyp).ok_or_else(|| KernelError::HypothesisNotFound(hyp.clone()))?;
                let new_expr = Expr::Or(Box::new(expr.clone()), Box::new(right.clone()));
                let new_id = format!("h{}", self.next_hyp_idx);
                self.next_hyp_idx += 1;
                self.hyps.insert(new_id.clone(), new_expr);
                Ok(Some(new_id))
            }
            DeductionStep::OrIntroR { left, hyp } => {
                let expr = self.hyps.get(hyp).ok_or_else(|| KernelError::HypothesisNotFound(hyp.clone()))?;
                let new_expr = Expr::Or(Box::new(left.clone()), Box::new(expr.clone()));
                let new_id = format!("h{}", self.next_hyp_idx);
                self.next_hyp_idx += 1;
                self.hyps.insert(new_id.clone(), new_expr);
                Ok(Some(new_id))
            }
            DeductionStep::OrElim { hyp_or, left_impl, right_impl } => {
                let or_expr = self.hyps.get(hyp_or).ok_or_else(|| KernelError::HypothesisNotFound(hyp_or.clone()))?;
                let (a_or, b_or) = match or_expr {
                    Expr::Or(a, b) => (a, b),
                    _ => return Err(KernelError::TypeMismatch {
                        expected: "Or(_, _)".to_string(),
                        found: format!("{:?}", or_expr),
                    }),
                };

                let left_expr = self.hyps.get(left_impl).ok_or_else(|| KernelError::HypothesisNotFound(left_impl.clone()))?;
                let (a_impl, c_left) = match left_expr {
                    Expr::Impl(a, c) => (a, c),
                    _ => return Err(KernelError::TypeMismatch {
                        expected: "Impl(_, _)".to_string(),
                        found: format!("{:?}", left_expr),
                    }),
                };

                let right_expr = self.hyps.get(right_impl).ok_or_else(|| KernelError::HypothesisNotFound(right_impl.clone()))?;
                let (b_impl, c_right) = match right_expr {
                    Expr::Impl(b, c) => (b, c),
                    _ => return Err(KernelError::TypeMismatch {
                        expected: "Impl(_, _)".to_string(),
                        found: format!("{:?}", right_expr),
                    }),
                };

                if **a_or != **a_impl {
                    return Err(KernelError::TypeMismatch {
                        expected: format!("Impl({:?}, _)", a_or),
                        found: format!("{:?}", left_expr),
                    });
                }

                if **b_or != **b_impl {
                    return Err(KernelError::TypeMismatch {
                        expected: format!("Impl({:?}, _)", b_or),
                        found: format!("{:?}", right_expr),
                    });
                }

                if **c_left != **c_right {
                    return Err(KernelError::TypeMismatch {
                        expected: format!("matching conclusion {:?}", c_left),
                        found: format!("{:?}", c_right),
                    });
                }

                let new_id = format!("h{}", self.next_hyp_idx);
                self.next_hyp_idx += 1;
                self.hyps.insert(new_id.clone(), (**c_left).clone());
                Ok(Some(new_id))
            }
            DeductionStep::Contradiction { pos_hyp, neg_hyp } => {
                let pos_expr = self.hyps.get(pos_hyp).ok_or_else(|| KernelError::HypothesisNotFound(pos_hyp.clone()))?;
                let neg_expr = self.hyps.get(neg_hyp).ok_or_else(|| KernelError::HypothesisNotFound(neg_hyp.clone()))?;
                match neg_expr {
                    Expr::Not(inner) if **inner == *pos_expr => {
                        let new_id = format!("h{}", self.next_hyp_idx);
                        self.next_hyp_idx += 1;
                        self.hyps.insert(new_id.clone(), Expr::False);
                        Ok(Some(new_id))
                    }
                    _ => Err(KernelError::TypeMismatch {
                        expected: format!("Not({:?})", pos_expr),
                        found: format!("{:?}", neg_expr),
                    }),
                }
            }
            DeductionStep::FalseElim { hyp_false } => {
                let expr = self.hyps.get(hyp_false).ok_or_else(|| KernelError::HypothesisNotFound(hyp_false.clone()))?;
                if *expr == Expr::False {
                    self.status = ProofStatus::Proven;
                    Ok(None)
                } else {
                    Err(KernelError::TypeMismatch {
                        expected: "False".to_string(),
                        found: format!("{:?}", expr),
                    })
                }
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

    #[test]
    fn test_or_intro_and_elim() {
        let a = Expr::Prop("A".to_string());
        let b = Expr::Prop("B".to_string());
        let c = Expr::Prop("C".to_string());

        let mut state = ProofState::new(
            vec![
                ("h0".to_string(), a.clone()),
                ("h1".to_string(), Expr::Impl(Box::new(a.clone()), Box::new(c.clone()))),
                ("h2".to_string(), Expr::Impl(Box::new(b.clone()), Box::new(c.clone()))),
            ],
            c.clone(),
        );

        // OrIntroL: from h0: A create h3: A ∨ B
        let h3 = state.apply_step(&DeductionStep::OrIntroL {
            hyp: "h0".to_string(),
            right: b.clone(),
        }).unwrap().unwrap();
        assert_eq!(h3, "h3");
        assert_eq!(state.hyps.get("h3"), Some(&Expr::Or(Box::new(a.clone()), Box::new(b.clone()))));

        // OrElim: from h3: A ∨ B, h1: A -> C, h2: B -> C create h4: C
        let h4 = state.apply_step(&DeductionStep::OrElim {
            hyp_or: "h3".to_string(),
            left_impl: "h1".to_string(),
            right_impl: "h2".to_string(),
        }).unwrap().unwrap();
        assert_eq!(h4, "h4");
        assert_eq!(state.hyps.get("h4"), Some(&c));

        // Exact(h4) closes goal
        state.apply_step(&DeductionStep::Exact { hyp: "h4".to_string() }).unwrap();
        assert_eq!(state.status, ProofStatus::Proven);
    }

    #[test]
    fn test_contradiction_and_false_elim() {
        let p = Expr::Prop("P".to_string());
        let not_p = Expr::Not(Box::new(p.clone()));
        let target = Expr::Prop("Q".to_string());

        let mut state = ProofState::new(
            vec![
                ("h0".to_string(), p.clone()),
                ("h1".to_string(), not_p),
            ],
            target,
        );

        // Contradiction(h0, h1) -> h2: False
        let h2 = state.apply_step(&DeductionStep::Contradiction {
            pos_hyp: "h0".to_string(),
            neg_hyp: "h1".to_string(),
        }).unwrap().unwrap();
        assert_eq!(h2, "h2");
        assert_eq!(state.hyps.get("h2"), Some(&Expr::False));

        // FalseElim(h2) -> closes proof
        let res = state.apply_step(&DeductionStep::FalseElim {
            hyp_false: "h2".to_string(),
        }).unwrap();
        assert_eq!(res, None);
        assert_eq!(state.status, ProofStatus::Proven);
    }
}
