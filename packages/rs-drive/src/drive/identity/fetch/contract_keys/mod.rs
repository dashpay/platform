mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use std::collections::BTreeMap;

use dpp::identifier::Identifier;
use dpp::identity::Purpose;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches identities contract keys given identity ids, contract id, optional document type name and purposes
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `identity_ids` - The slice of identity ids to prove
    /// * `contract_id` - The contract id
    /// * `document_type_name` - The optional document type name
    /// * `purposes` - Key purposes
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a map with keys per purpose per identity id, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn fetch_identities_contract_keys(
        &self,
        identity_ids: &[[u8; 32]],
        contract_id: &[u8; 32],
        document_type_name: Option<String>,
        purposes: Vec<Purpose>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, BTreeMap<Purpose, Vec<u8>>>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .fetch
            .fetch_identities_contract_keys
        {
            0 => self.fetch_identities_contract_keys_v0(
                identity_ids,
                contract_id,
                document_type_name,
                purposes,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identities_contract_keys".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
