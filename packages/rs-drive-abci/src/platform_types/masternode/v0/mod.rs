/// Accessors for Masternode
pub mod accessors;

use dpp::bincode::{Decode, Encode};
use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, DMNStateDiff, MasternodeType};
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

/// `state` in the form Drive keeps masternode states in, in memory as in
/// storage: `MasternodeStateV0` has no `addresses`, so the platform ports
/// resolved from them move into the legacy port fields and `addresses` is
/// dropped. It has no payout list either, so `payouts` is dropped too.
///
/// A node that restarted loads its masternode states in this form. Keeping
/// them in it in memory too means every node derives the same validators
/// from the same Core diffs, however long it has run.
pub(crate) fn in_stored_form(state: DMNState) -> DMNState {
    MasternodeStateV0::from(state).into()
}

/// `state`, kept in stored form, with a Core state diff applied, in stored
/// form again.
pub(crate) fn apply_diff_in_stored_form(state: &DMNState, diff: &DMNStateDiff) -> DMNState {
    let mut updated = state.clone();
    updated.apply_diff(diff.clone());
    in_stored_form(updated)
}

impl From<DMNState> for MasternodeStateV0 {
    fn from(value: DMNState) -> Self {
        // The payout list and the nested addresses are not stored. The stored state keeps the
        // platform ports resolved from the addresses instead: an ExtAddr evonode's legacy port
        // fields stay zero and go stale when its ports move. Loaded back as legacy ports, they
        // resolve to the same ports. Without a nested port the legacy field is stored as it is.
        let resolved_platform_p2p_port = value.platform_p2p_address().map(|(_, port)| port);
        let resolved_platform_http_port = value.platform_http_address().map(|(_, port)| port);
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
            platform_p2p_port: resolved_platform_p2p_port.or(platform_p2p_port),
            platform_http_port: resolved_platform_http_port.or(platform_http_port),
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
    use super::*;
    use crate::platform_types::platform_state::platform_state_for_saving::v2::{
        deserialize_masternode_entry, serialize_masternode_entry,
    };
    use crate::platform_types::validator::v0::PlatformPorts;
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

    /// An evonode registered at 1.2.3.4 with the given legacy platform ports
    /// and nested addresses.
    fn evonode_state(
        legacy_platform_p2p_port: Option<u32>,
        legacy_platform_http_port: Option<u32>,
        addresses: Option<MasternodeAddresses>,
    ) -> DMNState {
        #[allow(deprecated)]
        DMNState {
            service: "1.2.3.4:9999".parse().expect("socket address"),
            registered_height: 0,
            pose_revived_height: None,
            pose_ban_height: None,
            revocation_reason: 0,
            owner_address: Some([0u8; 20]),
            voting_address: [0u8; 20],
            payout_address: Some([0u8; 20]),
            payouts: None,
            pub_key_operator: vec![0u8; 48],
            operator_payout_address: None,
            platform_node_id: Some([7u8; 20]),
            legacy_platform_p2p_port,
            legacy_platform_http_port,
            addresses,
        }
    }

    fn platform_addresses(p2p: &str, https: &str) -> MasternodeAddresses {
        MasternodeAddresses {
            core_p2p: vec!["1.2.3.4:9999".to_string()],
            platform_p2p: vec![p2p.to_string()],
            platform_https: vec![https.to_string()],
        }
    }

    /// The stored state keeps no `addresses`, so it must keep the ports they
    /// carry: an ExtAddr evonode's legacy port fields stay zero, and after a
    /// port move they are stale.
    #[test]
    fn should_store_the_platform_ports_from_addresses() {
        let stored = MasternodeStateV0::from(evonode_state(
            Some(36656),
            Some(0),
            Some(platform_addresses("1.2.3.4:36657", "1.2.3.4:8443")),
        ));

        assert_eq!(stored.platform_p2p_port, Some(36657));
        assert_eq!(stored.platform_http_port, Some(8443));
    }

    /// A node that restarts must resolve the same platform ports as one that
    /// kept its state in memory, or it advertises stale validator ports.
    #[test]
    fn should_resolve_the_same_platform_ports_after_a_store_and_load() {
        let in_memory = evonode_state(
            Some(36656),
            Some(0),
            Some(platform_addresses("1.2.3.4:36657", "1.2.3.4:8443")),
        );

        let loaded = DMNState::from(MasternodeStateV0::from(in_memory.clone()));

        assert_eq!(PlatformPorts::of(&loaded), PlatformPorts::of(&in_memory));
        assert_eq!(
            in_stored_form(loaded.clone()),
            loaded,
            "a loaded state is already in stored form"
        );
    }

    /// Without nested platform ports the legacy fields are stored as they are,
    /// zero included, and so is a value no port can have: that is what every
    /// earlier binary stores, and a state loaded back is then already in
    /// stored form.
    #[test]
    fn should_store_legacy_ports_unchanged_without_nested_ports() {
        for (p2p, http) in [
            (Some(26656), Some(443)),
            (Some(0), Some(0)),
            (None, None),
            (Some(70000), Some(70000)),
        ] {
            let stored = MasternodeStateV0::from(evonode_state(p2p, http, None));
            assert_eq!(
                (stored.platform_p2p_port, stored.platform_http_port),
                (p2p, http)
            );
            let loaded = DMNState::from(stored);
            assert_eq!(in_stored_form(loaded.clone()), loaded);

            let core_entry_only = MasternodeAddresses {
                core_p2p: vec!["1.2.3.4:9999".to_string()],
                platform_p2p: vec![],
                platform_https: vec![],
            };
            let stored = MasternodeStateV0::from(evonode_state(p2p, http, Some(core_entry_only)));
            assert_eq!(
                (stored.platform_p2p_port, stored.platform_http_port),
                (p2p, http)
            );
        }
    }

    /// Masternode states are kept in memory in stored form so that a node
    /// that restarted holds the same state as one that did not. That only
    /// holds while the stored form is exactly what the store writes and
    /// reads back.
    #[test]
    fn should_hold_in_stored_form_exactly_what_the_store_reads_back() {
        let revoked = |addresses: Option<MasternodeAddresses>| DMNState {
            service: "[::]:0".parse().expect("socket address"),
            pose_ban_height: Some(200),
            revocation_reason: 1,
            platform_node_id: Some([0u8; 20]),
            ..evonode_state(Some(26656), Some(443), addresses)
        };
        let placeholder_host = MasternodeAddresses {
            core_p2p: vec![],
            platform_p2p: vec!["255.255.255.255:36657".to_string()],
            platform_https: vec![],
        };
        let on_ipv6 = DMNState {
            service: "[2001:db8::1]:9999".parse().expect("socket address"),
            ..evonode_state(
                None,
                None,
                Some(platform_addresses(
                    "[2001:db8::1]:36656",
                    "[2001:db8::1]:443",
                )),
            )
        };
        let states = [
            evonode_state(Some(26656), Some(443), None),
            evonode_state(Some(0), Some(0), None),
            evonode_state(None, None, None),
            evonode_state(
                Some(36656),
                Some(443),
                Some(platform_addresses("1.2.3.4:36656", "1.2.3.4:443")),
            ),
            evonode_state(
                Some(0),
                Some(0),
                Some(platform_addresses("1.2.3.4:36657", "1.2.3.4:8443")),
            ),
            evonode_state(
                None,
                None,
                Some(platform_addresses("1.2.3.4:36657", "1.2.3.4:8443")),
            ),
            evonode_state(Some(36657), Some(443), Some(placeholder_host)),
            on_ipv6,
            revoked(None),
            revoked(Some(MasternodeAddresses::default())),
        ];

        for state in states {
            let masternode = MasternodeListItem {
                node_type: MasternodeType::Evo,
                pro_tx_hash: ProTxHash::from_byte_array([0x77u8; 32]),
                collateral_hash: Txid::from_byte_array([0x11u8; 32]),
                collateral_index: 1,
                collateral_address: Some([0x22u8; 20]),
                operator_reward: 0.0,
                state: state.clone(),
            };
            let entry = serialize_masternode_entry(&masternode, PlatformVersion::latest())
                .expect("serializable masternode");
            let loaded = deserialize_masternode_entry(&entry).expect("its own entry");

            assert_eq!(
                loaded,
                MasternodeListItem {
                    state: in_stored_form(state.clone()),
                    ..masternode
                },
                "state: {state:?}"
            );
        }
    }
}
