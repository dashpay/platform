mod v0;

use std::collections::BTreeMap;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::identity::Purpose;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;
use dpp::identifier::Identifier;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Proves identities contract keys given identity ids and contract id.
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `identity_ids` - The slice of identity ids to prove
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of bytes representing the proved identities, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn get_identities_contract_keys(
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
            .fetch
            .identities_contract_keys
        {
            0 => self.get_identities_contract_keys_v0(
                identity_ids,
                contract_id,
                document_type_name,
                purposes,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_identities_contract_keys".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
