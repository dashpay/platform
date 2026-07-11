use crate::consensus::basic::token::{InvalidTokenNoteTooBigError, TokenNoteOnlyAllowedWhenProposerError, TokenPricingScheduleEmptyError};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::v0_methods::TokenSetPriceForDirectPurchaseTransitionV0Methods;
use crate::state_transition::batch_transition::TokenSetPriceForDirectPurchaseTransition;
use crate::tokens::token_pricing_schedule::TokenPricingSchedule;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;

pub(super) trait TokenSetPriceForDirectPurchaseTransitionActionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenSetPriceForDirectPurchaseTransitionActionStructureValidationV0
    for TokenSetPriceForDirectPurchaseTransition
{
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // A price that is merely "too high" needs no validation: it just makes the token
        // non-purchasable. An empty `SetPrices` schedule is different — it is a storable value
        // that carries no usable price tier, so it must be rejected here. Otherwise it would flow
        // unchanged into state and the direct-purchase transformer would have to treat it as
        // "not for sale". Rejecting it at structure validation prevents such a schedule from ever
        // being written, complementing the transformer's defensive handling of an empty schedule.
        if let Some(TokenPricingSchedule::SetPrices(prices)) = self.price() {
            if prices.is_empty() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::TokenPricingScheduleEmptyError(
                        TokenPricingScheduleEmptyError::new(self.base().token_id()),
                    )),
                ));
            }
        }

        if let Some(public_note) = self.public_note() {
            if public_note.len() > MAX_TOKEN_NOTE_LEN {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(
                        InvalidTokenNoteTooBigError::new(
                            MAX_TOKEN_NOTE_LEN as u32,
                            "public_note",
                            public_note.len() as u32,
                        ),
                    )),
                ));
            }
            if let Some(group_state_transition_info) = self.base().using_group_info() {
                if !group_state_transition_info.action_is_proposer {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(
                            BasicError::TokenNoteOnlyAllowedWhenProposerError(
                                TokenNoteOnlyAllowedWhenProposerError::new(),
                            ),
                        ),
                    ));
                }
            }
        }
        Ok(SimpleConsensusValidationResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::GroupStateTransitionInfo;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::TokenSetPriceForDirectPurchaseTransitionV0;
    use platform_value::Identifier;

    fn make_transition(
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfo>,
    ) -> TokenSetPriceForDirectPurchaseTransition {
        make_transition_with_price(public_note, using_group_info, None)
    }

    fn make_transition_with_price(
        public_note: Option<String>,
        using_group_info: Option<GroupStateTransitionInfo>,
        price: Option<TokenPricingSchedule>,
    ) -> TokenSetPriceForDirectPurchaseTransition {
        TokenSetPriceForDirectPurchaseTransition::V0(TokenSetPriceForDirectPurchaseTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 1,
                token_contract_position: 0,
                data_contract_id: Identifier::default(),
                token_id: Identifier::default(),
                using_group_info,
            }),
            price,
            public_note,
        })
    }

    #[test]
    fn should_pass_with_no_public_note() {
        let transition = make_transition(None, None);
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn should_pass_with_short_public_note_and_no_group() {
        let transition = make_transition(Some("hello".to_string()), None);
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn should_pass_with_public_note_and_proposer_group() {
        let group_info = GroupStateTransitionInfo {
            group_contract_position: 0,
            action_id: Identifier::default(),
            action_is_proposer: true,
        };
        let transition = make_transition(Some("hello".to_string()), Some(group_info));
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn should_return_error_when_public_note_too_big() {
        let long_note = "a".repeat(MAX_TOKEN_NOTE_LEN + 1);
        let transition = make_transition(Some(long_note), None);
        let result = transition.validate_structure_v0().unwrap();
        assert!(!result.is_valid());
        let error = result.errors.first().unwrap();
        assert!(matches!(
            error,
            ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_))
        ));
    }

    #[test]
    fn should_pass_with_single_price() {
        let transition =
            make_transition_with_price(None, None, Some(TokenPricingSchedule::SinglePrice(100)));
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn should_pass_with_non_empty_set_prices() {
        let mut prices = std::collections::BTreeMap::new();
        prices.insert(1u64, 100u64);
        let transition =
            make_transition_with_price(None, None, Some(TokenPricingSchedule::SetPrices(prices)));
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid());
    }

    /// Regression test for the token direct-purchase chain-halt bug: an empty `SetPrices`
    /// schedule must be rejected at structure validation so it can never reach state and
    /// detonate the direct-purchase transformer.
    #[test]
    fn should_return_error_when_set_prices_is_empty() {
        let transition = make_transition_with_price(
            None,
            None,
            Some(TokenPricingSchedule::SetPrices(
                std::collections::BTreeMap::new(),
            )),
        );
        let result = transition.validate_structure_v0().unwrap();
        assert!(!result.is_valid());
        let error = result.errors.first().unwrap();
        assert!(matches!(
            error,
            ConsensusError::BasicError(BasicError::TokenPricingScheduleEmptyError(_))
        ));
    }

    #[test]
    fn should_return_error_when_note_present_but_non_proposer_in_group() {
        let group_info = GroupStateTransitionInfo {
            group_contract_position: 0,
            action_id: Identifier::default(),
            action_is_proposer: false,
        };
        let transition = make_transition(Some("hello".to_string()), Some(group_info));
        let result = transition.validate_structure_v0().unwrap();
        assert!(!result.is_valid());
        let error = result.errors.first().unwrap();
        assert!(matches!(
            error,
            ConsensusError::BasicError(BasicError::TokenNoteOnlyAllowedWhenProposerError(_))
        ));
    }
}
