//! Content-addressed model weight chunking, Merkle tree verification, and P2P distribution protocol.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// Default P2P chunk size for model weights: 1 Megabyte (1,048,576 bytes).
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// GossipSub topic for model weight chunk announcements.
pub const TOPIC_CHUNKS: &str = "/bourbaki/1.0.0/chunks";

/// Cryptographic Merkle root hash identifying a model weight distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ModelRootHash(pub [u8; 32]);

impl ModelRootHash {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let mut arr = [0u8; 32];
        hex::decode_to_slice(s, &mut arr)?;
        Ok(Self(arr))
    }
}

impl fmt::Display for ModelRootHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Errors occurring during model chunking, validation, or reassembly.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ChunkError {
    #[error("Cannot chunk empty data")]
    EmptyData,

    #[error("Invalid chunk size: {0}")]
    InvalidChunkSize(usize),

    #[error("Invalid chunk index {index} for total {total}")]
    InvalidChunkIndex { index: usize, total: usize },

    #[error("Missing chunks: received {received} of {total} required")]
    MissingChunks { received: usize, total: usize },

    #[error("Chunk hash mismatch at index {index}: expected {expected}, got {computed}")]
    HashMismatch {
        index: usize,
        expected: String,
        computed: String,
    },

    #[error("Model root Merkle hash mismatch: expected {expected}, got {computed}")]
    RootMismatch { expected: String, computed: String },

    #[error("Corrupt or inconsistent chunk data: {0}")]
    CorruptData(String),
}

/// A discrete, content-addressed chunk of a model weight binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelChunk {
    /// Identifier or filename of the model checkpoint (e.g. "bourbaki_v2.pt").
    pub model_name: String,
    /// 0-indexed position of this chunk within the stream.
    pub chunk_index: usize,
    /// Total number of chunks comprising the full model.
    pub total_chunks: usize,
    /// SHA-256 hash of this specific chunk's data payload.
    pub chunk_hash: [u8; 32],
    /// Merkle root hash over all chunk hashes for the entire model.
    pub root_hash: ModelRootHash,
    /// Raw binary data of the chunk (up to 1MB).
    pub data: Vec<u8>,
}

impl ModelChunk {
    /// Verify that this chunk's data matches its declared chunk_hash.
    pub fn verify_hash(&self) -> bool {
        let computed = compute_sha256(&self.data);
        computed == self.chunk_hash
    }
}

/// Manifest listing all chunk hashes and total topology for a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkManifest {
    pub model_name: String,
    pub total_size_bytes: usize,
    pub chunk_size_bytes: usize,
    pub total_chunks: usize,
    pub root_hash: ModelRootHash,
    pub chunk_hashes: Vec<[u8; 32]>,
}

impl ChunkManifest {
    pub fn verify_merkle_root(&self) -> bool {
        let computed_root = compute_merkle_root(&self.chunk_hashes);
        computed_root == self.root_hash
    }
}

/// Broadcast gossip message for sharing chunks over libp2p.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkGossipMessage {
    pub chunk: ModelChunk,
    pub sender_peer_id: String,
}

/// Model weight chunking, verification, and assembly engine.
#[derive(Debug, Clone, Default)]
pub struct ModelChunker;

impl ModelChunker {
    /// Split raw binary data into chunks and produce a ChunkManifest.
    pub fn chunk_bytes(
        model_name: impl Into<String>,
        data: &[u8],
        chunk_size: Option<usize>,
    ) -> Result<(ChunkManifest, Vec<ModelChunk>), ChunkError> {
        if data.is_empty() {
            return Err(ChunkError::EmptyData);
        }
        let chunk_size = chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE);
        if chunk_size == 0 {
            return Err(ChunkError::InvalidChunkSize(0));
        }

        let name = model_name.into();
        let total_size = data.len();
        let total_chunks = (total_size + chunk_size - 1) / chunk_size;

        let mut raw_chunks = Vec::with_capacity(total_chunks);
        let mut chunk_hashes = Vec::with_capacity(total_chunks);

        for (idx, slice) in data.chunks(chunk_size).enumerate() {
            let hash = compute_sha256(slice);
            chunk_hashes.push(hash);
            raw_chunks.push((idx, slice.to_vec(), hash));
        }

        let root_hash = compute_merkle_root(&chunk_hashes);

        let manifest = ChunkManifest {
            model_name: name.clone(),
            total_size_bytes: total_size,
            chunk_size_bytes: chunk_size,
            total_chunks,
            root_hash,
            chunk_hashes: chunk_hashes.clone(),
        };

        let chunks = raw_chunks
            .into_iter()
            .map(|(chunk_index, chunk_data, chunk_hash)| ModelChunk {
                model_name: name.clone(),
                chunk_index,
                total_chunks,
                chunk_hash,
                root_hash,
                data: chunk_data,
            })
            .collect();

