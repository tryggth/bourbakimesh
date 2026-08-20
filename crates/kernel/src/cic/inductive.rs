//! Inductive Type Declarations, Primitive Recursors, and ι-Reduction for CIC.
//!
//! Provides support for inductive families, multi-argument constructors,
//! dependent recursors, and deterministic computation via ι-reduction.

use crate::cic::expr::Expr;
use serde::{Deserialize, Serialize};

/// Declaration of an inductive constructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Constructor {
    pub name: String,
    pub ty: Expr,
}

/// Declaration of an inductive type family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InductiveType {
    pub name: String,
    pub num_params: usize,
    pub num_indices: usize,
    pub level_params: Vec<String>,
    pub ty: Expr,
    pub constructors: Vec<Constructor>,
}

/// Reduction rule for a constructor in a primitive recursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecursorRule {
    pub ctor_name: String,
    pub num_fields: usize,
    pub rhs: Expr,
}

/// Primitive recursor (eliminator) for an inductive type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recursor {
    pub name: String,
    pub ind_name: String,
    pub num_params: usize,
    pub num_indices: usize,
    pub num_motives: usize,
    pub num_minors: usize,
    pub rules: Vec<RecursorRule>,
}

impl InductiveType {
    /// Constructs the canonical Bool inductive type.
    pub fn bool_type() -> (Self, Recursor) {
        let type_0 = Expr::type_0();
        let bool_expr = Expr::const_term("Bool", vec![]);

        let constructors = vec![
            Constructor {
                name: "Bool.false".to_string(),
                ty: bool_expr.clone(),
            },
            Constructor {
                name: "Bool.true".to_string(),
                ty: bool_expr.clone(),
            },
        ];

        let ind = InductiveType {
            name: "Bool".to_string(),
            num_params: 0,
            num_indices: 0,
            level_params: vec![],
            ty: type_0,
            constructors,
        };

        let rec = Recursor {
            name: "Bool.rec".to_string(),
            ind_name: "Bool".to_string(),
            num_params: 0,
            num_indices: 0,
            num_motives: 1,
            num_minors: 2,
            rules: vec![
                RecursorRule {
                    ctor_name: "Bool.false".to_string(),
                    num_fields: 0,
                    rhs: Expr::BVar(0),
                },
                RecursorRule {
                    ctor_name: "Bool.true".to_string(),
                    num_fields: 0,
                    rhs: Expr::BVar(0),
                },
            ],
        };

        (ind, rec)
    }

    /// Constructs the canonical Nat inductive type.
    pub fn nat_type() -> (Self, Recursor) {
        let type_0 = Expr::type_0();
        let nat_expr = Expr::const_term("Nat", vec![]);

        let constructors = vec![
            Constructor {
                name: "Nat.zero".to_string(),
                ty: nat_expr.clone(),
            },
            Constructor {
                name: "Nat.succ".to_string(),
                ty: Expr::arrow(nat_expr.clone(), nat_expr.clone()),
            },
        ];

        let ind = InductiveType {
            name: "Nat".to_string(),
            num_params: 0,
            num_indices: 0,
            level_params: vec![],
            ty: type_0,
            constructors,
        };

        let rec = Recursor {
            name: "Nat.rec".to_string(),
            ind_name: "Nat".to_string(),
            num_params: 0,
            num_indices: 0,
            num_motives: 1,
            num_minors: 2,
            rules: vec![
                RecursorRule {
                    ctor_name: "Nat.zero".to_string(),
                    num_fields: 0,
                    rhs: Expr::BVar(0),
                },
                RecursorRule {
                    ctor_name: "Nat.succ".to_string(),
                    num_fields: 1,
                    rhs: Expr::BVar(0),
                },
            ],
        };

        (ind, rec)
    }

