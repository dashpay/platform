use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn create_owner_identity_v1(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Identity, Error> {
        let payout_address = masternode.state.payout_address.ok_or_else(|| {
            Error::Execution(ExecutionError::DashCoreBadResponseError(format!(
                "masternode {} has no payout address",
                masternode.pro_tx_hash
            )))
        })?;
        let owner_address = masternode.state.owner_address.ok_or_else(|| {
            Error::Execution(ExecutionError::DashCoreBadResponseError(format!(
                "masternode {} has no owner address",
                masternode.pro_tx_hash
            )))
        })?;
        let owner_identifier = Self::get_owner_identifier(masternode)?;
        let mut identity = Identity::create_basic_identity(owner_identifier, platform_version)?;
        identity.add_public_keys([
            Self::get_owner_identity_withdrawal_key(payout_address, 0, platform_version)?,
            Self::get_owner_identity_owner_key(owner_address, 1, platform_version)?,
        ]);
        Ok(identity)
    }
}
