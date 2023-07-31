use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::dashcore::hashes::Hash;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn create_owner_identity_v0(
        &self,
        masternode: &MasternodeListItem,
    ) -> Result<Identity, Error> {
        let owner_identifier = Self::get_owner_identifier(masternode)?;
        let mut identity = Self::create_basic_identity(owner_identifier);
        identity.add_public_keys([Self::get_owner_identity_key(
            masternode.state.payout_address,
            0,
        )?]);
        Ok(identity)
    }

    fn get_owner_identifier(masternode: &MasternodeListItem) -> Result<[u8; 32], Error> {
        let masternode_identifier: [u8; 32] = masternode.pro_tx_hash.into_inner();
        Ok(masternode_identifier)
    }
}
