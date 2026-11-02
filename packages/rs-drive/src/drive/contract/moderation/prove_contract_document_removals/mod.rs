mod v0;

use crate::drive::contract::moderation::types::ContractDocumentRemovalsQuery;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the records of the documents a contract's moderators deleted, within one
    /// document type: the ones of the ids named (an id with no record is proved absent), or
    /// one page in document id order. The document type must be one whose removal tree
    /// exists; the caller checks that against the contract.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract whose moderators deleted the documents.
    /// * `query`: The document type and the selection: document ids, or a page.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` with the GroveDB proof of the selected records.
    /// * `Err(Error)` when the method version is unknown, the query names no ids, too many or a
    ///   repeated id, or has a limit of zero or above the maximum, or proving fails.
    pub fn prove_contract_document_removals(
        &self,
        contract_id: Identifier,
        query: &ContractDocumentRemovalsQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .prove_contract_document_removals
        {
            0 => self.prove_contract_document_removals_v0(
                contract_id,
                query,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_contract_document_removals".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
