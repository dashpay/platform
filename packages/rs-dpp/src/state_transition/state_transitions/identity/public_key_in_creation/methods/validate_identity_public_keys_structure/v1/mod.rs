use crate::consensus::basic::identity::{
    IdentityPublicKeyLimitsNotAllowedError, IdentityPublicKeyTypeNotAllowedForPurposeError,
    InvalidIdentityPublicKeyBudgetError, InvalidIdentityPublicKeyDataError,
};
use crate::consensus::ConsensusError;
use crate::identity::{KeyType, Purpose, SecurityLevel};
use crate::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV1Getters,
};
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::util::ml_dsa_65;
use crate::validation::SimpleConsensusValidationResult;
use crate::{ProtocolError, PublicKeyValidationError};
use platform_version::version::PlatformVersion;

/// The size of a hash key's data, which is the 160-bit hash itself.
const HASH_KEY_DATA_SIZE: usize = 20;

impl IdentityPublicKeyInCreation {
    /// v1 runs the v0 checks, then decides which keys may carry a budget or an expiry, then
    /// refuses an ML_DSA_65 key for a purpose it cannot serve, then checks the size of each
    /// hash key's and each ML_DSA_65 key's data.
    ///
    /// Limits exist to hand an application a key that can only do so much, so they are only
    /// allowed on AUTHENTICATION keys below the MASTER security level: the master key is what
    /// replaces a spent or expired key, and must not run out itself. A budget of zero could
    /// never sign anything, so it is refused instead of registering a dead key. Whether the
    /// expiry is still ahead needs the block time and is checked in state validation.
    ///
    /// The data of an ECDSA_HASH160, BIP13_SCRIPT_HASH or EDDSA_25519_HASH160 key is the hash
    /// itself, so it must be 20 bytes; data of any other size has no hash to register. The data
    /// of an ECDSA_SECP256K1 or BLS12_381 key is left to its proof of possession, which refuses
    /// data that is not a public key (an uncompressed 65-byte ECDSA_SECP256K1 key is valid).
    ///
    /// The data of an ML_DSA_65 key must be the 1,952 bytes of an encoded public key: every
    /// string of that size is a public key and no other is. An ML_DSA_65 key signs and can
    /// neither encrypt nor decrypt, so it is refused for the ENCRYPTION and DECRYPTION
    /// purposes, where it would be a key nobody can use. The key type exists from protocol
    /// version 16; a transition carrying one is not active before it, so the versions before it
    /// that select this generation never see one here.
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
                } else if identity_public_key.total_budget() == Some(0) {
                    Some(InvalidIdentityPublicKeyBudgetError::new(identity_public_key.id()).into())
                } else {
                    None
                }
            })
            .collect();
        if !validation_errors.is_empty() {
            return Ok(SimpleConsensusValidationResult::new_with_errors(
                validation_errors,
            ));
        }

        // Unreachable before protocol version 16, where no transition carrying the key type is
        // active.
        let validation_errors: Vec<ConsensusError> = identity_public_keys_with_witness
            .iter()
            .filter(|identity_public_key| {
                identity_public_key.key_type() == KeyType::ML_DSA_65
                    && matches!(
                        identity_public_key.purpose(),
                        Purpose::ENCRYPTION | Purpose::DECRYPTION
                    )
            })
            .map(|identity_public_key| {
                IdentityPublicKeyTypeNotAllowedForPurposeError::new(
                    identity_public_key.id(),
                    identity_public_key.key_type(),
                    identity_public_key.purpose(),
                )
                .into()
            })
            .collect();
        if !validation_errors.is_empty() {
            return Ok(SimpleConsensusValidationResult::new_with_errors(
                validation_errors,
            ));
        }

        // Last of the rules, so a key set that also breaks another one is still refused for that
        // rule. Without this rule a hash key of the wrong size fails when its hash is taken, as
        // an internal error that keeps its transition out of every block: a transition this rule
        // refuses never succeeded, and one refused for an earlier rule keeps its error.
        let validation_errors: Vec<ConsensusError> = identity_public_keys_with_witness
            .iter()
            .filter_map(|identity_public_key| {
                let required_size = match identity_public_key.key_type() {
                    KeyType::ECDSA_HASH160
                    | KeyType::BIP13_SCRIPT_HASH
                    | KeyType::EDDSA_25519_HASH160 => HASH_KEY_DATA_SIZE,
                    // Unreachable before protocol version 16, where no transition carrying the
                    // key type is active.
                    KeyType::ML_DSA_65 => ml_dsa_65::PUBLIC_KEY_SIZE,
                    KeyType::ECDSA_SECP256K1 | KeyType::BLS12_381 => return None,
                };
                (identity_public_key.data().len() != required_size).then(|| {
                    InvalidIdentityPublicKeyDataError::new(
                        identity_public_key.id(),
                        PublicKeyValidationError::new(format!(
                            "{} key data must be {required_size} bytes, got {}",
                            identity_public_key.key_type(),
                            identity_public_key.data().len()
                        )),
                    )
                    .into()
                })
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
            total_budget: budget,
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

    fn key_with_data(id: u32, key_type: KeyType, data_len: usize) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreationV0 {
            id,
            key_type,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![id as u8 + 2; data_len]),
            signature: BinaryData::default(),
        }
        .into()
    }

    const HASH_KEY_TYPES: [KeyType; 3] = [
        KeyType::ECDSA_HASH160,
        KeyType::BIP13_SCRIPT_HASH,
        KeyType::EDDSA_25519_HASH160,
    ];

    /// The data of a hash key is the hash itself. Data of any other size has no hash to take,
    /// which would otherwise only surface when the key is registered, as an internal error.
    #[test]
    fn should_reject_hash_key_data_that_is_not_20_bytes() {
        for key_type in HASH_KEY_TYPES {
            for data_len in [0, 1, 19, 21, 33, 65] {
                let result = validate(&[key_with_data(1, key_type, data_len)]);
                assert!(
                    matches!(
                        result.errors.as_slice(),
                        [ConsensusError::BasicError(
                            BasicError::InvalidIdentityPublicKeyDataError(e)
                        )] if e.public_key_id() == 1
                    ),
                    "{key_type} with {data_len} bytes: {:?}",
                    result.errors
                );
            }
            let result = validate(&[key_with_data(1, key_type, 20)]);
            assert!(result.is_valid(), "{key_type}: {:?}", result.errors);
        }
    }

    /// The proof of possession of an ECDSA_SECP256K1 or BLS12_381 key already refuses data that
    /// is not a public key, and an uncompressed 65-byte ECDSA_SECP256K1 key is valid, so the
    /// structure check leaves their data alone.
    #[test]
    fn should_leave_the_data_of_a_key_with_a_proof_of_possession_to_that_proof() {
        for (key_type, data_len) in [
            (KeyType::ECDSA_SECP256K1, 0),
            (KeyType::ECDSA_SECP256K1, 20),
            (KeyType::ECDSA_SECP256K1, 33),
            (KeyType::ECDSA_SECP256K1, 65),
            (KeyType::BLS12_381, 0),
            (KeyType::BLS12_381, 47),
            (KeyType::BLS12_381, 48),
        ] {
            let result = validate(&[key_with_data(1, key_type, data_len)]);
            assert!(
                result.is_valid(),
                "{key_type} with {data_len} bytes: {:?}",
                result.errors
            );
        }
    }

    #[test]
    fn should_report_every_hash_key_with_data_of_the_wrong_size() {
        let result = validate(&[
            key_with_data(0, KeyType::ECDSA_HASH160, 19),
            key_with_data(1, KeyType::ECDSA_HASH160, 20),
            key_with_data(2, KeyType::BIP13_SCRIPT_HASH, 0),
        ]);
        let reported_key_ids: Vec<_> = result
            .errors
            .iter()
            .map(|error| match error {
                ConsensusError::BasicError(BasicError::InvalidIdentityPublicKeyDataError(e)) => {
                    e.public_key_id()
                }
                other => panic!("unexpected error {other:?}"),
            })
            .collect();
        assert_eq!(reported_key_ids, vec![0, 2]);
    }

    /// A key set that breaks another structure rule is refused for that rule even when a hash
    /// key's data has the wrong size too: the key data rule runs last.
    #[test]
    fn should_report_a_failed_limit_rule_rather_than_the_key_data() {
        let result = validate(&[
            key(
                1,
                Purpose::AUTHENTICATION,
                SecurityLevel::HIGH,
                Some(0),
                None,
            ),
            key_with_data(2, KeyType::ECDSA_HASH160, 19),
        ]);
        assert!(
            matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidIdentityPublicKeyBudgetError(e)
                )] if e.public_key_id() == 1
            ),
            "{:?}",
            result.errors
        );
    }

    /// Protocol version 13 selects v0, which does not check the size of a hash key's data: its
    /// history replays unchanged.
    #[test]
    fn should_not_check_the_size_of_hash_key_data_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        for key_type in HASH_KEY_TYPES {
            let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
                &[key_with_data(1, key_type, 19)],
                false,
                platform_version,
            )
            .expect("expected validation to run");
            assert!(result.is_valid(), "{key_type}: {:?}", result.errors);
        }
    }

    /// Every string of 1,952 bytes is an encoded ML-DSA-65 public key and no other is, so data
    /// of another size is refused here, before any proof of possession is looked at.
    #[test]
    fn should_reject_ml_dsa_65_key_data_that_is_not_1952_bytes() {
        for data_len in [0, 20, 33, 1951, 1953, 2592] {
            let result = validate(&[key_with_data(1, KeyType::ML_DSA_65, data_len)]);
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::InvalidIdentityPublicKeyDataError(e)
                    )] if e.public_key_id() == 1
                ),
                "{data_len} bytes: {:?}",
                result.errors
            );
        }
        let result = validate(&[key_with_data(1, KeyType::ML_DSA_65, 1952)]);
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    fn ml_dsa_65_key(
        id: u32,
        purpose: Purpose,
        security_level: SecurityLevel,
    ) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreationV0 {
            id,
            key_type: KeyType::ML_DSA_65,
            purpose,
            security_level,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![id as u8 + 2; 1952]),
            signature: BinaryData::default(),
        }
        .into()
    }

    /// An ML_DSA_65 key signs and cannot encrypt or decrypt, so one registered for either
    /// purpose would be a key nobody can use. Every such key of the set is reported.
    #[test]
    fn should_reject_an_ml_dsa_65_key_for_encryption_or_decryption() {
        for purpose in [Purpose::ENCRYPTION, Purpose::DECRYPTION] {
            let result = validate(&[ml_dsa_65_key(1, purpose, SecurityLevel::MEDIUM)]);
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::IdentityPublicKeyTypeNotAllowedForPurposeError(e)
                    )] if e.public_key_id() == 1
                        && e.key_type() == KeyType::ML_DSA_65
                        && e.purpose() == purpose
                ),
                "{purpose:?}: {:?}",
                result.errors
            );
        }

        let result = validate(&[
            ml_dsa_65_key(0, Purpose::ENCRYPTION, SecurityLevel::MEDIUM),
            ml_dsa_65_key(1, Purpose::AUTHENTICATION, SecurityLevel::HIGH),
            ml_dsa_65_key(2, Purpose::DECRYPTION, SecurityLevel::MEDIUM),
        ]);
        let reported_key_ids: Vec<_> = result
            .errors
            .iter()
            .map(|error| match error {
                ConsensusError::BasicError(
                    BasicError::IdentityPublicKeyTypeNotAllowedForPurposeError(e),
                ) => e.public_key_id(),
                other => panic!("unexpected error {other:?}"),
            })
            .collect();
        assert_eq!(reported_key_ids, vec![0, 2]);
    }

    /// The rule is about the ML_DSA_65 key type alone: a key of a type that could be
    /// registered for ENCRYPTION or DECRYPTION before still can.
    #[test]
    fn should_leave_encryption_and_decryption_keys_of_other_types_alone() {
        for key_type in [KeyType::ECDSA_SECP256K1, KeyType::BLS12_381] {
            for purpose in [Purpose::ENCRYPTION, Purpose::DECRYPTION] {
                let key: IdentityPublicKeyInCreation = IdentityPublicKeyInCreationV0 {
                    id: 1,
                    key_type,
                    purpose,
                    security_level: SecurityLevel::MEDIUM,
                    contract_bounds: None,
                    read_only: false,
                    data: BinaryData::new(vec![3; key_type.default_size()]),
                    signature: BinaryData::default(),
                }
                .into();
                let result = validate(&[key]);
                assert!(
                    result.is_valid(),
                    "{key_type}/{purpose:?}: {:?}",
                    result.errors
                );
            }
        }
    }

    /// An ML_DSA_65 key is good for every purpose a key signs for, at every security level
    /// open to it.
    #[test]
    fn should_accept_an_ml_dsa_65_key_at_every_signing_purpose_and_security_level() {
        for (purpose, security_level) in [
            (Purpose::AUTHENTICATION, SecurityLevel::MASTER),
            (Purpose::AUTHENTICATION, SecurityLevel::CRITICAL),
            (Purpose::AUTHENTICATION, SecurityLevel::HIGH),
            (Purpose::AUTHENTICATION, SecurityLevel::MEDIUM),
            (Purpose::TRANSFER, SecurityLevel::CRITICAL),
        ] {
            let result = validate(&[ml_dsa_65_key(1, purpose, security_level)]);
            assert!(
                result.is_valid(),
                "{purpose:?}/{security_level:?}: {:?}",
                result.errors
            );
        }
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
