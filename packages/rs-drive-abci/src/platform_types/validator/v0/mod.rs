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

        // Resolve the platform ports via the accessors so a Core 23 entry (whose
        // ports live in the nested `addresses` object, legacy fields = None) still
        // produces a validator instead of being dropped; Core 22 falls back to the
        // legacy flat ports unchanged. A masternode with no resolvable platform
        // port on either form is not a valid HPMN validator.
        let (_, platform_p2p_port) = state.platform_p2p_address()?;
        let (_, platform_http_port) = state.platform_http_address()?;
        let platform_node_id = state.platform_node_id?;
        Some(ValidatorV0 {
            pro_tx_hash,
            public_key,
            node_ip: state.service.ip().to_string(),
            node_id: PubkeyHash::from_byte_array(platform_node_id),
            core_port: state.service.port(),
            platform_http_port: platform_http_port as u16,
            platform_p2p_port: platform_p2p_port as u16,
            is_banned: state.pose_ban_height.is_some(),
        })
    }
}
