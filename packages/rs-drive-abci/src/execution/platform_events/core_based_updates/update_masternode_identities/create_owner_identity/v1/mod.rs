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
        let owner_identifier = Self::get_owner_identifier(masternode)?;
        let mut identity = Identity::create_basic_identity(owner_identifier, platform_version)?;
        identity.add_public_keys([
            Self::get_owner_identity_withdrawal_key(
                masternode.state.payout_address,
                0,
                platform_version,
            )?,
            Self::get_owner_identity_owner_key(
                masternode.state.owner_address,
                1,
                platform_version,
            )?,
        ]);
        Ok(identity)
    }
}
