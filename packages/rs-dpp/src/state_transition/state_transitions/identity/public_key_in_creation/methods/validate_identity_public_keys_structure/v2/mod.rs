use crate::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyBasicError, DuplicatedIdentityPublicKeyIdBasicError,
    IdentityPublicKeyLimitsNotAllowedError, InvalidIdentityPublicKeyBudgetError,
    InvalidIdentityPublicKeyDataError, InvalidIdentityPublicKeySecurityLevelError,
    InvalidIdentityPublicKeyTypeForPurposeError, MissingMasterPublicKeyError,
    TooManyHecateSignerKeysError, TooManyMasterPublicKeyError,
};
use crate::consensus::basic::BasicError;
use crate::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
#[cfg(feature = "redpallas")]
use crate::identity::identity_public_key::redpallas_hecate;
use crate::identity::{KeyID, KeyType, Purpose, SecurityLevel};
use crate::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV1Getters,
};
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::validation::SimpleConsensusValidationResult;
use crate::{ProtocolError, PublicKeyValidationError};
use platform_version::version::PlatformVersion;

/// The size of a hash key's data, which is the 160-bit hash itself.
const HASH_KEY_DATA_SIZE: usize = 20;

/// The security levels a key of `purpose` added by a user may have, `None` for a purpose a user
/// cannot add.
fn allowed_security_levels(purpose: Purpose) -> Option<&'static [SecurityLevel]> {
    match purpose {
        Purpose::AUTHENTICATION => Some(&[
            SecurityLevel::MASTER,
            SecurityLevel::CRITICAL,
            SecurityLevel::HIGH,
            SecurityLevel::MEDIUM,
        ]),
        Purpose::ENCRYPTION | Purpose::DECRYPTION | Purpose::HECATE_SIGNER => {
            Some(&[SecurityLevel::MEDIUM])
        }
        Purpose::TRANSFER => Some(&[SecurityLevel::CRITICAL]),
        Purpose::SYSTEM | Purpose::VOTING | Purpose::OWNER => None,
    }
}

/// Whether a REDPALLAS_HECATE key's data is valid, with the reason when it is not. Every build
/// that validates state transitions has the `redpallas` feature; one without it cannot check the
/// data and fails instead of admitting it.
fn redpallas_hecate_key_data_verdict(data: &[u8]) -> Result<Result<(), String>, ProtocolError> {
    #[cfg(feature = "redpallas")]
    {
        Ok(redpallas_hecate::validate_public_key_data(data))
    }
    #[cfg(not(feature = "redpallas"))]
    {
        let _ = data;
        Err(ProtocolError::NotSupported(
            "checking a REDPALLAS_HECATE key's data needs the redpallas feature".to_string(),
        ))
    }
}

/// Whether a key's type and purpose go together: a REDPALLAS_HECATE key is a HECATE_SIGNER key,
/// and a HECATE_SIGNER key is a REDPALLAS_HECATE key.
fn key_type_fits_purpose(key_type: KeyType, purpose: Purpose) -> bool {
    (key_type == KeyType::REDPALLAS_HECATE) == (purpose == Purpose::HECATE_SIGNER)
}

