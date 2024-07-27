use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::identifier::{Identifier, MasternodeIdentifiers};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn get_voter_identifier_from_masternode_list_item_v0(
        masternode: &MasternodeListItem,
    ) -> Identifier {
        let pro_tx_hash = &masternode.pro_tx_hash.into();
        let voting_address = &masternode.state.voting_address;
        Identifier::create_voter_identifier(pro_tx_hash, voting_address)
    }
}
