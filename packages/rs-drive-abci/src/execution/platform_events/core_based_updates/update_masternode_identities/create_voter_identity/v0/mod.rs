use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn create_voter_identity_v0(
        pro_tx_hash: &[u8; 32],
        voting_key: &[u8; 20],
        platform_version: &PlatformVersion,
    ) -> Result<Identity, Error> {
        let voting_identifier = Identifier::create_voter_identifier(pro_tx_hash, voting_key);
        let mut identity = Identity::create_basic_identity(voting_identifier, platform_version)?;
        identity.add_public_keys([Self::get_voter_identity_key(
            *voting_key,
            0,
            platform_version,
        )?]);
        Ok(identity)
    }

    pub(super) fn create_voter_identity_from_masternode_list_item_v0(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Identity, Error> {
        Self::create_voter_identity_v0(
            &masternode.pro_tx_hash.to_byte_array(),
            &masternode.state.voting_address,
            platform_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::platform::Platform;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem, MasternodeType};
    use dpp::identifier::MasternodeIdentifiers;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;
    use std::net::SocketAddr;
    use std::str::FromStr;

    fn make_masternode(pro_tx: [u8; 32], voting_address: [u8; 20]) -> MasternodeListItem {
        MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash: ProTxHash::from_byte_array(pro_tx),
            collateral_hash: Txid::from_byte_array([0u8; 32]),
            collateral_index: 0,
            collateral_address: [0u8; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: SocketAddr::from_str("1.2.3.4:1234").unwrap(),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: [0u8; 20],
                voting_address,
                payout_address: [0u8; 20],
                pub_key_operator: vec![0u8; 48],
                operator_payout_address: None,
                platform_node_id: None,
                platform_p2p_port: None,
                platform_http_port: None,
            },
        }
    }

    /// v0 voter identity id must equal `create_voter_identifier(pro_tx, voting_key)`.
    /// It must carry exactly one identity public key (the voter key).
    #[test]
    fn v0_voter_identity_id_matches_voter_identifier() {
        let platform_version = PlatformVersion::latest();
        let pro_tx = [0x42u8; 32];
        let voting_key = [0x21u8; 20];

        let identity = Platform::<MockCoreRPCLike>::create_voter_identity_v0(
            &pro_tx,
            &voting_key,
            platform_version,
        )
        .expect("v0 must succeed on valid inputs");

        let expected_id: Identifier = Identifier::create_voter_identifier(&pro_tx, &voting_key);
        assert_eq!(identity.id(), expected_id);
        assert_eq!(
            identity.public_keys().len(),
            1,
            "voter identity has exactly one key"
        );
    }

    /// The `from_masternode_list_item` flavor must derive the same identity as the
    /// direct constructor applied to pro_tx_hash + voting_address from the item.
    #[test]
    fn v0_from_masternode_list_item_agrees_with_direct_constructor() {
        let platform_version = PlatformVersion::latest();
        let pro_tx = [0x77u8; 32];
        let voting_key = [0x55u8; 20];
        let mn = make_masternode(pro_tx, voting_key);

        let direct = Platform::<MockCoreRPCLike>::create_voter_identity_v0(
            &pro_tx,
            &voting_key,
            platform_version,
        )
        .expect("direct");
        let via_mn =
            Platform::<MockCoreRPCLike>::create_voter_identity_from_masternode_list_item_v0(
                &mn,
                platform_version,
            )
            .expect("via mn");

        assert_eq!(direct.id(), via_mn.id());
    }

    /// Changing the voting key must change the resulting identifier.
    #[test]
    fn v0_distinct_voting_keys_yield_distinct_identities() {
        let platform_version = PlatformVersion::latest();
        let pro_tx = [0x01u8; 32];
        let a = Platform::<MockCoreRPCLike>::create_voter_identity_v0(
            &pro_tx,
            &[0xaau8; 20],
            platform_version,
        )
        .expect("a");
        let b = Platform::<MockCoreRPCLike>::create_voter_identity_v0(
            &pro_tx,
            &[0xbbu8; 20],
            platform_version,
        )
        .expect("b");
        assert_ne!(a.id(), b.id());
    }
}