        Ok((manifest, chunks))
    }

    /// Reassemble and verify complete binary model from chunks.
    pub fn assemble_chunks(chunks: Vec<ModelChunk>) -> Result<Vec<u8>, ChunkError> {
        if chunks.is_empty() {
            return Err(ChunkError::EmptyData);
        }

        let total_chunks = chunks[0].total_chunks;
        let root_hash = chunks[0].root_hash;

        if chunks.len() < total_chunks {
            return Err(ChunkError::MissingChunks {
                received: chunks.len(),
                total: total_chunks,
            });
        }

        let mut ordered_chunks: HashMap<usize, ModelChunk> = HashMap::with_capacity(total_chunks);

        for chunk in chunks {
            if chunk.total_chunks != total_chunks {
                return Err(ChunkError::CorruptData(format!(
                    "Inconsistent total_chunks in chunk {}: expected {}, got {}",
                    chunk.chunk_index, total_chunks, chunk.total_chunks
                )));
            }
            if chunk.root_hash != root_hash {
                return Err(ChunkError::RootMismatch {
                    expected: root_hash.to_hex(),
                    computed: chunk.root_hash.to_hex(),
                });
            }
            if chunk.chunk_index >= total_chunks {
                return Err(ChunkError::InvalidChunkIndex {
                    index: chunk.chunk_index,
                    total: total_chunks,
                });
            }
            if !chunk.verify_hash() {
                let computed = hex::encode(compute_sha256(&chunk.data));
                return Err(ChunkError::HashMismatch {
                    index: chunk.chunk_index,
                    expected: hex::encode(chunk.chunk_hash),
                    computed,
                });
            }
            ordered_chunks.insert(chunk.chunk_index, chunk);
        }

        if ordered_chunks.len() != total_chunks {
            return Err(ChunkError::MissingChunks {
                received: ordered_chunks.len(),
                total: total_chunks,
            });
        }

        let mut assembled = Vec::new();
        let mut chunk_hashes = Vec::with_capacity(total_chunks);

        for idx in 0..total_chunks {
            let chunk = ordered_chunks.get(&idx).ok_or(ChunkError::MissingChunks {
                received: ordered_chunks.len(),
                total: total_chunks,
            })?;
            chunk_hashes.push(chunk.chunk_hash);
            assembled.extend_from_slice(&chunk.data);
        }

        // Verify Merkle root matches
        let computed_root = compute_merkle_root(&chunk_hashes);
        if computed_root != root_hash {
            return Err(ChunkError::RootMismatch {
                expected: root_hash.to_hex(),
                computed: computed_root.to_hex(),
            });
        }

        Ok(assembled)
    }
}

/// Compute SHA-256 of byte slice.
pub fn compute_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute cryptographic Merkle root of a list of chunk hashes.
pub fn compute_merkle_root(hashes: &[[u8; 32]]) -> ModelRootHash {
    if hashes.is_empty() {
        return ModelRootHash([0u8; 32]);
    }
    if hashes.len() == 1 {
        return ModelRootHash(hashes[0]);
    }

    let mut current_layer: Vec<[u8; 32]> = hashes.to_vec();

    while current_layer.len() > 1 {
        let mut next_layer = Vec::with_capacity((current_layer.len() + 1) / 2);
        for chunk in current_layer.chunks(2) {
            if chunk.len() == 2 {
                let mut hasher = Sha256::new();
                hasher.update(&chunk[0]);
                hasher.update(&chunk[1]);
                next_layer.push(hasher.finalize().into());
            } else {
                let mut hasher = Sha256::new();
                hasher.update(&chunk[0]);
                hasher.update(&chunk[0]);
                next_layer.push(hasher.finalize().into());
            }
        }
        current_layer = next_layer;
    }

    ModelRootHash(current_layer[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_and_reassemble_roundtrip() {
        let test_data =
            b"This is a test model weight binary payload that spans multiple chunks.".repeat(500);
        let chunk_size = 256; // 256 bytes per chunk for testing

        let (manifest, chunks) =
            ModelChunker::chunk_bytes("test_model.pt", &test_data, Some(chunk_size)).unwrap();

        assert_eq!(manifest.model_name, "test_model.pt");
        assert_eq!(manifest.total_size_bytes, test_data.len());
        assert!(manifest.total_chunks > 1);
        assert_eq!(chunks.len(), manifest.total_chunks);
        assert!(manifest.verify_merkle_root());

        // Verify individual chunks
        for chunk in &chunks {
            assert!(chunk.verify_hash());
            assert_eq!(chunk.root_hash, manifest.root_hash);
        }

        // Reassemble in order
        let assembled = ModelChunker::assemble_chunks(chunks.clone()).unwrap();
        assert_eq!(assembled, test_data);

        // Reassemble out-of-order
        let mut shuffled = chunks.clone();
        shuffled.reverse();
        let assembled_shuffled = ModelChunker::assemble_chunks(shuffled).unwrap();
        assert_eq!(assembled_shuffled, test_data);
    }

    #[test]
    fn test_corrupt_chunk_detection() {
        let test_data = b"Some important model parameters".repeat(20);
        let (_manifest, mut chunks) =
            ModelChunker::chunk_bytes("test_model.pt", &test_data, Some(64)).unwrap();

        // Corrupt first chunk
        chunks[0].data[0] ^= 0xFF;

        let err = ModelChunker::assemble_chunks(chunks).unwrap_err();
        assert!(matches!(err, ChunkError::HashMismatch { .. }));
    }

    #[test]
    fn test_missing_chunks_detection() {
        let test_data = b"Model data for missing chunk test".repeat(10);
        let (_, mut chunks) =
            ModelChunker::chunk_bytes("test_model.pt", &test_data, Some(32)).unwrap();

        chunks.pop(); // Remove last chunk

        let err = ModelChunker::assemble_chunks(chunks).unwrap_err();
        assert!(matches!(err, ChunkError::MissingChunks { .. }));
    }
}
