use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn create_owner_identity_v0(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Identity, Error> {
        let owner_identifier = Self::get_owner_identifier(masternode)?;
        let mut identity = Identity::create_basic_identity(owner_identifier, platform_version)?;
        identity.add_public_keys([Self::get_owner_identity_withdrawal_key(
            masternode.state.payout_address,
            0,
            platform_version,
        )?]);
        Ok(identity)
    }

    pub(crate) fn get_owner_identifier(
        masternode: &MasternodeListItem,
    ) -> Result<Identifier, Error> {
        let masternode_identifier: [u8; 32] = masternode.pro_tx_hash.into();
        Ok(masternode_identifier.into())
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
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;
    use std::net::SocketAddr;
    use std::str::FromStr;

    fn make_masternode(pro_tx: [u8; 32], payout_address: [u8; 20]) -> MasternodeListItem {
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
                payout_address,
                pub_key_operator: vec![0u8; 48],
                operator_payout_address: None,
                platform_node_id: None,
                platform_p2p_port: None,
                platform_http_port: None,
            },
        }
    }

    /// v0 owner identity id is the pro_tx_hash cast to Identifier. Any refactor that
    /// alters this contract (e.g. hashing the pro_tx) would break the identity mapping.
    #[test]
    fn v0_owner_identifier_equals_pro_tx_hash() {
        let pro_tx = [0x33u8; 32];
        let mn = make_masternode(pro_tx, [0u8; 20]);

        let id = Platform::<MockCoreRPCLike>::get_owner_identifier(&mn)
            .expect("owner identifier must not fail");
        let expected: Identifier = pro_tx.into();
        assert_eq!(id, expected);
    }

    /// v0 owner identity carries exactly one identity public key (the withdrawal key),
    /// and its id must match `get_owner_identifier`.
    #[test]
    fn v0_create_owner_identity_has_single_key_and_matches_identifier() {
        let platform_version = PlatformVersion::latest();
        let pro_tx = [0xabu8; 32];
        let mn = make_masternode(pro_tx, [0x12u8; 20]);

        let identity = Platform::<MockCoreRPCLike>::create_owner_identity_v0(&mn, platform_version)
            .expect("create_owner_identity_v0 must succeed");

        assert_eq!(identity.id(), Identifier::from(pro_tx));
        assert_eq!(
            identity.public_keys().len(),
            1,
            "owner identity has exactly one key"
        );
    }

    /// Different pro_tx_hash must produce different owner identity ids.
    #[test]
    fn v0_distinct_pro_tx_hashes_yield_distinct_owner_identities() {
        let platform_version = PlatformVersion::latest();
        let a = make_masternode([0x01u8; 32], [0u8; 20]);
        let b = make_masternode([0x02u8; 32], [0u8; 20]);

        let ia =
            Platform::<MockCoreRPCLike>::create_owner_identity_v0(&a, platform_version).expect("a");
        let ib =
            Platform::<MockCoreRPCLike>::create_owner_identity_v0(&b, platform_version).expect("b");
        assert_ne!(ia.id(), ib.id());
    }
}
