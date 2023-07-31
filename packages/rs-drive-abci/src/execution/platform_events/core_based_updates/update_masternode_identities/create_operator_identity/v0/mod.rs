use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn create_operator_identity_v0(
        &self,
        masternode: &MasternodeListItem,
    ) -> Result<Identity, Error> {
        let operator_identifier =
            Self::get_operator_identifier_from_masternode_list_item(masternode)?;
        let mut identity = Self::create_basic_identity(operator_identifier);
        identity.add_public_keys(self.get_operator_identity_keys(
            masternode.state.pub_key_operator.clone(),
            masternode.state.operator_payout_address,
            masternode.state.platform_node_id,
        )?);

        Ok(identity)
    }
}
