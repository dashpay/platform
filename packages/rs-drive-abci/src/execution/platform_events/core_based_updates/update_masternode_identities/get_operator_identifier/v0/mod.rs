use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
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

#[cfg(test)]
mod tests {
    use crate::platform_types::platform::Platform;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem, MasternodeType};
    use dpp::identifier::MasternodeIdentifiers;
    use dpp::prelude::Identifier;
    use std::net::SocketAddr;
    use std::str::FromStr;

    fn make_masternode(pro_tx: [u8; 32], pub_key_operator: Vec<u8>) -> MasternodeListItem {
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
                voting_address: [0u8; 20],
                payout_address: [0u8; 20],
                pub_key_operator,
                operator_payout_address: None,
                platform_node_id: None,
                platform_p2p_port: None,
                platform_http_port: None,
            },
        }
    }

    /// v0 operator identifier is `create_operator_identifier(pro_tx_hash, pub_key_operator)`.
    /// This test pins the contract so any silent drift (e.g. swapping args) is caught.
    #[test]
    fn v0_matches_create_operator_identifier() {
        let pro_tx = [0x22u8; 32];
        let pub_key_operator = vec![0x33u8; 48];
        let mn = make_masternode(pro_tx, pub_key_operator.clone());

        let got =
            Platform::<MockCoreRPCLike>::get_operator_identifier_from_masternode_list_item_v0(&mn);
        let expected = Identifier::create_operator_identifier(&pro_tx, pub_key_operator.as_slice());
        assert_eq!(got, expected);
    }

    /// Different operator public keys with the same pro_tx_hash must produce
    /// different identifiers (otherwise key rotation would collide).
    #[test]
    fn v0_distinct_pub_keys_produce_distinct_identifiers() {
        let pro_tx = [0x11u8; 32];
        let a = make_masternode(pro_tx, vec![0xaau8; 48]);
        let b = make_masternode(pro_tx, vec![0xbbu8; 48]);

        let id_a =
            Platform::<MockCoreRPCLike>::get_operator_identifier_from_masternode_list_item_v0(&a);
        let id_b =
            Platform::<MockCoreRPCLike>::get_operator_identifier_from_masternode_list_item_v0(&b);
        assert_ne!(id_a, id_b);
    }

    /// Different pro_tx_hash with the same operator key must also produce different
    /// identifiers (otherwise two distinct masternodes would alias).
    #[test]
    fn v0_distinct_pro_tx_hashes_produce_distinct_identifiers() {
        let pub_key = vec![0xccu8; 48];
        let a = make_masternode([0x01u8; 32], pub_key.clone());
        let b = make_masternode([0x02u8; 32], pub_key);

        let id_a =
            Platform::<MockCoreRPCLike>::get_operator_identifier_from_masternode_list_item_v0(&a);
        let id_b =
            Platform::<MockCoreRPCLike>::get_operator_identifier_from_masternode_list_item_v0(&b);
        assert_ne!(id_a, id_b);
    }

    /// Deterministic: same inputs must always produce the same identifier.
    #[test]
    fn v0_is_deterministic() {
        let mn = make_masternode([0x99u8; 32], vec![0x88u8; 48]);
        let first =
            Platform::<MockCoreRPCLike>::get_operator_identifier_from_masternode_list_item_v0(&mn);
        let second =
            Platform::<MockCoreRPCLike>::get_operator_identifier_from_masternode_list_item_v0(&mn);
        assert_eq!(first, second);
    }
}
