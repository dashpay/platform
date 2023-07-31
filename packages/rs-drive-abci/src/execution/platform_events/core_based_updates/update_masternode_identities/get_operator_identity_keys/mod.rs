mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Retrieves the operator identity public keys using provided operator parameters.
    ///
    /// This function derives the operator identity public keys and delegates to a version-specific method depending on the platform version.
    ///
    /// # Arguments
    ///
    /// * pub_key_operator - The public key operator.
    /// * operator_payout_address - Optional operator payout address.
    /// * platform_node_id - Optional platform node ID.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<Vec<IdentityPublicKey>, Error> - Returns the derived operator identity public keys if successful. Otherwise, returns an error.
    pub(crate) fn get_operator_identity_keys(
        pub_key_operator: Vec<u8>,
        operator_payout_address: Option<[u8; 20]>,
        platform_node_id: Option<[u8; 20]>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<IdentityPublicKey>, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .get_operator_identity_keys
        {
            0 => Self::get_operator_identity_keys_v0(
                pub_key_operator,
                operator_payout_address,
                platform_node_id,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "get_operator_identity_keys".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
