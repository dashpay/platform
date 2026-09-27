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
