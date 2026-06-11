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

        let DMNState {
            service,
            platform_node_id,
            pose_ban_height,
            platform_p2p_port,
            platform_http_port,
            ..
        } = state;
        let Some(platform_http_port) = platform_http_port else {
            return None;
        };
        let Some(platform_p2p_port) = platform_p2p_port else {
            return None;
        };
        let platform_node_id = (*platform_node_id)?;
        Some(ValidatorV0 {
            pro_tx_hash,
            public_key,
            node_ip: service.ip().to_string(),
            node_id: PubkeyHash::from_byte_array(platform_node_id),
            core_port: service.port(),
            platform_http_port: *platform_http_port as u16,
            platform_p2p_port: *platform_p2p_port as u16,
            is_banned: pose_ban_height.is_some(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PlatformConfig;
    use dpp::dashcore_rpc::json::MasternodeListItem;
    use dpp::version::PlatformVersion;

    // Core 23 Evo entry: platform ports live only in the nested `addresses`
    // object; the deprecated top-level platformP2PPort/platformHTTPPort are absent.
    const CORE23_EVO_ENTRY: &str = r#"{
        "type": "Evo",
        "proTxHash": "9a8cfd0e5fa3a7467b81a5a2fa41e40f7981591cfb62d86e35db37962c128bb0",
        "collateralHash": "35215134107b5e423d327cab12d2b4c60a9b769301096e05a95916676d2f7867",
        "collateralIndex": 0,
        "collateralAddress": "yd2PwFoqtEJdnJVSEzBDMxVnFVgEvJyvyY",
        "operatorReward": 0,
        "state": {
            "service": "192.0.2.1:9999",
            "registeredHeight": 1176,
            "revocationReason": 0,
            "ownerAddress": "yLtkvxSueGSufQZQq8L9GVHch9QRqJqGkZ",
            "votingAddress": "yLtkvxSueGSufQZQq8L9GVHch9QRqJqGkZ",
            "payoutAddress": "ybhjexnMcGckdJCyUwFu3F25zPo4mqQg1k",
            "pubKeyOperator": "a792ce1af5f7bb9281053b3934cb8b08d00d075a56498e1a525388ce467f188e8a80911fd96a20982baa9b9678452534",
            "platformNodeID": "9f3ea5525b35daf58dd17e916b8ec03cd0fa2f0c",
            "addresses": {
                "core_p2p": ["192.0.2.1:9999"],
                "platform_p2p": ["192.0.2.2:36656"],
                "platform_https": ["192.0.2.2:443"]
            }
        }
    }"#;

    #[test]
    fn validator_built_from_core23_addresses_entry() {
        let item: MasternodeListItem =
            serde_json::from_str(CORE23_EVO_ENTRY).expect("deserialize Core-23 Evo entry");
        let pro_tx_hash = item.pro_tx_hash;

        let platform_version = PlatformVersion::latest();
        let mut state = PlatformState::default_with_protocol_versions(
            platform_version.protocol_version,
            platform_version.protocol_version,
            &PlatformConfig::default_local(),
        )
        .expect("create platform state");
        state.hpmn_masternode_list_mut().insert(pro_tx_hash, item);

        let validator =
            ValidatorV0::new_validator_if_masternode_in_state(pro_tx_hash, None, &state)
                .expect("Core-23 evonode must yield a validator, not be dropped");

        assert_eq!(validator.platform_p2p_port, 36656);
        assert_eq!(validator.platform_http_port, 443);
        assert_eq!(validator.core_port, 9999);
    }
}
