/// Accessors for Masternode
pub mod accessors;

use dpp::bincode::{Decode, Encode};
use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeType};
use dpp::dashcore_rpc::json::MasternodeListItem;
use std::fmt::{Debug, Formatter};

use dpp::dashcore::{ProTxHash, Txid};

use std::net::SocketAddr;

/// `Masternode` represents a masternode on the network.
#[derive(Clone, PartialEq, Encode, Decode)]
pub struct MasternodeV0 {
    /// The type of masternode (e.g., full, partial).
    pub node_type: MasternodeType,
    /// A unique hash representing the masternode's registration transaction.
    #[bincode(with_serde)]
    pub pro_tx_hash: ProTxHash,
    /// A unique hash representing the collateral transaction.
    #[bincode(with_serde)]
    pub collateral_hash: Txid,
    /// The index of the collateral transaction output.
    pub collateral_index: u32,
    /// The address where the collateral is stored; zero bytes when it has none, as for a
    /// shared masternode.
    pub collateral_address: [u8; 20],
    /// The amount of the operator's reward for running the masternode.
    pub operator_reward: f32,
    /// The current state of the masternode (e.g., enabled, pre-enabled, banned).
    pub state: MasternodeStateV0,
}

impl Debug for MasternodeV0 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MasternodeV0")
            .field("node_type", &self.node_type)
            .field("pro_tx_hash", &self.pro_tx_hash.to_string())
            .field("collateral_hash", &self.collateral_hash)
            .field("collateral_index", &self.collateral_index)
            .field("collateral_address", &self.collateral_address)
            .field("operator_reward", &self.operator_reward)
            .field("state", &self.state)
            .finish()
    }
}

impl From<MasternodeListItem> for MasternodeV0 {
    fn from(value: MasternodeListItem) -> Self {
        let MasternodeListItem {
            node_type,
            pro_tx_hash,
            collateral_hash,
            collateral_index,
            collateral_address,
            operator_reward,
            state,
        } = value;

        Self {
            node_type,
            pro_tx_hash,
            collateral_hash,
            collateral_index,
            collateral_address: stored_address(collateral_address),
            operator_reward,
            state: state.into(),
        }
    }
}

impl From<MasternodeV0> for MasternodeListItem {
    fn from(value: MasternodeV0) -> Self {
        let MasternodeV0 {
            node_type,
            pro_tx_hash,
            collateral_hash,
            collateral_index,
            collateral_address,
            operator_reward,
            state,
        } = value;

        Self {
            node_type,
            pro_tx_hash,
            collateral_hash,
            collateral_index,
            collateral_address: loaded_address(collateral_address),
            operator_reward,
            state: state.into(),
        }
    }
}

/// The stored form of an address a masternode may not have. The stored format has no room for
/// an absent address, so it is stored as zero bytes, and zero bytes read back as absent. That
/// loses no owner address: Core refuses a null owner key for every masternode but a shared one,
/// which has none. A payout or collateral address of zero bytes also reads back as absent;
/// nothing reads those from the stored list.
fn stored_address(address: Option<[u8; 20]>) -> [u8; 20] {
    address.unwrap_or_default()
}

/// The address a [`stored_address`] holds.
fn loaded_address(address: [u8; 20]) -> Option<[u8; 20]> {
    (address != [0u8; 20]).then_some(address)
}

/// A `MasternodeState` contains information about a masternode's state.
#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct MasternodeStateV0 {
    /// Masternode's network service address.
    #[bincode(with_serde)]
    pub service: SocketAddr,

    /// Block height when the masternode was registered.
    pub registered_height: u32,

    /// Block height when the masternode last revived from a Proof-of-Service ban.
    pub pose_revived_height: Option<u32>,

    /// Block height when the masternode was banned due to a failed Proof-of-Service.
    pub pose_ban_height: Option<u32>,

    /// Reason for the masternode's revocation (encoded as an integer).
    pub revocation_reason: u32,

    /// The masternode owner's public address; zero bytes when it has none, as a shared
    /// masternode.
    pub owner_address: [u8; 20],

    /// The masternode voting public address.
    pub voting_address: [u8; 20],

    /// The masternode payout public address; zero bytes when it has none, as a shared
    /// masternode or one with a payout list.
    pub payout_address: [u8; 20],

    /// The masternode operator's public key.
    pub pub_key_operator: Vec<u8>,

    /// Optional masternode operator's payout public address.
    pub operator_payout_address: Option<[u8; 20]>,

    /// Platform-specific node ID for the masternode.
    pub platform_node_id: Option<[u8; 20]>,

    /// Optional platform-specific P2P port for the masternode.
    pub platform_p2p_port: Option<u32>,

    /// Optional platform-specific HTTP port for the masternode.
    pub platform_http_port: Option<u32>,
}

impl From<DMNState> for MasternodeStateV0 {
    fn from(value: DMNState) -> Self {
        // The payout list and the nested addresses are not stored.
        #[allow(deprecated)]
        let DMNState {
            service,
            registered_height,
            pose_revived_height,
            pose_ban_height,
            revocation_reason,
            owner_address,
            voting_address,
            payout_address,
            payouts: _,
            pub_key_operator,
            operator_payout_address,
            platform_node_id,
            legacy_platform_p2p_port: platform_p2p_port,
            legacy_platform_http_port: platform_http_port,
            addresses: _,
        } = value;

        Self {
            service,
            registered_height,
            pose_revived_height,
            pose_ban_height,
            revocation_reason,
            owner_address: stored_address(owner_address),
            voting_address,
            payout_address: stored_address(payout_address),
            pub_key_operator,
            operator_payout_address,
            platform_node_id,
            platform_p2p_port,
            platform_http_port,
        }
    }
}