    /// Constructs the canonical And (conjunction) inductive type.
    pub fn and_type() -> (Self, Recursor) {
        let prop = Expr::prop();
        let and_ty = Expr::arrow(prop.clone(), Expr::arrow(prop.clone(), prop.clone()));

        // And.intro : ∀ (A : Prop) (B : Prop), A → B → And A B
        let intro_ty = Expr::forall_e(
            "A",
            prop.clone(),
            Expr::forall_e(
                "B",
                prop.clone(),
                Expr::forall_e(
                    "a",
                    Expr::BVar(1),
                    Expr::forall_e(
                        "b",
                        Expr::BVar(1),
                        Expr::mk_app(
                            Expr::const_term("And", vec![]),
                            vec![Expr::BVar(3), Expr::BVar(2)],
                        ),
                    ),
                ),
            ),
        );

        let ind = InductiveType {
            name: "And".to_string(),
            num_params: 2,
            num_indices: 0,
            level_params: vec![],
            ty: and_ty,
            constructors: vec![Constructor {
                name: "And.intro".to_string(),
                ty: intro_ty,
            }],
        };

        let rec = Recursor {
            name: "And.rec".to_string(),
            ind_name: "And".to_string(),
            num_params: 2,
            num_indices: 0,
            num_motives: 1,
            num_minors: 1,
            rules: vec![RecursorRule {
                ctor_name: "And.intro".to_string(),
                num_fields: 2,
                rhs: Expr::BVar(0),
            }],
        };

        (ind, rec)
    }

    /// Constructs the canonical Or (disjunction) inductive type.
    pub fn or_type() -> (Self, Recursor) {
        let prop = Expr::prop();
        let or_ty = Expr::arrow(prop.clone(), Expr::arrow(prop.clone(), prop.clone()));

        // Or.inl : ∀ (A : Prop) (B : Prop), A → Or A B
        let inl_ty = Expr::forall_e(
            "A",
            prop.clone(),
            Expr::forall_e(
                "B",
                prop.clone(),
                Expr::forall_e(
                    "a",
                    Expr::BVar(1),
                    Expr::mk_app(
                        Expr::const_term("Or", vec![]),
                        vec![Expr::BVar(2), Expr::BVar(1)],
                    ),
                ),
            ),
        );

        // Or.inr : ∀ (A : Prop) (B : Prop), B → Or A B
        let inr_ty = Expr::forall_e(
            "A",
            prop.clone(),
            Expr::forall_e(
                "B",
                prop.clone(),
                Expr::forall_e(
                    "b",
                    Expr::BVar(0),
                    Expr::mk_app(
                        Expr::const_term("Or", vec![]),
                        vec![Expr::BVar(2), Expr::BVar(1)],
                    ),
                ),
            ),
        );

        let ind = InductiveType {
            name: "Or".to_string(),
            num_params: 2,
            num_indices: 0,
            level_params: vec![],
            ty: or_ty,
            constructors: vec![
                Constructor {
                    name: "Or.inl".to_string(),
                    ty: inl_ty,
                },
                Constructor {
                    name: "Or.inr".to_string(),
                    ty: inr_ty,
                },
            ],
        };

        let rec = Recursor {
            name: "Or.rec".to_string(),
            ind_name: "Or".to_string(),
            num_params: 2,
            num_indices: 0,
            num_motives: 1,
            num_minors: 2,
            rules: vec![
                RecursorRule {
                    ctor_name: "Or.inl".to_string(),
                    num_fields: 1,
                    rhs: Expr::BVar(0),
                },
                RecursorRule {
                    ctor_name: "Or.inr".to_string(),
                    num_fields: 1,
                    rhs: Expr::BVar(0),
                },
            ],
        };

        (ind, rec)
    }

