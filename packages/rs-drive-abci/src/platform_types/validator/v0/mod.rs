use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use dpp::bls_signatures::{Bls12381G2Impl, PublicKey as BlsPublicKey};
pub use dpp::core_types::validator::v0::*;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{ProTxHash, PubkeyHash};
use dpp::dashcore_rpc::json::{DMNState, MasternodeListItem};
pub(crate) trait NewValidatorIfMasternodeInState {
    fn new_validator_if_masternode_in_state(
        pro_tx_hash: ProTxHash,
        public_key: Option<BlsPublicKey<Bls12381G2Impl>>,
        state: &PlatformState,
    ) -> Option<ValidatorV0>;
}

impl NewValidatorIfMasternodeInState for ValidatorV0 {
    /// Makes a validator if the masternode is in the list and is valid
    fn new_validator_if_masternode_in_state(
        pro_tx_hash: ProTxHash,
        public_key: Option<BlsPublicKey<Bls12381G2Impl>>,
        state: &PlatformState,
    ) -> Option<Self> {
        let MasternodeListItem { state, .. } = state.hpmn_masternode_list().get(&pro_tx_hash)?;

        let platform_ports = PlatformPorts::of(state);
        let PlatformPorts {
            p2p: Some(platform_p2p_port),
            http: Some(platform_http_port),
        } = platform_ports
        else {
            tracing::debug!(
                pro_tx_hash = %pro_tx_hash,
                ?platform_ports,
                "evonode is not a validator: its platform ports do not resolve"
            );
            return None;
        };
        let DMNState {
            service,
            platform_node_id,
            pose_ban_height,
            ..
        } = state;
        let platform_node_id = (*platform_node_id)?;
        // Only the ports come from the platform entries: the validator is
        // advertised on the core IP from `service`.
        Some(ValidatorV0 {
            pro_tx_hash,
            public_key,
            node_ip: service.ip().to_string(),
            node_id: PubkeyHash::from_byte_array(platform_node_id),
            core_port: service.port(),
            platform_http_port,
            platform_p2p_port,
            is_banned: pose_ban_height.is_some(),
        })
    }
}

/// The platform ports a masternode's state gives its validator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlatformPorts {
    /// The port Tenderdash dials the validator on.
    pub p2p: Option<u16>,
    /// The platform HTTPS port.
    pub http: Option<u16>,
}

