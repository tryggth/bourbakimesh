//! Strategy Extractor Compiler ($\mathcal{E}(\sigma) \to \text{Term}_{\text{CIC}}$).

use crate::ast::{MatchCase, Term, Universe};
use bourbaki_ir::{
    ArenaDialogue, ConjunctionBranch, LogicalPayload, MoveKind, PlayTrace, Polarity, StrategyNode,
    StrategyTree,
};
use thiserror::Error;

/// Extraction errors during strategy-to-CIC compilation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExtractionError {
    #[error("Empty strategy tree")]
    EmptyStrategy,

    #[error("Missing expected payload: {0}")]
    MissingPayload(String),

    #[error("Invalid strategy branching at step {0}")]
    InvalidBranching(usize),

    #[error("Unsound axiom discharge on hypothesis {0}")]
    UnsoundDischarge(usize),

    #[error("Strategy did not terminate with a Proponent winning move")]
    IncompleteProponentStrategy,
}

/// Extractor converting valid game-semantic arena strategies into sound CIC terms.
#[derive(Debug, Clone, Default)]
pub struct StrategyExtractor;

impl StrategyExtractor {
    /// Create a new strategy extractor instance.
    pub fn new() -> Self {
        Self
    }

    /// Compile a complete branched StrategyTree into a CIC Term.
    pub fn compile_strategy(strategy: &StrategyTree) -> Result<Term, ExtractionError> {
        let root = strategy
            .root
            .as_ref()
            .ok_or(ExtractionError::EmptyStrategy)?;
        Self::compile_node(root)
    }

    /// Compile a linear PlayTrace into a CIC Term.
    pub fn compile_trace(trace: &PlayTrace) -> Result<Term, ExtractionError> {
        if trace.is_empty() {
            return Err(ExtractionError::EmptyStrategy);
        }

        let last_move = trace.moves().last().unwrap();
        if last_move.player != Polarity::Proponent {
            return Err(ExtractionError::IncompleteProponentStrategy);
        }

        // Rule 1: Leaf / Discharge at the conclusion
        let mut current_term = Self::extract_leaf_payload(&last_move.payload)?;

        // Fold preceding Opponent and Proponent moves in reverse
        for m in trace.moves().iter().rev().skip(1) {
            match m.player {
                Polarity::Opponent => match &m.payload {
                    // Rule 2: Pi-Intro / Lambda binding for hypothesis attack
                    LogicalPayload::AttackHypothesis { hyp_id } => {
                        let hyp_name = format!("hyp_{}", hyp_id);
                        let hyp_type = Term::var(format!("A_{}", hyp_id));
                        current_term = Term::lam(hyp_name, hyp_type, current_term);
                    }
                    LogicalPayload::InstantiateUniversal { term_repr } => {
                        let (name, ty) = parse_binder(term_repr);
                        current_term = Term::lam(name, ty, current_term);
                    }
                    _ => {
                        if m.kind == MoveKind::Question {
                            let binder = format!("x_{}", m.id);
                            current_term = Term::lam(binder, Term::var("Prop"), current_term);
                        }
                    }
                },
                Polarity::Proponent => {
                    // Rule 3: Application / Modus Ponens argument
                    if let LogicalPayload::ProvideWitness { term_repr } = &m.payload {
                        current_term = Term::app(current_term, Term::var(term_repr.clone()));
                    }
                }
            }
        }

        Ok(current_term)
    }

    /// Extract a CIC proof term from an ArenaDialogue.
    pub fn extract(&self, dialogue: &ArenaDialogue) -> Result<Term, ExtractionError> {
        Self::compile_trace(dialogue.trace())
    }

