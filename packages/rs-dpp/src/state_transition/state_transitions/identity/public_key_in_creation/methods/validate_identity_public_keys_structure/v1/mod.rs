use crate::consensus::basic::identity::{
    IdentityPublicKeyLimitsNotAllowedError, InvalidIdentityPublicKeyBudgetError,
};
use crate::consensus::ConsensusError;
use crate::identity::{Purpose, SecurityLevel};
use crate::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV1Getters,
};
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl IdentityPublicKeyInCreation {
    /// v1 runs the v0 checks, then decides which keys may carry a budget or an expiry.
    ///
    /// Limits exist to hand an application a key that can only do so much, so they are only
    /// allowed on AUTHENTICATION keys below the MASTER security level: the master key is what
    /// replaces a spent or expired key, and must not run out itself. A budget of zero could
    /// never sign anything, so it is refused instead of registering a dead key. Whether the
    /// expiry is still ahead needs the block time and is checked in state validation.
    #[inline(always)]
    pub(super) fn validate_identity_public_keys_structure_v1(
        identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
        in_create_identity: bool,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let result = Self::validate_identity_public_keys_structure_v0(
            identity_public_keys_with_witness,
            in_create_identity,
            platform_version,
        )?;
        if !result.is_valid() {
            return Ok(result);
        }

        let validation_errors: Vec<ConsensusError> = identity_public_keys_with_witness
            .iter()
            .filter(|identity_public_key| identity_public_key.has_limits())
            .filter_map(|identity_public_key| {
                if identity_public_key.purpose() != Purpose::AUTHENTICATION
                    || identity_public_key.security_level() == SecurityLevel::MASTER
                {
                    Some(
                        IdentityPublicKeyLimitsNotAllowedError::new(
                            identity_public_key.id(),
                            identity_public_key.purpose(),
                            identity_public_key.security_level(),
                        )
                        .into(),
                    )
                } else if identity_public_key.budget() == Some(0) {
                    Some(InvalidIdentityPublicKeyBudgetError::new(identity_public_key.id()).into())
                } else {
                    None
                }
            })
            .collect();

        Ok(SimpleConsensusValidationResult::new_with_errors(
            validation_errors,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::identity::KeyType;
    use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use crate::state_transition::public_key_in_creation::v1::IdentityPublicKeyInCreationV1;
    use platform_value::BinaryData;

    fn key(
        id: u32,
        purpose: Purpose,
        security_level: SecurityLevel,
        budget: Option<u64>,
        expires_at: Option<u64>,
    ) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreationV1 {
            id,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose,
            security_level,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![id as u8 + 2; 33]),
            budget,
            expires_at,
            signature: BinaryData::default(),
        }
        .into()
    }

    fn validate(keys: &[IdentityPublicKeyInCreation]) -> SimpleConsensusValidationResult {
        IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            keys,
            false,
            PlatformVersion::latest(),
        )
        .expect("expected validation to run")
    }

    #[test]
    fn should_accept_a_budget_and_an_expiry_on_an_authentication_key_below_master() {
        for security_level in [
            SecurityLevel::CRITICAL,
            SecurityLevel::HIGH,
            SecurityLevel::MEDIUM,
        ] {
            let result = validate(&[key(
                1,
                Purpose::AUTHENTICATION,
                security_level,
                Some(1_000),
                Some(2_000),
            )]);
            assert!(result.is_valid(), "{:?}", result.errors);
        }
    }

    #[test]
    fn should_accept_a_version_1_key_without_limits_on_any_purpose() {
        let result = validate(&[key(
            1,
            Purpose::TRANSFER,
            SecurityLevel::CRITICAL,
            None,
            None,
        )]);
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_reject_limits_on_a_master_key() {
        let result = validate(&[key(
            1,
            Purpose::AUTHENTICATION,
            SecurityLevel::MASTER,
            None,
            Some(2_000),
        )]);
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::IdentityPublicKeyLimitsNotAllowedError(e)
            )] if e.public_key_id() == 1
        ));
    }

    #[test]
    fn should_reject_limits_on_a_key_that_is_not_for_authentication() {
        for (purpose, security_level) in [
            (Purpose::TRANSFER, SecurityLevel::CRITICAL),
            (Purpose::ENCRYPTION, SecurityLevel::MEDIUM),
            (Purpose::DECRYPTION, SecurityLevel::MEDIUM),
        ] {
            let result = validate(&[key(1, purpose, security_level, Some(1_000), None)]);
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IdentityPublicKeyLimitsNotAllowedError(_)
                    )]
                ),
                "{purpose:?}: {:?}",
                result.errors
            );
        }
    }

    #[test]
    fn should_reject_a_budget_of_zero() {
        let result = validate(&[key(
            1,
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            Some(0),
            None,
        )]);
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidIdentityPublicKeyBudgetError(e)
            )] if e.public_key_id() == 1
        ));
    }

    #[test]
    fn should_report_every_offending_key() {
        let unlimited: IdentityPublicKeyInCreation = IdentityPublicKeyInCreationV0 {
            id: 0,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![1; 33]),
            signature: BinaryData::default(),
        }
        .into();
        let result = validate(&[
            unlimited,
            key(1, Purpose::TRANSFER, SecurityLevel::CRITICAL, Some(5), None),
            key(
                2,
                Purpose::AUTHENTICATION,
                SecurityLevel::HIGH,
                Some(0),
                None,
            ),
        ]);
        assert_eq!(result.errors.len(), 2, "{:?}", result.errors);
    }
}
