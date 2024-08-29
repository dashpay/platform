use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::identifier::MasternodeIdentifiers;
use dpp::prelude::Identifier;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn get_operator_identifier_from_masternode_list_item_v0(
        masternode: &MasternodeListItem,
    ) -> Identifier {
        let pro_tx_hash = &masternode.pro_tx_hash.into();
        Identifier::create_operator_identifier(
            pro_tx_hash,
            masternode.state.pub_key_operator.as_slice(),
        )
    }
}
