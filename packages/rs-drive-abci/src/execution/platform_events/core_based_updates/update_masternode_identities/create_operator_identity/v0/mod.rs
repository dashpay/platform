use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
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

#[cfg(test)]
mod tests {
    use crate::platform_types::platform::Platform;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem, MasternodeType};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::version::PlatformVersion;
    use std::net::SocketAddr;
    use std::str::FromStr;

    fn make_masternode(
        pro_tx: [u8; 32],
        operator_payout_address: Option<[u8; 20]>,
        platform_node_id: Option<[u8; 20]>,
    ) -> MasternodeListItem {
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
                pub_key_operator: vec![1u8; 48],
                operator_payout_address,
                platform_node_id,
                platform_p2p_port: None,
                platform_http_port: None,
            },
        }
    }

    /// Operator identity with neither optional address carries exactly one key
    /// (the BLS12_381 operator key).
    #[test]
    fn v0_operator_identity_minimal_one_key() {
        let platform_version = PlatformVersion::latest();
        let mn = make_masternode([0x10u8; 32], None, None);

        let identity =
            Platform::<MockCoreRPCLike>::create_operator_identity_v0(&mn, platform_version)
                .expect("must succeed");

        assert_eq!(identity.public_keys().len(), 1);
    }

    /// With both `operator_payout_address` and `platform_node_id` set, operator
    /// identity has exactly three keys.
    #[test]
    fn v0_operator_identity_full_three_keys() {
        let platform_version = PlatformVersion::latest();
        let mn = make_masternode([0x20u8; 32], Some([0xaau8; 20]), Some([0xbbu8; 20]));

        let identity =
            Platform::<MockCoreRPCLike>::create_operator_identity_v0(&mn, platform_version)
                .expect("must succeed");
        assert_eq!(identity.public_keys().len(), 3);
    }

    /// Different pro_tx_hash values must produce different operator identity ids
    /// even if other MN fields are identical.
    #[test]
    fn v0_distinct_pro_tx_hashes_yield_distinct_operator_identities() {
        let platform_version = PlatformVersion::latest();
        let a = make_masternode([0x01u8; 32], None, None);
        let b = make_masternode([0x02u8; 32], None, None);

        let ia = Platform::<MockCoreRPCLike>::create_operator_identity_v0(&a, platform_version)
            .expect("a");
        let ib = Platform::<MockCoreRPCLike>::create_operator_identity_v0(&b, platform_version)
            .expect("b");
        assert_ne!(ia.id(), ib.id());
    }

    /// Only payout_address set → 2 keys (operator + transfer).
    #[test]
    fn v0_operator_identity_with_only_payout() {
        let platform_version = PlatformVersion::latest();
        let mn = make_masternode([0x30u8; 32], Some([0xccu8; 20]), None);
        let identity =
            Platform::<MockCoreRPCLike>::create_operator_identity_v0(&mn, platform_version)
                .expect("must succeed");
        assert_eq!(identity.public_keys().len(), 2);
    }

    /// Only platform_node_id set → 2 keys (operator + node).
    #[test]
    fn v0_operator_identity_with_only_node_id() {
        let platform_version = PlatformVersion::latest();
        let mn = make_masternode([0x40u8; 32], None, Some([0xddu8; 20]));
        let identity =
            Platform::<MockCoreRPCLike>::create_operator_identity_v0(&mn, platform_version)
                .expect("must succeed");
        assert_eq!(identity.public_keys().len(), 2);
    }
}