    /// Constructs the canonical Eq (Leibniz equality) inductive family.
    pub fn eq_type() -> (Self, Recursor) {
        let type_0 = Expr::type_0();
        let prop = Expr::prop();

        // Eq : ∀ (A : Type 0), A → A → Prop
        let eq_ty = Expr::forall_e(
            "A",
            type_0.clone(),
            Expr::forall_e(
                "x",
                Expr::BVar(0),
                Expr::forall_e("y", Expr::BVar(1), prop.clone()),
            ),
        );

        // Eq.refl : ∀ (A : Type 0) (x : A), Eq A x x
        let refl_ty = Expr::forall_e(
            "A",
            type_0.clone(),
            Expr::forall_e(
                "x",
                Expr::BVar(0),
                Expr::mk_app(
                    Expr::const_term("Eq", vec![]),
                    vec![Expr::BVar(1), Expr::BVar(0), Expr::BVar(0)],
                ),
            ),
        );

        let ind = InductiveType {
            name: "Eq".to_string(),
            num_params: 2,
            num_indices: 1,
            level_params: vec![],
            ty: eq_ty,
            constructors: vec![Constructor {
                name: "Eq.refl".to_string(),
                ty: refl_ty,
            }],
        };

        let rec = Recursor {
            name: "Eq.rec".to_string(),
            ind_name: "Eq".to_string(),
            num_params: 2,
            num_indices: 1,
            num_motives: 1,
            num_minors: 1,
            rules: vec![RecursorRule {
                ctor_name: "Eq.refl".to_string(),
                num_fields: 0,
                rhs: Expr::BVar(0),
            }],
        };

        (ind, rec)
    }

    /// Constructs the canonical List inductive family.
    pub fn list_type() -> (Self, Recursor) {
        let type_0 = Expr::type_0();
        let list_ty = Expr::arrow(type_0.clone(), type_0.clone());

        // List.nil : ∀ (A : Type 0), List A
        let nil_ty = Expr::forall_e(
            "A",
            type_0.clone(),
            Expr::mk_app(Expr::const_term("List", vec![]), vec![Expr::BVar(0)]),
        );

        // List.cons : ∀ (A : Type 0), A → List A → List A
        let cons_ty = Expr::forall_e(
            "A",
            type_0.clone(),
            Expr::arrow(
                Expr::BVar(0),
                Expr::arrow(
                    Expr::mk_app(Expr::const_term("List", vec![]), vec![Expr::BVar(1)]),
                    Expr::mk_app(Expr::const_term("List", vec![]), vec![Expr::BVar(2)]),
                ),
            ),
        );

        let ind = InductiveType {
            name: "List".to_string(),
            num_params: 1,
            num_indices: 0,
            level_params: vec![],
            ty: list_ty,
            constructors: vec![
                Constructor {
                    name: "List.nil".to_string(),
                    ty: nil_ty,
                },
                Constructor {
                    name: "List.cons".to_string(),
                    ty: cons_ty,
                },
            ],
        };

        let rec = Recursor {
            name: "List.rec".to_string(),
            ind_name: "List".to_string(),
            num_params: 1,
            num_indices: 0,
            num_motives: 1,
            num_minors: 2,
            rules: vec![
                RecursorRule {
                    ctor_name: "List.nil".to_string(),
                    num_fields: 0,
                    rhs: Expr::BVar(0),
                },
                RecursorRule {
                    ctor_name: "List.cons".to_string(),
                    num_fields: 2,
                    rhs: Expr::BVar(0),
                },
            ],
        };

        (ind, rec)
    }

    /// Constructs the False (empty) inductive type.
    pub fn false_type() -> (Self, Recursor) {
        let prop = Expr::prop();

        let ind = InductiveType {
            name: "False".to_string(),
            num_params: 0,
            num_indices: 0,
            level_params: vec![],
            ty: prop,
            constructors: vec![],
        };

        let rec = Recursor {
            name: "False.rec".to_string(),
            ind_name: "False".to_string(),
            num_params: 0,
            num_indices: 0,
            num_motives: 1,
            num_minors: 0,
            rules: vec![],
        };

        (ind, rec)
    }
}
