use crate::state_transition::batch_transition::token_place_ask_transition::validate_structure::v0::TokenPlaceAskTransitionStructureValidationV0;
use crate::state_transition::batch_transition::TokenPlaceAskTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

pub trait TokenPlaceAskTransitionStructureValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenPlaceAskTransitionStructureValidation for TokenPlaceAskTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_place_ask_transition_structure_validation
        {
            0 => self.validate_structure_v0(),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenPlaceAskTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::state_transition::batch_transition::token_place_ask_transition::TokenPlaceAskTransitionV0;

    const SIGNED_MAX: u64 = i64::MAX as u64;

    fn errors(amount: u64, price: u64) -> Vec<ConsensusError> {
        TokenPlaceAskTransition::V0(TokenPlaceAskTransitionV0 {
            base: Default::default(),
            amount,
            price,
            decimals: 8,
            min_fill: 1,
        })
        .validate_structure(PlatformVersion::latest())
        .expect("expected the structure validation to run")
        .errors
    }

    /// A listing of a token with 39 or more decimals is worth one credit at most, so terms at
    /// their bounds pass the structure validation there too.
    #[test]
    fn should_accept_a_listing_of_a_token_with_39_or_more_decimals() {
        for decimals in [39, 255] {
            let errors = TokenPlaceAskTransition::V0(TokenPlaceAskTransitionV0 {
                base: Default::default(),
                amount: SIGNED_MAX,
                price: SIGNED_MAX,
                decimals,
                min_fill: 1,
            })
            .validate_structure(PlatformVersion::latest())
            .expect("expected the structure validation to run")
            .errors;
            assert!(errors.is_empty(), "decimals {decimals}: {errors:?}");
        }
    }

    /// A listing's amount and price are stored as signed 8-byte values, so each is 1 to
    /// `i64::MAX`. At 8 decimals, `i64::MAX + 1` base units at 1 credit and 1 base unit at
    /// `i64::MAX + 1` credits are both worth what an order holds, so only these bounds refuse
    /// them before they reach the order's document.
    #[test]
    fn should_refuse_an_amount_or_price_outside_a_signed_8_byte_value() {
        for (term, accepted) in [
            (0, false),
            (1, true),
            (SIGNED_MAX, true),
            (SIGNED_MAX + 1, false),
        ] {
            let by_amount = errors(term, 1);
            let by_price = errors(1, term);
            if accepted {
                assert!(by_amount.is_empty(), "amount {term}: {by_amount:?}");
                assert!(by_price.is_empty(), "price {term}: {by_price:?}");
            } else {
                assert!(
                    matches!(
                        by_amount.as_slice(),
                        [ConsensusError::BasicError(
                            BasicError::InvalidTokenAmountError(_)
                        )]
                    ),
                    "amount {term}: {by_amount:?}"
                );
                assert!(
                    matches!(
                        by_price.as_slice(),
                        [ConsensusError::BasicError(
                            BasicError::InvalidTokenOrderPriceError(_)
                        )]
                    ),
                    "price {term}: {by_price:?}"
                );
            }
        }
    }
}
