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
    /// of the contract `contract_id` (`prove_private_document_entries`), for the page that
    /// `start_position` and `count` select (`private_document_entries_page_limit`, which a
    /// misaligned start fails). Returns the root hash the proof derives and the page: its
    /// entries, each split into Drive's header and the client's entry, and the store's count.
    /// A type that is not private is refused, and so is every protocol version before 15.
    pub fn verify_private_document_entries(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        start_position: u64,
        count: u32,
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
                count,
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
