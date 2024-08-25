use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[inline(always)]
    pub(super) fn create_operator_identity_v0(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Identity, Error> {
        let operator_identifier =
            Self::get_operator_identifier_from_masternode_list_item(masternode, platform_version)?;
        let mut identity = Identity::create_basic_identity(operator_identifier, platform_version)?;
        identity.add_public_keys(Self::get_operator_identity_keys(
            masternode.state.pub_key_operator.clone(),
            masternode.state.operator_payout_address,
            masternode.state.platform_node_id,
            platform_version,
        )?);

        Ok(identity)
    }
}