impl PlatformPorts {
    /// Resolves each port from the nested `addresses` entry first, else from
    /// the legacy port field as it is, zero included: rust-dashcore's
    /// `platform_{p2p,http}_address()` fall back that way.
    ///
    /// A zero legacy port is kept rather than read as "not set": whether an
    /// evonode is a validator decides quorum rotation, so it must not depend
    /// on how a port is read.
    pub(crate) fn of(state: &DMNState) -> Self {
        Self {
            p2p: state
                .platform_p2p_address()
                .and_then(|(_host, port)| u16::try_from(port).ok()),
            http: state
                .platform_http_address()
                .and_then(|(_host, port)| u16::try_from(port).ok()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PlatformConfig;
    use dpp::dashcore::{Network, Txid};
    use dpp::dashcore_rpc::json::{MasternodeAddresses, MasternodeType};
    use dpp::version::PlatformVersion;

    const PRO_TX_HASH: [u8; 32] = [0x77u8; 32];

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

    fn validator_for(dmn_state: DMNState) -> Option<ValidatorV0> {
        let platform_version = PlatformVersion::latest();
        let mut state = PlatformState::default_with_protocol_versions(
            platform_version.protocol_version,
            platform_version.protocol_version,
            &PlatformConfig::default_for_network(Network::Testnet),
        )
        .expect("platform state");
        let pro_tx_hash = ProTxHash::from_byte_array(PRO_TX_HASH);
        state.insert_masternode(MasternodeListItem {
            node_type: MasternodeType::Evo,
            pro_tx_hash,
            collateral_hash: Txid::from_byte_array([0u8; 32]),
            collateral_index: 0,
            collateral_address: Some([0u8; 20]),
            operator_reward: 0.0,
            state: dmn_state,
        });
        ValidatorV0::new_validator_if_masternode_in_state(pro_tx_hash, None, &state)
    }

    /// An ExtAddr evonode keeps its platform ports in its network info; its
    /// legacy port fields stay zero.
    #[test]
    fn should_take_platform_ports_from_addresses_when_legacy_ports_are_zero() {
        let validator = validator_for(evonode_state(
            Some(0),
            Some(0),
            Some(platform_addresses("1.2.3.4:36656", "1.2.3.4:443")),
        ))
        .expect("the evonode has platform ports");

        assert_eq!(validator.platform_p2p_port, 36656);
        assert_eq!(validator.platform_http_port, 443);
    }

    /// A state whose legacy ports are absent takes its ports from
    /// `addresses`.
    #[test]
    fn should_take_platform_ports_from_addresses_when_legacy_ports_are_absent() {
        let validator = validator_for(evonode_state(
            None,
            None,
            Some(platform_addresses("1.2.3.4:36656", "1.2.3.4:443")),
        ))
        .expect("the evonode has platform ports");

        assert_eq!(validator.platform_p2p_port, 36656);
        assert_eq!(validator.platform_http_port, 443);
    }

    /// The validator is advertised on the core IP from `service`, whatever
    /// host `addresses` names for the platform services: a legacy evonode's
    /// port-only diff pairs the port with the placeholder host
    /// 255.255.255.255.
    #[test]
    fn should_advertise_the_service_ip_whatever_host_addresses_name() {
        let validator = validator_for(evonode_state(
            Some(36656),
            Some(443),
            Some(platform_addresses(
                "255.255.255.255:36656",
                "203.0.113.7:443",
            )),
        ))
        .expect("the evonode has platform ports");

        assert_eq!(validator.node_ip, "1.2.3.4");
        assert_eq!(validator.core_port, 9999);
    }

    /// Core before v23 lists no nested addresses; the legacy ports apply.
    #[test]
    fn should_take_legacy_ports_without_addresses() {
        let validator = validator_for(evonode_state(Some(26656), Some(443), None))
            .expect("the evonode has platform ports");

        assert_eq!(validator.node_ip, "1.2.3.4");
        assert_eq!(validator.platform_p2p_port, 26656);
        assert_eq!(validator.platform_http_port, 443);
    }

    /// Validator-set membership decides quorum rotation, so it must not
    /// depend on how a node reads a port: an evonode whose legacy ports are 0
    /// and that lists no nested ports stays a validator, with those ports.
    #[test]
    fn should_keep_an_evonode_with_zero_legacy_ports_as_a_validator() {
        let validator = validator_for(evonode_state(Some(0), Some(0), None))
            .expect("membership does not depend on the port value");

        assert_eq!(validator.platform_p2p_port, 0);
        assert_eq!(validator.platform_http_port, 0);
    }

    /// A revoked legacy evonode as Core lists it with
    /// `-deprecatedrpc=service`: its network info is empty, so `service` is
    /// `[::]:0` and `addresses` has no platform entry, while the legacy port
    /// fields keep the ports it was registered with.
    fn revoked_legacy_evonode_state(addresses: &str) -> DMNState {
        serde_json::from_str(&format!(
            r#"{{
                "service": "[::]:0",
                "addresses": {addresses},
                "registeredHeight": 100,
                "PoSeRevivedHeight": -1,
                "PoSeBanHeight": 200,
                "revocationReason": 1,
                "ownerAddress": "yPBWCdMRY5PsS3hJzs7csbdWQVRR85yxUz",
                "votingAddress": "ySM11LUD65Bi4p1gm68XLkdWc65TBKRzvQ",
                "payoutAddress": "yX4Ve7Q8Y4jscV4LZJD8HVCHKyePzR3MhA",
                "pubKeyOperator": "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                "platformNodeID": "0000000000000000000000000000000000000000",
                "platformP2PPort": 26656,
                "platformHTTPPort": 443
            }}"#
        ))
        .expect("Core's JSON for a revoked evonode")
    }

    /// Whether an evonode is a validator decides quorum rotation, so every
    /// node must build the same member for it. For an evonode whose listed
    /// `addresses` hold no platform entry, that is the member the legacy
    /// port fields alone give: banned, on the unspecified IP, with its
    /// registered ports.
    #[test]
    fn should_build_a_revoked_legacy_evonode_from_its_legacy_ports() {
        for addresses in [r#"{}"#, r#"{"core_p2p": []}"#] {
            let validator = validator_for(revoked_legacy_evonode_state(addresses));

            assert_eq!(
                validator,
                Some(ValidatorV0 {
                    pro_tx_hash: ProTxHash::from_byte_array(PRO_TX_HASH),
                    public_key: None,
                    node_ip: "::".to_string(),
                    node_id: PubkeyHash::from_byte_array([0u8; 20]),
                    core_port: 0,
                    platform_http_port: 443,
                    platform_p2p_port: 26656,
                    is_banned: true,
                }),
                "addresses: {addresses}"
            );
        }
    }
}
