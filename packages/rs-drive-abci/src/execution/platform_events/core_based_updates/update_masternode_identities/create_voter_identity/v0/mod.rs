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
    pub(crate) fn create_voter_identity(
        pro_tx_hash: &[u8; 32],
        voting_key: &[u8; 20],
    ) -> Result<Identity, Error> {
        let voting_identifier = Self::get_voter_identifier(pro_tx_hash, voting_key)?;
        let mut identity = Self::create_basic_identity(voting_identifier);
        identity.add_public_keys([Self::get_voter_identity_key(*voting_key, 0)?]);
        Ok(identity)
    }

    pub(crate) fn create_voter_identity_from_masternode_list_item(
        &self,
        masternode: &MasternodeListItem,
    ) -> Result<Identity, Error> {
        Self::create_voter_identity(
            masternode.pro_tx_hash.as_inner(),
            &masternode.state.voting_address,
        )
    }
}
