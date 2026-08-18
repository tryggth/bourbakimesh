use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::ast::{DeductionStep, Expr, ProofStatus, Term};

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

fn find_term_witness(pattern: &Term, actual: &Term, var: &str, witness: &mut Option<Term>) -> bool {
    if pattern == &Term::Var(var.to_string()) {
        if let Some(w) = witness {
            w == actual
        } else {
            *witness = Some(actual.clone());
            true
        }
    } else {
        match (pattern, actual) {
            (Term::Const(c1), Term::Const(c2)) => c1 == c2,
            (Term::Var(v1), Term::Var(v2)) => v1 == v2,
            (Term::Func(f1, a1), Term::Func(f2, a2)) => {
                if f1 != f2 || a1.len() != a2.len() {
                    return false;
                }
                a1.iter().zip(a2.iter()).all(|(p, a)| find_term_witness(p, a, var, witness))
            }
            _ => false,
        }
    }
}

fn find_expr_witness(pattern: &Expr, actual: &Expr, var: &str, witness: &mut Option<Term>) -> bool {
    match (pattern, actual) {
        (Expr::Pred(p1, terms1), Expr::Pred(p2, terms2)) => {
            if p1 != p2 || terms1.len() != terms2.len() {
                return false;
            }
            terms1.iter().zip(terms2.iter()).all(|(p, a)| find_term_witness(p, a, var, witness))
        }
        (Expr::Prop(p1), Expr::Prop(p2)) => p1 == p2,
        (Expr::And(l1, r1), Expr::And(l2, r2))
        | (Expr::Or(l1, r1), Expr::Or(l2, r2))
        | (Expr::Impl(l1, r1), Expr::Impl(l2, r2)) => {
            find_expr_witness(l1, l2, var, witness) && find_expr_witness(r1, r2, var, witness)
        }
        (Expr::Not(i1), Expr::Not(i2)) => find_expr_witness(i1, i2, var, witness),
        (Expr::False, Expr::False) => true,
        (Expr::Eq(t1a, t1b), Expr::Eq(t2a, t2b)) => {
            find_term_witness(t1a, t2a, var, witness) && find_term_witness(t1b, t2b, var, witness)
        }
        _ => false,
    }
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
            DeductionStep::ForallElim { hyp, term } => {
                let expr = self.hyps.get(hyp).ok_or_else(|| KernelError::HypothesisNotFound(hyp.clone()))?;
                match expr {
                    Expr::Forall { var, body } => {
                        let instantiated = body.substitute(var, term);
                        let new_id = format!("h{}", self.next_hyp_idx);
                        self.next_hyp_idx += 1;
                        self.hyps.insert(new_id.clone(), instantiated);
                        Ok(Some(new_id))
                    }
                    _ => Err(KernelError::TypeMismatch {
                        expected: "Forall { .. }".to_string(),
                        found: format!("{:?}", expr),
                    }),
                }
            }
            DeductionStep::ExistsIntro { hyp, var, body } => {
                let hyp_expr = self.hyps.get(hyp).ok_or_else(|| KernelError::HypothesisNotFound(hyp.clone()))?;
                let mut witness = None;
                if !find_expr_witness(body, hyp_expr, var, &mut witness) {
                    return Err(KernelError::TypeMismatch {
                        expected: format!("expression matching body {:?} with witness for {}", body, var),
                        found: format!("{:?}", hyp_expr),
                    });
                }
                if let Some(w) = witness {
                    let subst = body.substitute(var, &w);
                    if subst != *hyp_expr {
                        return Err(KernelError::TypeMismatch {
                            expected: format!("{:?}", hyp_expr),
                            found: format!("{:?}", subst),
                        });
                    }
                } else if *body != *hyp_expr {
                    return Err(KernelError::TypeMismatch {
                        expected: format!("{:?}", hyp_expr),
                        found: format!("{:?}", body),
                    });
                }

                let new_expr = Expr::Exists {
                    var: var.clone(),
                    body: Box::new(body.clone()),
                };
                let new_id = format!("h{}", self.next_hyp_idx);
                self.next_hyp_idx += 1;
                self.hyps.insert(new_id.clone(), new_expr);
                Ok(Some(new_id))
            }
            DeductionStep::ExistsElim { hyp_exists, hyp_impl } => {
                let ex_expr = self.hyps.get(hyp_exists).ok_or_else(|| KernelError::HypothesisNotFound(hyp_exists.clone()))?;
                let (ex_var, ex_body) = match ex_expr {
                    Expr::Exists { var, body } => (var, body),
                    _ => return Err(KernelError::TypeMismatch {
                        expected: "Exists { .. }".to_string(),
                        found: format!("{:?}", ex_expr),
                    }),
                };

                let impl_expr = self.hyps.get(hyp_impl).ok_or_else(|| KernelError::HypothesisNotFound(hyp_impl.clone()))?;
                let (ant, con, bound_var) = match impl_expr {
                    Expr::Forall { var: all_var, body: all_body } => {
                        match &**all_body {
                            Expr::Impl(ant, con) => (ant, con, Some(all_var.as_str())),
                            _ => return Err(KernelError::TypeMismatch {
                                expected: "Forall { body: Impl(..) }".to_string(),
                                found: format!("{:?}", impl_expr),
                            }),
                        }
                    }
                    Expr::Impl(ant, con) => (ant, con, None),
                    _ => return Err(KernelError::TypeMismatch {
                        expected: "Impl(..) or Forall { body: Impl(..) }".to_string(),
                        found: format!("{:?}", impl_expr),
                    }),
                };

                let ant_aligned = if let Some(bv) = bound_var {
                    if bv == ex_var {
                        (**ant).clone()
                    } else {
                        ant.substitute(bv, &Term::Var(ex_var.clone()))
                    }
                } else {
                    (**ant).clone()
                };

                if ant_aligned != **ex_body {
                    return Err(KernelError::TypeMismatch {
                        expected: format!("antecedent matching {:?}", ex_body),
                        found: format!("{:?}", ant),
                    });
                }

                if con.contains_free_var(ex_var) || bound_var.map_or(false, |bv| con.contains_free_var(bv)) {
                    return Err(KernelError::InvalidRuleApplication(
                        format!("Variable {} occurs free in conclusion {:?}", ex_var, con)
                    ));
                }

                let new_id = format!("h{}", self.next_hyp_idx);
                self.next_hyp_idx += 1;
                self.hyps.insert(new_id.clone(), (**con).clone());
                Ok(Some(new_id))
            }
            DeductionStep::Rewrite { eq_hyp, target_hyp } => {
                let eq_expr = self.hyps.get(eq_hyp).ok_or_else(|| KernelError::HypothesisNotFound(eq_hyp.clone()))?;
                let (t1, t2) = match eq_expr {
                    Expr::Eq(t1, t2) => (t1, t2),
                    _ => return Err(KernelError::TypeMismatch {
                        expected: "Eq(Term, Term)".to_string(),
                        found: format!("{:?}", eq_expr),
                    }),
                };

                let target_expr = self.hyps.get(target_hyp).ok_or_else(|| KernelError::HypothesisNotFound(target_hyp.clone()))?;
                let rewritten = target_expr.replace_term(t1, t2);

                let new_id = format!("h{}", self.next_hyp_idx);
                self.next_hyp_idx += 1;
                self.hyps.insert(new_id.clone(), rewritten);
                Ok(Some(new_id))
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
        let impl_expr = Expr::Impl(Box::new(p.clone()), Box::new(q.clone()));

        let mut state = ProofState::new(
            vec![
                ("h0".to_string(), impl_expr),
                ("h1".to_string(), p.clone()),
            ],
            q.clone(),
        );

        // Valid MP
        let res = state.apply_step(&DeductionStep::ModusPonens {
            r#impl: "h0".to_string(),
            arg: "h1".to_string(),
        });
        assert_eq!(res, Ok(Some("h2".to_string())));
        assert_eq!(state.hyps.get("h2"), Some(&q));

        // Mismatched argument (h2 is Q, but h0 expects P)
        let err = state.apply_step(&DeductionStep::ModusPonens {
            r#impl: "h0".to_string(),
            arg: "h2".to_string(),
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
        let target = Expr::Eq(Term::Var("x".to_string()), Term::Var("x".to_string()));
        let mut state = ProofState::new(vec![], target);

        assert_eq!(state.status, ProofStatus::Open);
        let res = state.apply_step(&DeductionStep::Reflexivity {
            term: Term::Var("x".to_string()),
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

        let h3 = state.apply_step(&DeductionStep::OrIntroL {
            hyp: "h0".to_string(),
            right: b.clone(),
        }).unwrap().unwrap();
        assert_eq!(h3, "h3");
        assert_eq!(state.hyps.get("h3"), Some(&Expr::Or(Box::new(a.clone()), Box::new(b.clone()))));

        let h4 = state.apply_step(&DeductionStep::OrElim {
            hyp_or: "h3".to_string(),
            left_impl: "h1".to_string(),
            right_impl: "h2".to_string(),
        }).unwrap().unwrap();
        assert_eq!(h4, "h4");
        assert_eq!(state.hyps.get("h4"), Some(&c));

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

        let h2 = state.apply_step(&DeductionStep::Contradiction {
            pos_hyp: "h0".to_string(),
            neg_hyp: "h1".to_string(),
        }).unwrap().unwrap();
        assert_eq!(h2, "h2");
        assert_eq!(state.hyps.get("h2"), Some(&Expr::False));

        let res = state.apply_step(&DeductionStep::FalseElim {
            hyp_false: "h2".to_string(),
        }).unwrap();
        assert_eq!(res, None);
        assert_eq!(state.status, ProofStatus::Proven);
    }
}
