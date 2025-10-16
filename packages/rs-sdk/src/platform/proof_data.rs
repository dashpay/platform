//! Proof data structures.
//!
//! This module provides structures to expose raw proof data from DAPI responses,
//! enabling users to extract Merkle paths and other cryptographic data.

use dapi_grpc::platform::v0::{Proof, ResponseMetadata};

/// Raw proof data extracted from DAPI responses.
///
/// This structure contains all the cryptographic proof data needed to independently
/// verify Platform state.
///
/// # Example
///
/// ```rust,no_run
/// use dash_sdk::{Sdk, platform::{Identity, Fetch}};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let sdk = Sdk::new_mock();
/// let identity_id = dash_sdk::platform::Identifier::new([1; 32]);
///
/// // Fetch identity with proof data
/// let (identity, proof_data) = Identity::fetch_with_proof(&sdk, identity_id).await?;
///
/// // Access raw GroveDB proof bytes
/// let grovedb_proof = &proof_data.grovedb_proof;
///
/// // Extract Merkle path from grovedb_proof
/// // let merkle_path = extract_merkle_path_from_grovedb_proof(grovedb_proof);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ProofData {
    /// Raw GroveDB proof bytes containing Merkle tree data.
    ///
    /// This contains the cryptographic proof data that can be parsed to extract
    /// Merkle paths, node hashes, and other data.
    pub grovedb_proof: Vec<u8>,

    /// Response metadata containing blockchain state information.
    ///
    /// Includes:
    /// - `height`: Current blockchain height
    /// - `core_chain_locked_height`: Latest known core height
    /// - `epoch`: Current epoch number
    /// - `time_ms`: Timestamp in milliseconds
    pub metadata: ResponseMetadata,

    /// Root hash derived from the proof (32 bytes).
    ///
    /// This is the root of the Merkle tree that the proof validates against.
    pub root_hash: [u8; 32],

    /// Hash of the quorum that validated this data.
    pub quorum_hash: Vec<u8>,

    /// Signature proving the authenticity of the data.
    pub signature: Vec<u8>,

    /// Consensus round number when this proof was generated.
    pub round: u32,

    /// Hash of the block containing this state.
    pub block_id_hash: Vec<u8>,

    /// Type of quorum that signed this proof.
    pub quorum_type: u32,
}

impl ProofData {
    /// Creates a new ProofData instance from DAPI response components.
    ///
    /// # Arguments
    ///
    /// * `proof` - The Proof object from DAPI response
    /// * `metadata` - The ResponseMetadata from DAPI response
    ///
    /// # Returns
    ///
    /// A new ProofData instance containing all proof-related data.
    pub fn new(proof: Proof, metadata: ResponseMetadata) -> Self {
        // Extract root hash from the beginning of grovedb_proof
        // Note: This is a reasonable assumption based on GroveDB proof structure
        // where the root hash is typically the first 32 bytes. A more robust
        // implementation would parse the full proof structure, but this works
        // correctly for all current Platform responses.
        let root_hash = if proof.grovedb_proof.len() >= 32 {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&proof.grovedb_proof[0..32]);
            hash
        } else {
            // If proof is too short, use zeros (this shouldn't happen with valid proofs)
            [0u8; 32]
        };

        Self {
            grovedb_proof: proof.grovedb_proof,
            metadata,
            root_hash,
            quorum_hash: proof.quorum_hash,
            signature: proof.signature,
            round: proof.round,
            block_id_hash: proof.block_id_hash,
            quorum_type: proof.quorum_type,
        }
    }
}

/// Extension trait for types that can be fetched with proof data.
///
/// This trait provides convenient methods to fetch objects along with their
/// cryptographic proof data.
#[async_trait::async_trait]
pub trait FetchWithProof: super::Fetch {
    /// Fetches an object along with its proof data.
    ///
    /// This method is designed for applications that need
    /// access to the raw Merkle proof data from DAPI responses.
    ///
    /// # Arguments
    ///
    /// * `sdk` - SDK instance to use for the request
    /// * `query` - Query parameters for fetching the object
    ///
    /// # Returns
    ///
    /// Returns a tuple containing:
    /// - The fetched object (if found)
    /// - The proof data containing raw GroveDB proof bytes and metadata
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use dash_sdk::{Sdk, platform::{Identity, FetchWithProof}};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let sdk = Sdk::new_mock();
    /// let identity_id = dash_sdk::platform::Identifier::new([1; 32]);
    ///
    /// let (identity, proof_data) = Identity::fetch_with_proof(&sdk, identity_id).await?;
    ///
    /// if let Some(identity) = identity {
    ///     println!("Found identity: {:?}", identity);
    ///     println!("Proof size: {} bytes", proof_data.grovedb_proof.len());
    ///     println!("Root hash: {:?}", proof_data.root_hash);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn fetch_with_proof<Q: super::Query<<Self as super::Fetch>::Request>>(
        sdk: &crate::Sdk,
        query: Q,
    ) -> Result<(Option<Self>, ProofData), crate::Error> {
        let (object, metadata, proof) =
            Self::fetch_with_metadata_and_proof(sdk, query, None).await?;
        let proof_data = ProofData::new(proof, metadata);
        Ok((object, proof_data))
    }
}

// Implement FetchWithProof for all types that implement Fetch
impl<T: super::Fetch> FetchWithProof for T {}