impl IdentityPublicKeyInCreation {
    /// v2 runs the rules of v1 and admits Hecate signer keys (protocol version 15).
    ///
    /// A HECATE_SIGNER key is a REDPALLAS_HECATE key at the MEDIUM security level, and the other
    /// way round; it cannot carry a budget or an expiry, as a key that is not for
    /// authentication never can. Its data must be 64 bytes: a RedPallas `ak` that is a Pallas
    /// point other than the identity with its sign bit clear, then a canonical `nk_commitment`.
    /// A transition adds at most one: an identity holds at most one enabled, which state
    /// validation checks against the keys it already has.
    ///
    /// The rules run in v1's order, each refusing the key set before the next runs, and the
    /// Hecate signer key rules run last, so a key set that also breaks an earlier rule keeps
    /// that rule's error.
    #[inline(always)]
    pub(super) fn validate_identity_public_keys_structure_v2(
        identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
        in_create_identity: bool,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let max_public_keys_in_creation = platform_version
            .dpp
            .state_transitions
            .identities
            .max_public_keys_in_creation as usize;
        if identity_public_keys_with_witness.len() > max_public_keys_in_creation {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                StateError::MaxIdentityPublicKeyLimitReachedError(
                    MaxIdentityPublicKeyLimitReachedError::new(max_public_keys_in_creation),
                )
                .into(),
            ));
        }

        let duplicated_ids = IdentityPublicKeyInCreation::duplicated_key_ids_witness(
            identity_public_keys_with_witness,
            platform_version,
        )?;
        if !duplicated_ids.is_empty() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                BasicError::DuplicatedIdentityPublicKeyIdBasicError(
                    DuplicatedIdentityPublicKeyIdBasicError::new(duplicated_ids),
                )
                .into(),
            ));
        }

        let duplicated_key_ids = IdentityPublicKeyInCreation::duplicated_keys_witness(
            identity_public_keys_with_witness,
            platform_version,
        )?;
        if !duplicated_key_ids.is_empty() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                BasicError::DuplicatedIdentityPublicKeyBasicError(
                    DuplicatedIdentityPublicKeyBasicError::new(duplicated_key_ids),
                )
                .into(),
            ));
        }

        if in_create_identity {
            let master_key_count = identity_public_keys_with_witness
                .iter()
                .filter(|key| key.security_level() == SecurityLevel::MASTER)
                .count();
            if master_key_count == 0 {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    BasicError::MissingMasterPublicKeyError(MissingMasterPublicKeyError::new())
                        .into(),
                ));
            } else if master_key_count > 1 {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    BasicError::TooManyMasterPublicKeyError(TooManyMasterPublicKeyError::new())
                        .into(),
                ));
            }
        }

        let validation_errors: Vec<ConsensusError> = identity_public_keys_with_witness
            .iter()
            .filter_map(|identity_public_key| {
                let levels = allowed_security_levels(identity_public_key.purpose());
                if levels
                    .is_some_and(|levels| levels.contains(&identity_public_key.security_level()))
                {
                    return None;
                }
                Some(
                    InvalidIdentityPublicKeySecurityLevelError::new(
                        identity_public_key.id(),
                        identity_public_key.purpose(),
                        identity_public_key.security_level(),
                        levels.map(<[SecurityLevel]>::to_vec),
                    )
                    .into(),
                )
            })
            .collect();
        if !validation_errors.is_empty() {
            return Ok(SimpleConsensusValidationResult::new_with_errors(
                validation_errors,
            ));
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

        let validation_errors: Vec<ConsensusError> = identity_public_keys_with_witness
            .iter()
            .filter(|identity_public_key| match identity_public_key.key_type() {
                KeyType::ECDSA_HASH160
                | KeyType::BIP13_SCRIPT_HASH
                | KeyType::EDDSA_25519_HASH160 => {
                    identity_public_key.data().len() != HASH_KEY_DATA_SIZE
                }
                // Checked with the Hecate signer key rules below.
                KeyType::ECDSA_SECP256K1 | KeyType::BLS12_381 | KeyType::REDPALLAS_HECATE => false,
            })
            .map(|identity_public_key| {
                InvalidIdentityPublicKeyDataError::new(
                    identity_public_key.id(),
                    PublicKeyValidationError::new(format!(
                        "{} key data must be {HASH_KEY_DATA_SIZE} bytes, got {}",
                        identity_public_key.key_type(),
                        identity_public_key.data().len()
                    )),
                )
                .into()
            })
            .collect();
        if !validation_errors.is_empty() {
            return Ok(SimpleConsensusValidationResult::new_with_errors(
                validation_errors,
            ));
        }

        // The Hecate signer key rules.
        let mut validation_errors: Vec<ConsensusError> = Vec::new();
        for identity_public_key in identity_public_keys_with_witness {
            if !key_type_fits_purpose(
                identity_public_key.key_type(),
                identity_public_key.purpose(),
            ) {
                validation_errors.push(
                    InvalidIdentityPublicKeyTypeForPurposeError::new(
                        identity_public_key.id(),
                        identity_public_key.key_type(),
                        identity_public_key.purpose(),
                    )
                    .into(),
                );
            } else if identity_public_key.key_type() == KeyType::REDPALLAS_HECATE {
                if let Err(message) =
                    redpallas_hecate_key_data_verdict(identity_public_key.data().as_slice())?
                {
                    validation_errors.push(
                        InvalidIdentityPublicKeyDataError::new(
                            identity_public_key.id(),
                            PublicKeyValidationError::new(message),
                        )
                        .into(),
                    );
                }
            }
        }
        if !validation_errors.is_empty() {
            return Ok(SimpleConsensusValidationResult::new_with_errors(
                validation_errors,
            ));
        }

        let signer_key_ids: Vec<KeyID> = identity_public_keys_with_witness
            .iter()
            .filter(|identity_public_key| identity_public_key.purpose() == Purpose::HECATE_SIGNER)
            .map(|identity_public_key| identity_public_key.id())
            .collect();
        if signer_key_ids.len() > 1 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                TooManyHecateSignerKeysError::new(signer_key_ids).into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(all(test, feature = "redpallas"))]
mod tests {
    use super::*;
    use crate::identity::identity_public_key::redpallas_hecate::RedPallasHecateSecretKey;
    use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use crate::state_transition::public_key_in_creation::v1::IdentityPublicKeyInCreationV1;
    use platform_value::BinaryData;

