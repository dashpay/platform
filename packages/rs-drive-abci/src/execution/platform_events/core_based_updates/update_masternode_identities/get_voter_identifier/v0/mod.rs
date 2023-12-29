use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;

use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn get_voter_identifier_v0(
        pro_tx_hash: &[u8; 32],
        voting_address: &[u8; 20],
        platform_version: &PlatformVersion,
    ) -> Result<[u8; 32], Error> {
        let voting_identifier =
            Self::hash_protxhash_with_key_data(pro_tx_hash, voting_address, platform_version)?;
        Ok(voting_identifier)
    }

    pub(super) fn get_voter_identifier_from_masternode_list_item_v0(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<[u8; 32], Error> {
        let pro_tx_hash = &masternode.pro_tx_hash.into();
        let voting_address = &masternode.state.voting_address;
        Self::get_voter_identifier(pro_tx_hash, voting_address, platform_version)
    }
}
