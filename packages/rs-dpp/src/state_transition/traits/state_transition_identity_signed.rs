#[cfg(any(
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
use crate::consensus::signature::{
    InvalidSignaturePublicKeySecurityLevelError, PublicKeyIsDisabledError,
};
use anyhow::anyhow;
use dashcore::secp256k1::{PublicKey as RawPublicKey, SecretKey as RawSecretKey};

#[cfg(feature = "state-transition-validation")]
use crate::state_transition::errors::WrongPublicKeyPurposeError;

#[cfg(any(
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::state_transition::StateTransitionLike;
#[cfg(any(
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
use crate::state_transition::StateTransitionSigningOptions;

#[cfg(any(
    feature = "state-transition-signing",
    feature = "state-transition-validation"
))]
use crate::identity::IdentityPublicKey;
use crate::identity::Purpose;
use crate::{
    identity::{KeyID, SecurityLevel},
    prelude::*,
};

pub trait StateTransitionIdentitySigned: StateTransitionLike {
    fn signature_public_key_id(&self) -> KeyID;
    fn set_signature_public_key_id(&mut self, key_id: KeyID);

    //this is not versioned because of it just being the base trait default
    #[cfg(any(
        feature = "state-transition-signing",
        feature = "state-transition-validation"
    ))]
    /// Verifies that the supplied public key has the correct security level
    /// and purpose to sign the state transition
    /// This should only be used for authentication
    fn verify_public_key_level_and_purpose(
        &self,
        public_key: &IdentityPublicKey,
        options: StateTransitionSigningOptions,
    ) -> Result<(), ProtocolError> {
        if !options.allow_signing_with_any_purpose
            && !self.purpose_requirement().contains(&public_key.purpose())
        {
            return Err(ProtocolError::WrongPublicKeyPurposeError(
                WrongPublicKeyPurposeError::new(public_key.purpose(), self.purpose_requirement()),
            ));
        }

        // Otherwise, key security level should be less than MASTER but more or equal than required
        if !options.allow_signing_with_any_security_level
            && !self
                .security_level_requirement(public_key.purpose())
                .contains(&public_key.security_level())
        {
            return Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
                InvalidSignaturePublicKeySecurityLevelError::new(
                    public_key.security_level(),
                    self.security_level_requirement(public_key.purpose()),
                ),
            ));
        }

        Ok(())
    }

    //this is not versioned because of it just being the base trait default
    #[cfg(any(
        feature = "state-transition-signing",
        feature = "state-transition-validation"
    ))]
    fn verify_public_key_is_enabled(
        &self,
        public_key: &IdentityPublicKey,
    ) -> Result<(), ProtocolError> {
        if public_key.disabled_at().is_some() {
            return Err(ProtocolError::PublicKeyIsDisabledError(
                PublicKeyIsDisabledError::new(public_key.id()),
            ));
        }
        Ok(())
    }

    /// Returns minimal key security level that can be used to sign this ST.
    /// Override this method if the ST requires a different security level.
    fn security_level_requirement(&self, purpose: Purpose) -> Vec<SecurityLevel>;

    /// The purpose requirement for the signing key
    /// The default is authentication
    /// However for Withdrawals and Fund Transfers the requirement is TRANSFER
    fn purpose_requirement(&self) -> Vec<Purpose> {
        vec![Purpose::AUTHENTICATION]
    }
}

pub fn get_compressed_public_ec_key(private_key: &[u8]) -> Result<[u8; 33], ProtocolError> {
    let sk = RawSecretKey::from_slice(private_key)
        .map_err(|e| anyhow!("Invalid ECDSA private key: {}", e))?;

    let secp = dashcore::secp256k1::Secp256k1::new();
    let public_key_compressed = RawPublicKey::from_secret_key(&secp, &sk).serialize();
    Ok(public_key_compressed)
}

#[cfg(all(
    test,
    any(
        feature = "state-transition-signing",
        feature = "state-transition-validation"
    )
))]
mod test {
    use super::*;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::{IdentityPublicKey, KeyType};
    use crate::prelude::UserFeeIncrease;
    use crate::state_transition::batch_transition::BatchTransition;
    use crate::state_transition::{
        StateTransition, StateTransitionFieldTypes, StateTransitionSigningOptions,
        StateTransitionType,
    };
    use platform_value::BinaryData;

    #[derive(Clone, Debug)]
    struct MockSignedTransition {
        signature_public_key_id: KeyID,
        required_levels: Vec<SecurityLevel>,
        required_purposes: Vec<Purpose>,
        user_fee_increase: UserFeeIncrease,
    }