    fn secp_key(
        id: KeyID,
        purpose: Purpose,
        security_level: SecurityLevel,
    ) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreationV0 {
            id,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose,
            security_level,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![id as u8 + 2; 33]),
            signature: BinaryData::default(),
        }
        .into()
    }

    fn signer_key_data(seed: u8) -> Vec<u8> {
        RedPallasHecateSecretKey::from_secret(&[seed; 32])
            .unwrap()
            .public_key_data()
            .unwrap()
            .to_vec()
    }

    fn signer_key(id: KeyID, data: Vec<u8>) -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreationV0 {
            id,
            key_type: KeyType::REDPALLAS_HECATE,
            purpose: Purpose::HECATE_SIGNER,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(data),
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
    fn should_accept_one_hecate_signer_key_with_other_keys() {
        let result = validate(&[
            secp_key(0, Purpose::AUTHENTICATION, SecurityLevel::HIGH),
            signer_key(1, signer_key_data(1)),
            secp_key(2, Purpose::ENCRYPTION, SecurityLevel::MEDIUM),
        ]);
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_accept_one_hecate_signer_key_when_creating_an_identity() {
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &[
                secp_key(0, Purpose::AUTHENTICATION, SecurityLevel::MASTER),
                signer_key(1, signer_key_data(1)),
            ],
            true,
            PlatformVersion::latest(),
        )
        .unwrap();
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_refuse_a_hecate_signer_key_above_medium() {
        for security_level in [
            SecurityLevel::MASTER,
            SecurityLevel::CRITICAL,
            SecurityLevel::HIGH,
        ] {
            let mut key = signer_key(1, signer_key_data(1));
            let IdentityPublicKeyInCreation::V0(v0) = &mut key else {
                unreachable!()
            };
            v0.security_level = security_level;
            let result = validate(&[key]);
            assert!(matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidIdentityPublicKeySecurityLevelError(_)
                )]
            ));
        }
    }

    #[test]
    fn should_refuse_a_key_type_and_purpose_that_do_not_go_together() {
        // A REDPALLAS_HECATE key for authentication.
        let mut wrong_purpose = signer_key(1, signer_key_data(1));
        let IdentityPublicKeyInCreation::V0(v0) = &mut wrong_purpose else {
            unreachable!()
        };
        v0.purpose = Purpose::ENCRYPTION;
        // A HECATE_SIGNER key of another type.
        let mut wrong_type = secp_key(2, Purpose::HECATE_SIGNER, SecurityLevel::MEDIUM);
        let IdentityPublicKeyInCreation::V0(v0) = &mut wrong_type else {
            unreachable!()
        };
        v0.key_type = KeyType::ECDSA_SECP256K1;

        let result = validate(&[wrong_purpose, wrong_type]);
        let ids: Vec<KeyID> = result
            .errors
            .iter()
            .map(|error| match error {
                ConsensusError::BasicError(
                    BasicError::InvalidIdentityPublicKeyTypeForPurposeError(e),
                ) => e.public_key_id(),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn should_refuse_malformed_hecate_signer_key_data() {
        let valid = signer_key_data(1);
        let mut sign_bit = valid.clone();
        sign_bit[31] |= 0x80;
        for data in [valid[..63].to_vec(), vec![0u8; 64], sign_bit] {
            let result = validate(&[signer_key(1, data)]);
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::InvalidIdentityPublicKeyDataError(e)
                    )] if e.public_key_id() == 1
                ),
                "{:?}",
                result.errors
            );
        }
    }

    #[test]
    fn should_refuse_two_hecate_signer_keys_in_one_transition() {
        let result = validate(&[
            signer_key(1, signer_key_data(1)),
            signer_key(2, signer_key_data(2)),
        ]);
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::TooManyHecateSignerKeysError(e))]
                if e.public_key_ids() == [1, 2]
        ));
    }

    #[test]
    fn should_refuse_limits_on_a_hecate_signer_key() {
        let key: IdentityPublicKeyInCreation = IdentityPublicKeyInCreationV1 {
            id: 1,
            key_type: KeyType::REDPALLAS_HECATE,
            purpose: Purpose::HECATE_SIGNER,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(signer_key_data(1)),
            total_budget: None,
            expires_at: Some(2_000),
            signature: BinaryData::default(),
        }
        .into();
        let result = validate(&[key]);
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::IdentityPublicKeyLimitsNotAllowedError(e)
            )] if e.public_key_id() == 1
        ));
    }

    #[test]
    fn should_still_refuse_purposes_a_user_cannot_add() {
        for purpose in [Purpose::SYSTEM, Purpose::VOTING, Purpose::OWNER] {
            let result = validate(&[secp_key(1, purpose, SecurityLevel::MEDIUM)]);
            assert!(matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidIdentityPublicKeySecurityLevelError(_)
                )]
            ));
        }
    }

    /// Generation 1, which protocol version 14 selects, refuses a Hecate signer key as a
    /// purpose it does not know.
    #[test]
    fn should_refuse_a_hecate_signer_key_at_protocol_version_14() {
        let result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &[signer_key(1, signer_key_data(1))],
            false,
            PlatformVersion::get(14).unwrap(),
        )
        .unwrap();
        assert!(!result.is_valid());
    }
}
