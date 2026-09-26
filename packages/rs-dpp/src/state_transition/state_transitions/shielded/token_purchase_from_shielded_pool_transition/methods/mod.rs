mod v0;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::balances::credits::TokenAmount;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::TokenContractPosition;
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    state_transition::{
        token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0,
        StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl TokenPurchaseFromShieldedPoolTransitionMethodsV0 for TokenPurchaseFromShieldedPoolTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundles(
        data_contract_id: Identifier,
        token_contract_position: TokenContractPosition,
        token_id: Identifier,
        token_count: TokenAmount,
        total_agreed_price: Credits,
        token_actions: Vec<SerializedAction>,
        token_anchor: [u8; 32],
        token_proof: Vec<u8>,
        token_binding_signature: [u8; 64],
        fee_actions: Vec<SerializedAction>,
        fee_anchor: [u8; 32],
        fee_proof: Vec<u8>,
        fee_binding_signature: [u8; 64],
        credit_amount: Credits,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .token_purchase_from_shielded_pool_state_transition
            .default_current_version
        {
            0 => TokenPurchaseFromShieldedPoolTransitionV0::try_from_bundles(
                data_contract_id,
                token_contract_position,
                token_id,
                token_count,
                total_agreed_price,
                token_actions,
                token_anchor,
                token_proof,
                token_binding_signature,
                fee_actions,
                fee_anchor,
                fee_proof,
                fee_binding_signature,
                credit_amount,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenPurchaseFromShieldedPoolTransition::try_from_bundles".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
