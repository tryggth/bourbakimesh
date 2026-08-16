//! Content-addressed proof DAG ledger implementation.

use crate::block::{BlockId, ProofBlock};
use std::collections::HashMap;
use thiserror::Error;

/// Errors arising during proof ledger operations and DAG integrity verification.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LedgerError {
    #[error("Block {0} already exists in the ledger")]
    DuplicateBlock(BlockId),

    #[error("Block {0} not found in the ledger")]
    BlockNotFound(BlockId),

    #[error("Invalid block hash: expected {expected}, computed {computed}")]
    InvalidBlockHash {
        expected: BlockId,
        computed: BlockId,
    },

    #[error("Referenced parent block {0} not found in DAG")]
    ParentNotFound(BlockId),

    #[error("Block {0} has uncertified status")]
    UncertifiedBlock(BlockId),
}

/// A decentralized content-addressed proof DAG ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProofLedger {
    blocks: HashMap<BlockId, ProofBlock>,
}

impl ProofLedger {
    /// Create an empty proof ledger.
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
        }
    }

    /// Insert a verified ProofBlock into the ledger, verifying its cryptographic hash and parents.
    pub fn insert_block(&mut self, block: ProofBlock) -> Result<BlockId, LedgerError> {
        let computed = block.compute_id();
        if block.id != computed {
            return Err(LedgerError::InvalidBlockHash {
                expected: block.id,
                computed,
            });
        }

        if self.blocks.contains_key(&block.id) {
            return Err(LedgerError::DuplicateBlock(block.id));
        }

        // Verify that all declared parent blocks exist
        for parent_id in &block.parent_ids {
            if !self.blocks.contains_key(parent_id) {
                return Err(LedgerError::ParentNotFound(*parent_id));
            }
        }

        let id = block.id;
        self.blocks.insert(id, block);
        Ok(id)
    }

    /// Retrieve a block by its cryptographic BlockId.
    pub fn get_block(&self, id: &BlockId) -> Option<&ProofBlock> {
        self.blocks.get(id)
    }

    /// True if the ledger contains the block.
    pub fn contains_block(&self, id: &BlockId) -> bool {
        self.blocks.contains_key(id)
    }

    /// Total number of blocks in the DAG.
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// True if the ledger contains zero blocks.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Verify the integrity of the entire DAG:
    /// - Every block's stored ID matches its recomputed content hash.
    /// - Every parent reference exists in the ledger.
    pub fn verify_chain_integrity(&self) -> Result<(), LedgerError> {
        for (id, block) in &self.blocks {
            let computed = block.compute_id();
            if *id != computed {
                return Err(LedgerError::InvalidBlockHash {
                    expected: *id,
                    computed,
                });
            }

            for parent_id in &block.parent_ids {
                if !self.blocks.contains_key(parent_id) {
                    return Err(LedgerError::ParentNotFound(*parent_id));
                }
            }
        }
        Ok(())
    }
}
