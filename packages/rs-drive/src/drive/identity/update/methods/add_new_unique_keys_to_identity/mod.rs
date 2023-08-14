mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::IdentityPublicKey;

use dpp::version::PlatformVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Add new unique keys to an identity. This function is version controlled.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity to which keys are to be added.
    /// * `keys_to_add` - The keys to be added.
    /// * `block_info` - The block information.
    /// * `apply` - Whether to apply the operations.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - The fee result if successful, or an error.
    pub fn add_new_unique_keys_to_identity(
        &self,
        identity_id: [u8; 32],
        keys_to_add: Vec<IdentityPublicKey>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .add_new_unique_keys_to_identity
        {
            0 => self.add_new_unique_keys_to_identity_v0(
                identity_id,
                keys_to_add,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_unique_keys_to_identity".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
