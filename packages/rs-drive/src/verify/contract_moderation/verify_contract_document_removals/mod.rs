mod v0;

use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalEntry, ContractDocumentRemovalsQuery,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of the records of the documents a contract's moderators deleted,
    /// within one document type, and returns the records the proof holds, in document id
    /// order.
    ///
    /// The verifier rebuilds the path query from `query`, so a read by ids proves each id
    /// named either present with its record or absent (an id the result does not hold has no
    /// record), and a page proves the records after its cursor up to its limit.
    ///
    /// # Parameters
    ///
    /// * `proof`: The proof, as `prove_contract_document_removals` produced it.
    /// * `contract_id`: The contract whose moderators deleted the documents.
    /// * `query`: The document type and the selection the proof was built for.
    /// * `verify_subset_of_proof`: Whether the proof may prove more than this query (verified as
    ///   a subset).
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((RootHash, Vec<ContractDocumentRemovalEntry>))` with the proof's root hash and the
    ///   proven records, in document id order; ids proved absent are left out.
    /// * `Err(Error)` when the method version is unknown, the proof fails verification, or a
    ///   proven record is malformed.
    pub fn verify_contract_document_removals(
        proof: &[u8],
        contract_id: Identifier,
        query: &ContractDocumentRemovalsQuery,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<ContractDocumentRemovalEntry>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_moderation
            .verify_contract_document_removals
        {
            0 => Self::verify_contract_document_removals_v0(
                proof,
                contract_id,
                query,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_document_removals".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
