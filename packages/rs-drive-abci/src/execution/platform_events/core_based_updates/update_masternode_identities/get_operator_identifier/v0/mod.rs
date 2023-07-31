use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::dashcore::hashes::Hash;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn get_operator_identifier_v0(
        pro_tx_hash: &[u8; 32],
        pub_key_operator: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<[u8; 32], Error> {
        let operator_identifier =
            Self::hash_protxhash_with_key_data(pro_tx_hash, pub_key_operator, platform_version)?;
        Ok(operator_identifier)
    }

    pub(super) fn get_operator_identifier_from_masternode_list_item_v0(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<[u8; 32], Error> {
        let pro_tx_hash = &masternode.pro_tx_hash.into_inner();
        Self::get_operator_identifier_v0(
            pro_tx_hash,
            masternode.state.pub_key_operator.as_slice(),
            platform_version,
        )
    }
}