    /// Recursively compile a StrategyNode into a CIC Term according to the 5 Extractor Rules.
    pub fn compile_node(node: &StrategyNode) -> Result<Term, ExtractionError> {
        // Base case: Leaf node (Proponent answer or discharge)
        if node.children.is_empty() {
            if node.current_move.player != Polarity::Proponent {
                return Err(ExtractionError::IncompleteProponentStrategy);
            }
            return Self::extract_leaf_payload(&node.current_move.payload);
        }

        // If the current node is an Opponent move, it binds a hypothesis / universal instantiation
        if node.current_move.player == Polarity::Opponent {
            match &node.current_move.payload {
                LogicalPayload::AttackHypothesis { hyp_id } => {
                    let child = node
                        .children
                        .first()
                        .ok_or(ExtractionError::IncompleteProponentStrategy)?;
                    let body_term = Self::compile_node(child)?;
                    let hyp_name = format!("hyp_{}", hyp_id);
                    let hyp_type = Term::var(format!("A_{}", hyp_id));
                    return Ok(Term::lam(hyp_name, hyp_type, body_term));
                }
                LogicalPayload::InstantiateUniversal { term_repr } => {
                    let child = node
                        .children
                        .first()
                        .ok_or(ExtractionError::IncompleteProponentStrategy)?;
                    let body_term = Self::compile_node(child)?;
                    let (name, ty) = parse_binder(term_repr);
                    return Ok(Term::lam(name, ty, body_term));
                }
                _ => {}
            }
        }

        // Check for Conjunction branches (Rule 5 / Conjunction intro)
        let has_conjunction_branch = node.children.iter().any(|c| {
            matches!(
                c.current_move.payload,
                LogicalPayload::AttackConjunction { .. }
            )
        });

        if has_conjunction_branch {
            let mut left_term = None;
            let mut right_term = None;

            for child in &node.children {
                if let LogicalPayload::AttackConjunction { branch } = &child.current_move.payload {
                    let prop_child = child
                        .children
                        .first()
                        .ok_or(ExtractionError::IncompleteProponentStrategy)?;
                    let branch_term = Self::compile_node(prop_child)?;
                    match branch {
                        ConjunctionBranch::Left => left_term = Some(branch_term),
                        ConjunctionBranch::Right => right_term = Some(branch_term),
                    }
                }
            }

            let left = left_term.ok_or_else(|| {
                ExtractionError::MissingPayload("Left conjunction branch missing".into())
            })?;
            let right = right_term.ok_or_else(|| {
                ExtractionError::MissingPayload("Right conjunction branch missing".into())
            })?;

            return Ok(Term::mk_app(
                Term::const_term("And.intro", vec![]),
                vec![left, right],
            ));
        }

        // Check for Inductive Case Demands (Rule 5: Pattern match)
        let has_inductive_cases = node.children.iter().any(|c| {
            matches!(
                c.current_move.payload,
                LogicalPayload::InductiveCaseDemand { .. }
            )
        });

        if has_inductive_cases {
            let mut match_cases = Vec::new();
            for child in &node.children {
                if let LogicalPayload::InductiveCaseDemand { constructor_idx } =
                    &child.current_move.payload
                {
                    let prop_child = child
                        .children
                        .first()
                        .ok_or(ExtractionError::IncompleteProponentStrategy)?;
                    let case_body = Self::compile_node(prop_child)?;
                    let constructor_name = format!("Constructor_{}", constructor_idx);
                    match_cases.push(MatchCase::new(
                        constructor_name,
                        vec![format!("x_{}", constructor_idx)],
                        case_body,
                    ));
                }
            }
            return Ok(Term::match_term(Term::var("v"), None, match_cases));
        }

        // Single child continuation
        if node.children.len() == 1 {
            let child = &node.children[0];
            return Self::compile_node(child);
        }

        // Fallback for general node
        let child = &node.children[0];
        Self::compile_node(child)
    }

    /// Extract a leaf payload into a variable, constant, or witness term (Rule 1).
    fn extract_leaf_payload(payload: &LogicalPayload) -> Result<Term, ExtractionError> {
        match payload {
            LogicalPayload::AxiomDischarge { premise_id } => {
                Ok(Term::var(format!("hyp_{}", premise_id)))
            }
            LogicalPayload::ProvideWitness { term_repr } => {
                let trimmed = term_repr.trim();
                if trimmed.starts_with('(') && trimmed.ends_with(')') {
                    let inner = &trimmed[1..trimmed.len() - 1];
                    let parts: Vec<&str> = inner.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let mut term = Term::var(parts[0]);
                        for &arg in &parts[1..] {
                            term = Term::app(term, Term::var(arg));
                        }
                        return Ok(term);
                    }
                } else {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let mut term = Term::var(parts[0]);
                        for &arg in &parts[1..] {
                            term = Term::app(term, Term::var(arg));
                        }
                        return Ok(term);
                    }
                }
                Ok(Term::var(term_repr.clone()))
            }
            LogicalPayload::RootGoal(goal) => Ok(Term::var(goal.clone())),
            other => Err(ExtractionError::MissingPayload(format!(
                "Cannot extract leaf from payload: {:?}",
                other
            ))),
        }
    }
}

/// Helper parsing a binder representation string like "A : Prop" or "h : A" into (name, Term).
fn parse_binder(repr: &str) -> (String, Term) {
    if let Some((name, ty_str)) = repr.split_once(':') {
        let name = name.trim().to_string();
        let ty_str = ty_str.trim();
        let ty = if ty_str == "Prop" {
            Term::sort(Universe::prop())
        } else if ty_str == "Type" {
            Term::sort(Universe::type_0())
        } else if let Some((dom, codom)) = ty_str.split_once("->") {
            Term::arrow(Term::var(dom.trim()), Term::var(codom.trim()))
        } else {
            Term::var(ty_str)
        };
        (name, ty)
    } else {
        (repr.trim().to_string(), Term::sort(Universe::prop()))
    }
}

/// Backwards compatible alias for TermExtractor.
pub type TermExtractor = StrategyExtractor;
