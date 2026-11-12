#[cfg(feature = "state-transition-signing")]
use crate::balances::credits::TokenAmount;
#[cfg(feature = "state-transition-signing")]
use crate::data_contract::TokenContractPosition;
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::token_unshield_with_shielded_fee_transition::methods::TokenUnshieldWithShieldedFeeTransitionMethodsV0;
use crate::state_transition::token_unshield_with_shielded_fee_transition::v0::TokenUnshieldWithShieldedFeeTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl TokenUnshieldWithShieldedFeeTransitionMethodsV0 for TokenUnshieldWithShieldedFeeTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn try_from_bundles(
        data_contract_id: Identifier,
        token_contract_position: TokenContractPosition,
        token_id: Identifier,
        recipient_id: Identifier,
        amount: TokenAmount,
        token_actions: Vec<SerializedAction>,
        token_anchor: [u8; 32],
        token_proof: Vec<u8>,
        token_binding_signature: [u8; 64],
        fee_actions: Vec<SerializedAction>,
        fee_anchor: [u8; 32],
        fee_proof: Vec<u8>,
        fee_binding_signature: [u8; 64],
        credit_amount: Credits,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let transition = TokenUnshieldWithShieldedFeeTransitionV0 {
            data_contract_id,
            token_contract_position,
            token_id,
            recipient_id,
            amount,
            token_actions,
            token_anchor,
            token_proof,
            token_binding_signature,
            fee_actions,
            fee_anchor,
            fee_proof,
            fee_binding_signature,
            credit_amount,
        };
        Ok(transition.into())
    }
}
