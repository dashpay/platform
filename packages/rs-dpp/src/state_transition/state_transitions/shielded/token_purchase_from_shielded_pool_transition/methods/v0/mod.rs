#[cfg(feature = "state-transition-signing")]
use crate::balances::credits::TokenAmount;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::TokenContractPosition;
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait TokenPurchaseFromShieldedPoolTransitionMethodsV0 {
    /// Build the transition from two already proven and signed Orchard bundles: the token pool
    /// bundle, whose sighash binds the token id and the transparent fields, and the credit pool
    /// fee bundle, whose sighash binds the token id and a digest of the token bundle's actions.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
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
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::TokenPurchaseFromShieldedPool
    }
}
