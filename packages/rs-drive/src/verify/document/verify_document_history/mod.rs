mod v0;
mod v1;

#[cfg(all(test, feature = "server", feature = "verify"))]
mod tests;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::document_history_drive_query::{
    DocumentHistoryDriveQuery, DocumentHistoryDriveQueryExecutionResult,
};
use crate::verify::RootHash;
use bincode::{Decode, DecodeUntrusted, Encode};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

/// Standalone decode budget for the two-proof envelope. It only needs to sit
/// far above any realistic proof while bounding hostile allocations before
/// GroveDB verification runs.
const MAX_DOCUMENT_HISTORY_PROOF_DECODE_BYTES: usize = 16 * 1024 * 1024;

/// Proof envelope of a history page in the per-type history tree.
///
/// It carries two GroveDB proofs that answer queries GroveDB cannot merge:
/// the exact-key absence-proof read of the current pointer and the history
/// tree's count, and the offset-paginated read of the history tree itself.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub struct DocumentHistoryProof {
    /// Proves the current pointer and the raw history count-tree element.
    pub metadata_proof: Vec<u8>,
    /// Omitted only if metadata proves that the history tree is absent.
    pub entries_proof: Option<Vec<u8>>,
}

impl DocumentHistoryProof {
    /// Encodes the envelope as the proof bytes carried on the wire.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        bincode::encode_to_vec(
            self,
            bincode::config::standard()
                .with_big_endian()
                .with_no_limit(),
        )
        .map_err(|e| {
            Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                "cannot encode document history proof: {e}"
            ))))
        })
    }

    /// Decodes wire proof bytes, requiring the whole input to be consumed: a
    /// proof is produced only by a node, so anything else is not the proof
    /// this code believes it is reading.
    pub fn from_bytes(proof: &[u8]) -> Result<Self, Error> {
        if proof.len() > MAX_DOCUMENT_HISTORY_PROOF_DECODE_BYTES {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "document history proof exceeds the decoding limit".to_string(),
            )));
        }
        let config = bincode::config::standard()
            .with_big_endian()
            .with_limit::<MAX_DOCUMENT_HISTORY_PROOF_DECODE_BYTES>();
        let (envelope, consumed): (Self, usize) =
            bincode::decode_from_slice_untrusted(proof, config).map_err(|e| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "cannot decode document history proof: {e}"
                )))
            })?;
        if consumed != proof.len() {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "document history proof contains trailing bytes".to_string(),
            )));
        }
        Ok(envelope)
    }
}

/// Reject a GroveDB proof envelope older than the floor the protocol
/// version sets in `SystemLimits::minimum_grovedb_proof_envelope_version`.
fn require_supported_grovedb_proof(
    proof: &[u8],
    label: &'static str,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    let config = bincode::config::standard()
        .with_big_endian()
        .with_limit::<16>();
    let (version, _): (u32, usize) =
        bincode::decode_from_slice(proof, config).map_err(|error| {
            Error::Proof(ProofError::InvalidGroveDBProofEnvelope {
                proof: label,
                reason: error.to_string(),
            })
        })?;
    let minimum = platform_version
        .system_limits
        .minimum_grovedb_proof_envelope_version;
    if version < minimum {
        return Err(Error::Proof(
            ProofError::UnsupportedGroveDBProofEnvelopeVersion {
                proof: label,
                version,
                minimum,
                protocol_version: platform_version.protocol_version,
            },
        ));
    }
    Ok(())
}

impl Drive {
    /// Verifies a proved page of a historical document's history in the
    /// layout the protocol version stores.
    pub fn verify_document_history(
        query: &DocumentHistoryDriveQuery,
        proof: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, DocumentHistoryDriveQueryExecutionResult), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document
            .verify_document_history
        {
            0 => {
                require_supported_grovedb_proof(proof, "document history proof", platform_version)?;
                let (start_at_ms, limit) = query.legacy_read()?;
                let (root, revisions) = Drive::verify_document_history_v0(
                    proof,
                    query.contract_id,
                    &query.document_type_name,
                    document_type,
                    query.document_id,
                    start_at_ms,
                    limit,
                    None,
                    platform_version,
                )?;
                let history = DocumentHistoryDriveQueryExecutionResult::from_legacy(
                    revisions.unwrap_or_default(),
                )?;
                Ok((root, history))
            }
            1 => Drive::verify_document_history_v1(query, proof, document_type, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_document_history".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
