use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::identity::identity_public_key_already_expired_error::IdentityPublicKeyAlreadyExpiredError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV1Getters,
};
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::SimpleConsensusValidationResult;

/// A key whose expiry is at or before the time of the block that adds it could never sign, and
/// would still take its id and, for a unique key type, its public key hash for good. The usual
/// way to get there is an expiry given in seconds instead of milliseconds, so it is refused.
pub(super) fn validate_identity_public_keys_limits_v0(
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    block_info: &BlockInfo,
) -> SimpleConsensusValidationResult {
    let validation_errors: Vec<ConsensusError> = identity_public_keys_with_witness
        .iter()
        .filter_map(|identity_public_key| {
            let expires_at = identity_public_key.expires_at()?;
            (block_info.time_ms >= expires_at).then(|| {
                IdentityPublicKeyAlreadyExpiredError::new(
                    identity_public_key.id(),
                    expires_at,
                    block_info.time_ms,
                )
                .into()
            })
        })
        .collect();

    SimpleConsensusValidationResult::new_with_errors(validation_errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::consensus::state::state_error::StateError;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use dpp::state_transition::public_key_in_creation::v1::IdentityPublicKeyInCreationV1;

    fn key(id: u32, expires_at: Option<u64>) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreationV1 {
            id,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![2; 33]),
            total_budget: Some(10),
            expires_at,
            signature: BinaryData::default(),
        }
        .into()
    }

    fn block_at(time_ms: u64) -> BlockInfo {
        BlockInfo {
            time_ms,
            ..Default::default()
        }
    }

    #[test]
    fn should_accept_keys_that_expire_after_the_block_or_never() {
        let unlimited: IdentityPublicKeyInCreation =
            IdentityPublicKeyInCreationV0::default().into();
        let result = validate_identity_public_keys_limits_v0(
            &[unlimited, key(1, None), key(2, Some(1_001))],
            &block_at(1_000),
        );
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_reject_a_key_that_expires_at_or_before_the_block_time() {
        let result = validate_identity_public_keys_limits_v0(
            &[key(1, Some(1_000)), key(2, Some(5)), key(3, Some(1_001))],
            &block_at(1_000),
        );
        let rejected: Vec<u32> = result
            .errors
            .iter()
            .map(|error| match error {
                ConsensusError::StateError(StateError::IdentityPublicKeyAlreadyExpiredError(e)) => {
                    assert_eq!(e.block_time_ms(), 1_000);
                    e.public_key_id()
                }
                other => panic!("unexpected error {other:?}"),
            })
            .collect();
        assert_eq!(rejected, vec![1, 2]);
    }
}
