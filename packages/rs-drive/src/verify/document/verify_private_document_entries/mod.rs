mod v0;

use crate::drive::document::private_document_store::PrivateDocumentEntries;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of a page of the entries of the private document type `document_type`
    /// of the contract `contract_id` (`prove_private_document_entries`), for the page from
    /// `start_position` to the end of its chunk (`private_document_entries_page_limit`, with
    /// the chunk power of the store element the proof binds). A type that is not private is
    /// refused, and so is every protocol version before 15.
    ///
    /// # Parameters
    /// * `proof`: The GroveDB proof of the page.
    /// * `contract_id`: The contract declaring the type.
    /// * `document_type`: The private document type whose store the proof is of.
    /// * `start_position`: The page's first position in the store, as requested.
    /// * `platform_version`: The protocol version selecting the method's generation.
    ///
    /// # Returns
    /// * `Ok((RootHash, PrivateDocumentEntries))`: the root hash the proof derives and the
    ///   page: its entries, each split into Drive's header and the client's entry, and the
    ///   store's count.
    /// * `Err(Error)` for a type that is not private, a protocol version without private
    ///   types, or a proof that does not verify.
    pub fn verify_private_document_entries(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        start_position: u64,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, PrivateDocumentEntries), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document
            .verify_private_document_entries
        {
            Some(0) => Self::verify_private_document_entries_v0(
                proof,
                contract_id,
                document_type,
                start_position,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_private_document_entries".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "verify_private_document_entries".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
