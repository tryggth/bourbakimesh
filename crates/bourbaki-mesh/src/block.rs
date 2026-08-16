//! Cryptographic ProofBlock and BlockId data structures for the Bourbaki DAG ledger.

use hex::ToHex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::str::FromStr;

/// 32-byte SHA-256 cryptographic hash identifier for a proof block.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BlockId(pub [u8; 32]);

impl BlockId {
    /// Create a BlockId from a raw 32-byte array.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Return byte slice.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Parse BlockId from a 64-character hex string.
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self(bytes))
    }

    /// Encode BlockId as a 64-character hex string.
    pub fn to_hex(&self) -> String {
        self.0.encode_hex()
    }

    /// Genesis BlockId.
    pub fn genesis() -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"bourbaki_genesis_v1");
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        Self(bytes)
    }
}

impl fmt::Debug for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlockId({})", &self.to_hex()[..12])
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl FromStr for BlockId {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

impl Serialize for BlockId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for BlockId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

/// An immutable content-addressed node in the BourbakiMesh proof DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofBlock {
    /// SHA-256 cryptographic hash of block contents.
    pub id: BlockId,
    /// Parent block dependencies in the DAG.
    pub parent_ids: Vec<BlockId>,
    /// Formal theorem identifier.
    pub theorem_name: String,
    /// Theorem proposition statement (Lean 4 or CIC syntax).
    pub statement: String,
    /// Game-semantic arena winning strategy tree.
    pub strategy: Option<bourbaki_ir::StrategyTree>,
    /// Extracted Calculus of Inductive Constructions (CIC) proof term.
    pub cic_term: Option<bourbaki_kernel::Term>,
    /// Certified true if verified by reference Lean 4 kernel.
    pub certified: bool,
    /// Block creation timestamp in UNIX seconds.
    pub timestamp_secs: u64,
}

impl ProofBlock {
    /// Construct a new ProofBlock and compute its content-addressed BlockId.
    pub fn new(
        parent_ids: Vec<BlockId>,
        theorem_name: String,
        statement: String,
        strategy: Option<bourbaki_ir::StrategyTree>,
        cic_term: Option<bourbaki_kernel::Term>,
        certified: bool,
        timestamp_secs: u64,
    ) -> Self {
        let mut block = Self {
            id: BlockId::default(),
            parent_ids,
            theorem_name,
            statement,
            strategy,
            cic_term,
            certified,
            timestamp_secs,
        };
        block.id = block.compute_id();
        block
    }

    /// Compute the SHA-256 content hash of the block.
    pub fn compute_id(&self) -> BlockId {
        let mut hasher = Sha256::new();
        hasher.update((self.parent_ids.len() as u64).to_le_bytes());
        for pid in &self.parent_ids {
            hasher.update(pid.as_bytes());
        }
        hasher.update(self.theorem_name.as_bytes());
        hasher.update(self.statement.as_bytes());
        hasher.update([self.certified as u8]);
        hasher.update(self.timestamp_secs.to_le_bytes());

        if let Some(strat) = &self.strategy {
            if let Ok(encoded) = bincode::serialize(strat) {
                hasher.update(&encoded);
            }
        }
        if let Some(term) = &self.cic_term {
            if let Ok(encoded) = bincode::serialize(term) {
                hasher.update(&encoded);
            }
        }

        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        BlockId(bytes)
    }

    /// Construct a genesis block for a theory root.
    pub fn genesis(root_statement: &str) -> Self {
        Self::new(
            vec![],
            "Bourbaki.Genesis".into(),
            root_statement.into(),
            None,
            None,
            true,
            1700000000,
        )
    }
}
