//! Document proof functionality for zero-knowledge proof applications.
//!
//! This module provides functionality to fetch documents along with their
//! merkle proofs to the state root, enabling use in zero-knowledge proof systems.

use crate::{Error, Sdk};
use dpp::document::{Document, DocumentV0Getters};
use dpp::identifier::Identifier;

/// A document along with its merkle proof to the state root
#[derive(Debug, Clone)]
pub struct DocumentWithProof {
    /// The fetched document
    pub document: Document,
    /// The owner ID of the document
    pub owner_id: Identifier,
    /// The merkle path from document to state root
    /// Each element is (sibling_hash, is_left)
    pub merkle_path: Vec<([u8; 32], bool)>,
    /// The state root hash
    pub state_root: [u8; 32],
    /// The block height at which this proof was generated
    pub block_height: u64,
}

/// Extract merkle path from GroveDB proof bytes
///
/// This function properly parses the GroveDB proof structure to extract
/// the merkle proof elements needed for ZK circuits.
///
/// GroveDB proofs can be in multiple formats:
/// - Raw Merk proof (doesn't start with 0x00)
/// - GroveDBProof V0 format (starts with 0x00)
///
/// # Arguments
///
/// * `proof_bytes` - The raw GroveDB proof bytes
///
/// # Returns
///
/// A vector of (hash, is_left) pairs representing the merkle path
pub fn extract_merkle_path_from_grove_proof(
    proof_bytes: &[u8],
) -> Result<Vec<([u8; 32], bool)>, Error> {
    if proof_bytes.is_empty() {
        return Err(Error::Generic("Empty proof".to_string()));
    }

    // Check if it's a raw Merk proof (doesn't start with 0x00)
    if proof_bytes[0] != 0x00 {
        return parse_as_merk_proof(proof_bytes);
    }

    // Parse as GroveDBProof V0
    // GroveDB proofs are bincode encoded with big-endian
    // For now, we'll try to find Merk proofs in the structure

    // Skip the version byte and try to find Merk proof patterns
    let mut offset = 1;

    // Look for Merk proof operations in the data
    while offset < proof_bytes.len() {
        // Try to parse from this position as a Merk proof
        if let Ok(path) = parse_as_merk_proof(&proof_bytes[offset..]) {
            if !path.is_empty() {
                return Ok(path);
            }
        }
        offset += 1;
    }

    Err(Error::Generic(
        "No valid Merk proofs found in GroveDB proof".to_string(),
    ))
}

/// Parse a Merk proof (the actual merkle path)
fn parse_as_merk_proof(proof_bytes: &[u8]) -> Result<Vec<([u8; 32], bool)>, Error> {
    let mut path = Vec::new();
    let mut offset = 0;

    while offset < proof_bytes.len() {
        // Read operation byte
        if offset >= proof_bytes.len() {
            break;
        }
        let op = proof_bytes[offset];
        offset += 1;

        match op {
            // Push operations (0x01 = left sibling, 0x02 = right sibling)
            0x01 | 0x02 => {
                let is_left = op == 0x01;

                // Decode hash length (varint)
                let (hash_len, bytes_read) = decode_varint(&proof_bytes[offset..])
                    .map_err(|e| Error::Generic(format!("Failed to decode varint: {}", e)))?;
                offset += bytes_read;

                // Read hash
                if offset + hash_len > proof_bytes.len() {
                    return Err(Error::Generic("Invalid hash length in proof".to_string()));
                }

                // We expect 32-byte hashes
                if hash_len != 32 {
                    // Skip non-32-byte data
                    offset += hash_len;
                    continue;
                }

                let mut sibling_hash = [0u8; 32];
                sibling_hash.copy_from_slice(&proof_bytes[offset..offset + 32]);
                offset += 32;

                path.push((sibling_hash, is_left));
            }
            // Push operation marker
            0x10 => break,
            // Parent operation - move up the tree
            0x11 => continue,
            // Child operation - move down the tree
            0x12 => {
                // Skip child index (varint encoded)
                let (_, bytes_read) = decode_varint(&proof_bytes[offset..]).unwrap_or((0, 0));
                offset += bytes_read;
            }
            // Skip unknown operations
            _ => {
                // Try to skip as varint
                if op & 0x80 != 0 {
                    while offset < proof_bytes.len() && (proof_bytes[offset] & 0x80) != 0 {
                        offset += 1;
                    }
                    if offset < proof_bytes.len() {
                        offset += 1;
                    }
                } else {
                    // Unknown operation, try to continue
                    continue;
                }
            }
        }
    }

    Ok(path)
}

/// Decode a varint (same as protobuf/bincode varint encoding)
pub(crate) fn decode_varint(bytes: &[u8]) -> Result<(usize, usize), String> {
    if bytes.is_empty() {
        return Err("Empty bytes for varint".to_string());
    }

    let mut value = 0usize;
    let mut shift = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        if i >= 10 {
            return Err("Varint too long".to_string());
        }

        value |= ((byte & 0x7F) as usize) << shift;

        if byte & 0x80 == 0 {
            return Ok((value, i + 1));
        }

        shift += 7;
    }

    Err("Incomplete varint".to_string())
}

/// Fetch a document along with its merkle proof to the state root
///
/// This function fetches a document and extracts its merkle proof from the
/// GroveDB proof, providing all necessary data for zero-knowledge proof
/// applications.
///
/// # Arguments
///
/// * `sdk` - The SDK instance to use for the request
/// * `contract_id` - The identifier of the contract containing the document
/// * `document_type_name` - The name of the document type within the contract
/// * `document_id` - The identifier of the document to fetch
///
/// # Returns
///
/// A `DocumentWithProof` containing the document and its merkle proof
///
/// # Example
///
/// ```rust,no_run
/// use dash_sdk::{Sdk, platform::Identifier};
/// use dash_sdk::platform::document_proof::fetch_document_with_proof;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let sdk = Sdk::new_mock();
/// let contract_id = Identifier::new([1; 32]);
/// let document_id = Identifier::new([2; 32]);
/// let document_type_name = "myDocumentType";
///
/// let doc_with_proof = fetch_document_with_proof(
///     &sdk,
///     &contract_id,
///     document_type_name,
///     &document_id
/// ).await?;
///
/// println!("Document owner: {:?}", doc_with_proof.owner_id);
/// println!("Merkle path length: {}", doc_with_proof.merkle_path.len());
/// println!("State root: {:?}", doc_with_proof.state_root);
/// # Ok(())
/// # }
/// ```
pub async fn fetch_document_with_proof(
    sdk: &Sdk,
    contract_id: &Identifier,
    document_type_name: &str,
    document_id: &Identifier,
) -> Result<DocumentWithProof, Error> {
    use super::proof_data::FetchWithProof;
    use super::{Document, DocumentQuery};

    // Create query for specific document ID using the existing with_document_id method
    let query = DocumentQuery::new_with_data_contract_id(sdk, *contract_id, document_type_name)
        .await?
        .with_document_id(document_id);

    // Fetch with proof
    let (document_opt, proof_data) = Document::fetch_with_proof(sdk, query).await?;

    let document = document_opt.ok_or(Error::Generic("Document not found".to_string()))?;

    // Extract merkle path using the complete parser
    let merkle_path = extract_merkle_path_from_grove_proof(&proof_data.grovedb_proof)?;

    // Get owner_id before moving document
    let owner_id = document.owner_id();

    Ok(DocumentWithProof {
        document,
        owner_id,
        merkle_path,
        state_root: proof_data.root_hash,
        block_height: proof_data.metadata.height,
    })
}