impl From<MasternodeStateV0> for DMNState {
    fn from(value: MasternodeStateV0) -> Self {
        let MasternodeStateV0 {
            service,
            registered_height,
            pose_revived_height,
            pose_ban_height,
            revocation_reason,
            owner_address,
            voting_address,
            payout_address,
            pub_key_operator,
            operator_payout_address,
            platform_node_id,
            platform_p2p_port,
            platform_http_port,
        } = value;

        #[allow(deprecated)]
        Self {
            service,
            registered_height,
            pose_revived_height,
            pose_ban_height,
            revocation_reason,
            owner_address: loaded_address(owner_address),
            voting_address,
            payout_address: loaded_address(payout_address),
            payouts: None,
            pub_key_operator,
            operator_payout_address,
            platform_node_id,
            legacy_platform_p2p_port: platform_p2p_port,
            legacy_platform_http_port: platform_http_port,
            addresses: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::platform_state::platform_state_for_saving::v2::{
        deserialize_masternode_entry, serialize_masternode_entry,
    };
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, PubkeyHash, ScriptBuf, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNPayout, DMNState, MasternodeAddresses, MasternodeListItem, MasternodeType,
    };
    use dpp::version::PlatformVersion;

    fn masternode(
        node_type: MasternodeType,
        collateral_address: Option<[u8; 20]>,
        owner_address: Option<[u8; 20]>,
        payout_address: Option<[u8; 20]>,
        payouts: Option<Vec<DMNPayout>>,
    ) -> MasternodeListItem {
        let is_evo = node_type == MasternodeType::Evo;
        MasternodeListItem {
            node_type,
            pro_tx_hash: ProTxHash::from_byte_array([0x31; 32]),
            collateral_hash: Txid::from_byte_array([0x32; 32]),
            collateral_index: 1,
            collateral_address,
            operator_reward: 0.0,
            state: DMNState {
                service: "1.2.3.4:9999".parse().expect("socket address"),
                registered_height: 10,
                pose_revived_height: None,
                pose_ban_height: Some(20),
                revocation_reason: 0,
                owner_address,
                voting_address: [0x33; 20],
                payout_address,
                payouts,
                pub_key_operator: vec![0x34; 48],
                operator_payout_address: Some([0x35; 20]),
                platform_node_id: is_evo.then_some([0x36; 20]),
                #[allow(deprecated)]
                legacy_platform_p2p_port: is_evo.then_some(26656),
                #[allow(deprecated)]
                legacy_platform_http_port: is_evo.then_some(443),
                addresses: None,
            },
        }
    }

    fn p2pkh_payout(key_hash: [u8; 20], reward: u16) -> DMNPayout {
        DMNPayout {
            address: key_hash,
            script: ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(key_hash)),
            reward,
        }
    }

    fn stored_and_loaded(masternode: &MasternodeListItem) -> MasternodeListItem {
        let bytes = serialize_masternode_entry(masternode, PlatformVersion::latest())
            .expect("expected to serialize the masternode entry");
        deserialize_masternode_entry(&bytes).expect("expected to deserialize the masternode entry")
    }

    /// A masternode from a list Core printed before v24 has every address, and reloads as
    /// it was stored.
    #[test]
    fn should_reload_a_masternode_with_every_address_unchanged() {
        let masternode = masternode(
            MasternodeType::Evo,
            Some([0x37; 20]),
            Some([0x38; 20]),
            Some([0x39; 20]),
            None,
        );

        assert_eq!(stored_and_loaded(&masternode), masternode);
    }

    /// The stored format has no room for an absent address: a shared masternode's owner,
    /// payout and collateral addresses are stored as zero bytes and must reload as absent,
    /// not as an address of zeros. The payout list and nested addresses are not stored.
    #[test]
    fn should_reload_absent_addresses_as_absent() {
        let mut shared = masternode(MasternodeType::Regular, None, None, None, None);
        shared.state.addresses = Some(MasternodeAddresses {
            core_p2p: vec!["1.2.3.4:9999".to_string()],
            platform_p2p: vec![],
            platform_https: vec![],
        });
        let mut expected = shared.clone();
        expected.state.addresses = None;
        assert_eq!(stored_and_loaded(&shared), expected);

        let multi_payout = masternode(
            MasternodeType::Evo,
            Some([0x37; 20]),
            Some([0x38; 20]),
            None,
            Some(vec![
                p2pkh_payout([0x3a; 20], 5000),
                p2pkh_payout([0x3b; 20], 5000),
            ]),
        );
        let mut expected = multi_payout.clone();
        expected.state.payouts = None;
        assert_eq!(stored_and_loaded(&multi_payout), expected);
    }

    /// Zero bytes stand for an absent address in the stored list, so an address of zero bytes
    /// reads back as absent. Core never prints one: it refuses a null owner key for every
    /// masternode but a shared one, and nothing reads a stored payout or collateral address.
    #[test]
    fn should_reload_an_address_of_zero_bytes_as_absent() {
        let zero = masternode(
            MasternodeType::Regular,
            Some([0u8; 20]),
            Some([0u8; 20]),
            Some([0u8; 20]),
            None,
        );
        let reloaded = stored_and_loaded(&zero);
        assert_eq!(reloaded.collateral_address, None);
        assert_eq!(reloaded.state.owner_address, None);
        assert_eq!(reloaded.state.payout_address, None);
    }
}
