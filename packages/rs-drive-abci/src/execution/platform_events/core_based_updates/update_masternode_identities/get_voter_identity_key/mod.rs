mod v0;

use dpp::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::platform_value::BinaryData;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    /// Retrieves a voter identity public key using the provided voting address and key ID.
    ///
    /// This function derives the voter identity public key and delegates to a version-specific method depending on the platform version.
    ///
    /// # Arguments
    ///
    /// * voting_address - The voting address.
    /// * key_id - The KeyID for the voter identity public key.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<IdentityPublicKey, Error> - Returns the derived voter identity public key if successful. Otherwise, returns an error.
    pub(crate) fn get_voter_identity_key(
        voting_address: [u8; 20],
        key_id: KeyID,
        platform_version: &PlatformVersion,
    ) -> Result<IdentityPublicKey, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates.masternode_updates
            .get_voter_identity_key
        {
            0 => Self::get_voter_identity_key_v0(voting_address, key_id),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "get_voter_identity_key".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}