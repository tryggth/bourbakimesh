//! Batch Mathlib Corpus Decompiler and Strategy Indexer.
//!
//! Converts formalized Lean 4 / CIC proof terms into verified game-semantic
//! dialogue strategies and exports topological curriculum metadata.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::ast::Term;
use crate::decompiler::{CICDecompiler, DecompileError};
use bourbaki_ir::{StrategyNode, StrategyTree};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors arising during corpus decompilation.
#[derive(Debug, Error)]
pub enum CorpusError {
    #[error("Decompilation error for theorem '{0}': {1}")]
    DecompilationFailed(String, #[source] DecompileError),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Bincode serialization error: {0}")]
    BincodeError(#[from] bincode::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Corpus validation failure: {0}")]
    ValidationFailed(String),
}

/// Raw exported theorem declaration from Lean 4 / Mathlib.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawTheorem {
    pub name: String,
    pub type_expr: String,
    pub prop_type: Term,
    pub proof_term: Term,
    #[serde(default)]
    pub dependency_count: usize,
}

/// Decompiled theorem containing a game-semantic StrategyTree and complexity metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompiledTheorem {
    pub name: String,
    pub type_expr: String,
    pub strategy: StrategyTree,
    pub trace_depth: usize,
    pub branch_count: usize,
    pub dependency_count: usize,
}

/// Serialized corpus dataset containing decompiled strategy trees.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorpusDataset {
    pub theorems: Vec<DecompiledTheorem>,
    pub total_nodes: usize,
}

impl CorpusDataset {
    /// Calculate summary statistics across stored strategies.
    pub fn calculate_stats(&mut self) {
        self.total_nodes = self
            .theorems
            .iter()
            .map(|t| Self::count_strategy_nodes(t.strategy.root.as_ref()))
            .sum();
    }

    fn count_strategy_nodes(node: Option<&StrategyNode>) -> usize {
        match node {
            None => 0,
            Some(n) => {
                let child_sum: usize = n
                    .children
                    .iter()
                    .map(|c| Self::count_strategy_nodes(Some(c)))
                    .sum();
                1 + child_sum
            }
        }
    }

    /// Serialize dataset to JSON string.
    pub fn to_json_string(&self) -> Result<String, CorpusError> {
        serde_json::to_string_pretty(self).map_err(CorpusError::from)
    }

    /// Deserialize dataset from JSON string.
    pub fn from_json_str(json_str: &str) -> Result<Self, CorpusError> {
        serde_json::from_str(json_str).map_err(CorpusError::from)
    }

    /// Save dataset to binary file using bincode.
    pub fn save_bincode(&self, path: &Path) -> Result<(), CorpusError> {
        let encoded: Vec<u8> = bincode::serialize(self)?;
        let mut file = File::create(path)?;
        file.write_all(&encoded)?;
        Ok(())
    }

    /// Load dataset from binary bincode file.
    pub fn load_bincode(path: &Path) -> Result<Self, CorpusError> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let dataset: Self = bincode::deserialize(&buffer)?;
        Ok(dataset)
    }
}

/// Batch corpus decompiler translating raw theorems into strategy trees.
pub struct CorpusDecompiler;

impl CorpusDecompiler {
    /// Decompile a single raw theorem declaration into a game-semantic StrategyTree.
    pub fn decompile_theorem(raw: &RawTheorem) -> Result<DecompiledTheorem, CorpusError> {
        let strategy = CICDecompiler::term_to_strategy(&raw.name, &raw.prop_type, &raw.proof_term)
            .map_err(|e| CorpusError::DecompilationFailed(raw.name.clone(), e))?;

        let (depth, branches) = Self::analyze_tree_topology(strategy.root.as_ref());

        Ok(DecompiledTheorem {
            name: raw.name.clone(),
            type_expr: raw.type_expr.clone(),
            strategy,
            trace_depth: depth,
            branch_count: branches,
            dependency_count: raw.dependency_count,
        })
    }

    /// Decompile a batch of raw theorems into a validated CorpusDataset.
    pub fn decompile_batch(raw_theorems: &[RawTheorem]) -> Result<CorpusDataset, CorpusError> {
        let mut dataset = CorpusDataset::default();

        for raw in raw_theorems {
            let decompiled = Self::decompile_theorem(raw)?;
            dataset.theorems.push(decompiled);
        }

        dataset.calculate_stats();
        Ok(dataset)
    }

    fn analyze_tree_topology(node: Option<&StrategyNode>) -> (usize, usize) {
        match node {
            None => (0, 0),
            Some(n) => {
                if n.children.is_empty() {
                    (1, 1)
                } else {
                    let mut max_child_depth = 0;
                    let mut total_branches = 0;
                    for child in &n.children {
                        let (d, b) = Self::analyze_tree_topology(Some(child));
                        max_child_depth = max_child_depth.max(d);
                        total_branches += b;
                    }
                    (1 + max_child_depth, total_branches)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Universe;

    #[test]
    fn test_batch_decompilation_and_stats() {
        let prop = Term::sort(Universe::Zero);
        let id_thm = RawTheorem {
            name: "id_prop".to_string(),
            type_expr: "A -> A".to_string(),
            prop_type: prop.clone(),
            proof_term: Term::lam("a", prop.clone(), Term::var("a")),
            dependency_count: 0,
        };

        let dataset = CorpusDecompiler::decompile_batch(&[id_thm]).unwrap();
        assert_eq!(dataset.theorems.len(), 1);
        assert_eq!(dataset.theorems[0].name, "id_prop");
        assert!(dataset.total_nodes > 0);
        assert!(dataset.theorems[0].trace_depth >= 1);
    }
}
