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
    /// `contract_id`: from `start_position` to the end of the chunk of its store holding it
    /// (`private_document_entries_page_limit`). A type that is not private is refused, and so
    /// is every protocol version before 15, which has no private type.
    ///
    /// # Parameters
    /// * `contract_id`: The contract declaring the type.
    /// * `document_type`: The private document type whose store is read.
    /// * `start_position`: The page's first position in the store.
    /// * `transaction`: The GroveDB transaction to read in, `None` for committed state.
    /// * `platform_version`: The protocol version selecting the method's generation.
    ///
    /// # Returns
    /// * `Ok(PrivateDocumentEntries)`: the page's entries, each split into Drive's header and
    ///   the client's entry, and the store's count.
    /// * `Err(Error)` for a type that is not private, a protocol version without private
    ///   types, or a read that failed.
    pub fn fetch_private_document_entries(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        start_position: u64,
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