    impl Default for MockSignedTransition {
        fn default() -> Self {
            Self {
                signature_public_key_id: 1,
                required_levels: vec![SecurityLevel::HIGH],
                required_purposes: vec![Purpose::AUTHENTICATION],
                user_fee_increase: 0,
            }
        }
    }

    impl StateTransitionFieldTypes for MockSignedTransition {
        fn signature_property_paths() -> Vec<&'static str> {
            vec![]
        }

        fn identifiers_property_paths() -> Vec<&'static str> {
            vec![]
        }

        fn binary_property_paths() -> Vec<&'static str> {
            vec![]
        }
    }

    impl From<MockSignedTransition> for StateTransition {
        fn from(_: MockSignedTransition) -> Self {
            StateTransition::Batch(BatchTransition::default())
        }
    }

    impl StateTransitionLike for MockSignedTransition {
        fn state_transition_protocol_version(&self) -> u32 {
            1
        }

        fn state_transition_type(&self) -> StateTransitionType {
            StateTransitionType::Batch
        }

        fn user_fee_increase(&self) -> UserFeeIncrease {
            self.user_fee_increase
        }

        fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
            self.user_fee_increase = user_fee_increase;
        }

        fn modified_data_ids(&self) -> Vec<Identifier> {
            vec![]
        }

        fn unique_identifiers(&self) -> Vec<String> {
            vec![]
        }
    }

    impl StateTransitionIdentitySigned for MockSignedTransition {
        fn signature_public_key_id(&self) -> KeyID {
            self.signature_public_key_id
        }

        fn set_signature_public_key_id(&mut self, key_id: KeyID) {
            self.signature_public_key_id = key_id;
        }

        fn security_level_requirement(&self, _purpose: Purpose) -> Vec<SecurityLevel> {
            self.required_levels.clone()
        }

        fn purpose_requirement(&self) -> Vec<Purpose> {
            self.required_purposes.clone()
        }
    }

    fn identity_public_key(
        id: KeyID,
        purpose: Purpose,
        security_level: SecurityLevel,
        disabled_at: Option<u64>,
    ) -> IdentityPublicKey {
        let compressed = get_compressed_public_ec_key(&[1; 32]).expect("expected valid key");

        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(compressed.to_vec()),
            disabled_at,
        })
    }

    fn strict_options() -> StateTransitionSigningOptions {
        StateTransitionSigningOptions {
            allow_signing_with_any_security_level: false,
            allow_signing_with_any_purpose: false,
        }
    }

    #[test]
    fn should_accept_matching_purpose_and_security_level() {
        let transition = MockSignedTransition::default();
        let key = identity_public_key(1, Purpose::AUTHENTICATION, SecurityLevel::HIGH, None);

        assert!(transition
            .verify_public_key_level_and_purpose(&key, strict_options())
            .is_ok());
    }

    #[test]
    fn should_reject_wrong_purpose() {
        let transition = MockSignedTransition::default();
        let key = identity_public_key(1, Purpose::TRANSFER, SecurityLevel::HIGH, None);

        assert!(matches!(
            transition.verify_public_key_level_and_purpose(&key, strict_options()),
            Err(ProtocolError::WrongPublicKeyPurposeError(_))
        ));
    }

    #[test]
    fn should_reject_wrong_security_level() {
        let transition = MockSignedTransition::default();
        let key = identity_public_key(1, Purpose::AUTHENTICATION, SecurityLevel::MEDIUM, None);

        assert!(matches!(
            transition.verify_public_key_level_and_purpose(&key, strict_options()),
            Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
                _
            ))
        ));
    }

    #[test]
    fn should_allow_any_purpose_when_option_enabled() {
        let transition = MockSignedTransition::default();
        let key = identity_public_key(1, Purpose::TRANSFER, SecurityLevel::HIGH, None);

        assert!(transition
            .verify_public_key_level_and_purpose(
                &key,
                StateTransitionSigningOptions {
                    allow_signing_with_any_security_level: false,
                    allow_signing_with_any_purpose: true,
                },
            )
            .is_ok());
    }

    #[test]
    fn should_allow_any_security_level_when_option_enabled() {
        let transition = MockSignedTransition::default();
        let key = identity_public_key(1, Purpose::AUTHENTICATION, SecurityLevel::MEDIUM, None);

        assert!(transition
            .verify_public_key_level_and_purpose(
                &key,
                StateTransitionSigningOptions {
                    allow_signing_with_any_security_level: true,
                    allow_signing_with_any_purpose: false,
                },
            )
            .is_ok());
    }

    #[test]
    fn should_verify_enabled_public_key() {
        let transition = MockSignedTransition::default();
        let key = identity_public_key(1, Purpose::AUTHENTICATION, SecurityLevel::HIGH, None);

        assert!(transition.verify_public_key_is_enabled(&key).is_ok());
    }

    #[test]
    fn should_reject_disabled_public_key() {
        let transition = MockSignedTransition::default();
        let key = identity_public_key(1, Purpose::AUTHENTICATION, SecurityLevel::HIGH, Some(42));

        assert!(matches!(
            transition.verify_public_key_is_enabled(&key),
            Err(ProtocolError::PublicKeyIsDisabledError(_))
        ));
    }

    #[test]
    fn should_default_purpose_to_authentication() {
        let transition = MockSignedTransition::default();

        assert_eq!(
            vec![Purpose::AUTHENTICATION],
            transition.purpose_requirement()
        );
    }

    #[test]
    fn should_support_transfer_purpose_requirement() {
        let transition = MockSignedTransition {
            required_purposes: vec![Purpose::TRANSFER],
            ..Default::default()
        };
        let key = identity_public_key(1, Purpose::TRANSFER, SecurityLevel::HIGH, None);

        assert!(transition
            .verify_public_key_level_and_purpose(&key, strict_options())
            .is_ok());
    }

    #[test]
    fn should_reject_authentication_key_for_transfer_requirement() {
        let transition = MockSignedTransition {
            required_purposes: vec![Purpose::TRANSFER],
            ..Default::default()
        };
        let key = identity_public_key(1, Purpose::AUTHENTICATION, SecurityLevel::HIGH, None);

        assert!(matches!(
            transition.verify_public_key_level_and_purpose(&key, strict_options()),
            Err(ProtocolError::WrongPublicKeyPurposeError(_))
        ));
    }

    #[test]
    fn should_set_signature_public_key_id() {
        let mut transition = MockSignedTransition::default();

        transition.set_signature_public_key_id(9);

        assert_eq!(9, transition.signature_public_key_id());
    }

    #[test]
    fn should_return_signature_public_key_id() {
        let transition = MockSignedTransition {
            signature_public_key_id: 7,
            ..Default::default()
        };

        assert_eq!(7, transition.signature_public_key_id());
    }

    #[test]
    fn should_return_configured_security_level_requirement() {
        let transition = MockSignedTransition {
            required_levels: vec![SecurityLevel::MASTER, SecurityLevel::HIGH],
            ..Default::default()
        };

        assert_eq!(
            vec![SecurityLevel::MASTER, SecurityLevel::HIGH],
            transition.security_level_requirement(Purpose::AUTHENTICATION)
        );
    }

    #[test]
    fn should_validate_master_security_level_when_required() {
        let transition = MockSignedTransition {
            required_levels: vec![SecurityLevel::MASTER],
            ..Default::default()
        };
        let key = identity_public_key(1, Purpose::AUTHENTICATION, SecurityLevel::MASTER, None);

        assert!(transition
            .verify_public_key_level_and_purpose(&key, strict_options())
            .is_ok());
    }

    #[test]
    fn should_reject_high_security_when_master_required() {
        let transition = MockSignedTransition {
            required_levels: vec![SecurityLevel::MASTER],
            ..Default::default()
        };
        let key = identity_public_key(1, Purpose::AUTHENTICATION, SecurityLevel::HIGH, None);

        assert!(matches!(
            transition.verify_public_key_level_and_purpose(&key, strict_options()),
            Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
                _
            ))
        ));
    }

    #[test]
    fn should_get_compressed_public_key_for_valid_private_key() {
        let compressed = get_compressed_public_ec_key(&[1; 32]).expect("expected key");

        assert_eq!(33, compressed.len());
        assert_ne!([0; 33], compressed);
    }

    #[test]
    fn should_return_error_for_invalid_private_key_size() {
        let result = get_compressed_public_ec_key(&[1; 31]);

        assert!(result.is_err());
    }

    #[test]
    fn should_return_deterministic_compressed_public_key() {
        let first = get_compressed_public_ec_key(&[2; 32]).expect("expected first key");
        let second = get_compressed_public_ec_key(&[2; 32]).expect("expected second key");

        assert_eq!(first, second);
    }
}
