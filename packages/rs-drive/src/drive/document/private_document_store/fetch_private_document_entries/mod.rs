mod v0;

use crate::drive::document::private_document_store::PrivateDocumentEntries;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// A page of the entries of the private document type `document_type` of the contract
    /// `contract_id`: from `start_position`, a chunk boundary of its store, for `count`
    /// entries, a whole chunk when `count` is 0 or larger (`private_document_entries_page_limit`).
    /// Each entry comes split into Drive's header and the client's entry, and the page carries
    /// the store's count. A type that is not private is refused, and so is every protocol
    /// version before 15, which has no private type.
    #[allow(clippy::too_many_arguments)]
    pub fn fetch_private_document_entries(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        start_position: u64,
        count: u32,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<PrivateDocumentEntries, Error> {
        match platform_version
            .drive
            .methods
            .document
            .fetch_private_document_entries
        {
            Some(0) => self.fetch_private_document_entries_v0(
                contract_id,
                document_type,
                start_position,
                count,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_private_document_entries".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_private_document_entries".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
