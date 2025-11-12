use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
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
