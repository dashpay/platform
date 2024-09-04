use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn create_owner_identity_v0(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Identity, Error> {
        let owner_identifier = Self::get_owner_identifier(masternode)?;
        let mut identity = Identity::create_basic_identity(owner_identifier, platform_version)?;
        identity.add_public_keys([Self::get_owner_identity_withdrawal_key(
            masternode.state.payout_address,
            0,
            platform_version,
        )?]);
        Ok(identity)
    }

    fn get_owner_identifier(masternode: &MasternodeListItem) -> Result<Identifier, Error> {
        let masternode_identifier: [u8; 32] = masternode.pro_tx_hash.into();
        Ok(masternode_identifier.into())
    }
}
