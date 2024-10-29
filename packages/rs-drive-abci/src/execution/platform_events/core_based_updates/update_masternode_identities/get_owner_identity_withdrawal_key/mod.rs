mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

use dpp::identity::{IdentityPublicKey, KeyID};

use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Retrieves an identity public key using the provided payout address and key ID.
    ///
    /// This function derives the identity public key and delegates to a version-specific method depending on the platform version.
    ///
    /// # Arguments
    ///
    /// * payout_address - The payout address for the owner.
    /// * key_id - The KeyID for the identity public key.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<IdentityPublicKey, Error> - Returns the derived identity public key if successful. Otherwise, returns an error.
    pub(crate) fn get_owner_identity_withdrawal_key(
        payout_address: [u8; 20],
        key_id: KeyID,
        platform_version: &PlatformVersion,
    ) -> Result<IdentityPublicKey, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .get_owner_identity_withdrawal_key
        {
            0 => Self::get_owner_identity_withdrawal_key_v0(payout_address, key_id),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "get_owner_identity_withdrawal_key".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
