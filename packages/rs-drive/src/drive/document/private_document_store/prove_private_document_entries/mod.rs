mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;

impl Drive {
    /// The proof of the page `fetch_private_document_entries` reads: GroveDB's position range
    /// proof of the type's store, which binds the entries, and the store's count, to the root
    /// hash. `verify_private_document_entries` checks it. A type that is not private is
    /// refused, and so is every protocol version before 15.
    ///
    /// # Parameters
    /// * `contract_id`: The contract declaring the type.
    /// * `document_type`: The private document type whose store is proved.
    /// * `start_position`: The page's first position in the store.
    /// * `platform_version`: The protocol version selecting the method's generation.
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)`: the proof, of committed state.
    /// * `Err(Error)` for a type that is not private, a protocol version without private
    ///   types, or a proof that failed.
    pub fn prove_private_document_entries(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        start_position: u64,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .prove_private_document_entries
        {
            Some(0) => self.prove_private_document_entries_v0(
                contract_id,
                document_type,
                start_position,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_private_document_entries".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "prove_private_document_entries".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
