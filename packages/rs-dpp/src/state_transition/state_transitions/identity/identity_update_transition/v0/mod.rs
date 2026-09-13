mod identity_signed;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::{BinaryData, Value};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use std::convert::{TryFrom, TryInto};

use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreationSignable;

use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::{
    identity::KeyID,
    prelude::{Identifier, Revision},
    ProtocolError,
};

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Encode, Decode, PlatformSignable, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
// There is a problem deriving bincode for a borrowed vector
// Hence we set to do it somewhat manually inside the PlatformSignable proc macro
// Instead of inside of bincode_derive
#[platform_signable(derive_bincode_with_borrowed_vec)]
#[derive(Default)]
pub struct IdentityUpdateTransitionV0 {
    /// Unique identifier of the identity to be updated
    pub identity_id: Identifier,

    /// The revision of the identity after update
    pub revision: Revision,

    /// Identity nonce for this transition to prevent replay attacks
    pub nonce: IdentityNonce,

    /// Public Keys to add to the Identity
    /// we want to skip serialization of transitions, as we does it manually in `to_object()`  and `to_json()`
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    #[platform_signable(into = "Vec<IdentityPublicKeyInCreationSignable>")]
    pub add_public_keys: Vec<IdentityPublicKeyInCreation>,

    /// Identity Public Keys ID's to disable for the Identity
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub disable_public_keys: Vec<KeyID>,

    /// The fee multiplier
    pub user_fee_increase: UserFeeIncrease,

    /// The ID of the public key used to sing the State Transition
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    /// Cryptographic signature of the State Transition
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

/// if the property isn't present the empty list is returned. If property is defined, the function
/// might return some serialization-related errors
fn get_list<T: TryFrom<Value, Error = platform_value::Error>>(
    value: &mut Value,
    property_name: &str,
) -> Result<Vec<T>, ProtocolError> {
    value
        .remove_optional_array(property_name)
        .map_err(ProtocolError::ValueError)?
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.try_into().map_err(ProtocolError::ValueError))
        .collect()
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod test {
    use super::*;
    use crate::state_transition::{
        StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionOwned, StateTransitionSingleSigned, StateTransitionType,
    };
    use platform_value::BinaryData;

    fn make_update_v0() -> IdentityUpdateTransitionV0 {
        IdentityUpdateTransitionV0 {
            identity_id: Identifier::random(),
            revision: 2,
            nonce: 5,
            add_public_keys: vec![],
            disable_public_keys: vec![1, 2],
            user_fee_increase: 3,
            signature_public_key_id: 0,
            signature: [0u8; 65].to_vec().into(),
        }
    }

    #[test]
    fn test_default() {
        let t = IdentityUpdateTransitionV0::default();
        assert_eq!(t.revision, 0);
        assert_eq!(t.nonce, 0);
        assert!(t.add_public_keys.is_empty());
        assert!(t.disable_public_keys.is_empty());
    }

    #[test]
    fn test_state_transition_like() {
        let t = make_update_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityUpdate
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.identity_id]);
        assert_eq!(t.owner_id(), t.identity_id);
    }

    #[test]
    fn test_unique_identifiers() {
        let t = make_update_v0();
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
        assert!(!ids[0].is_empty());
    }

    #[test]
    fn test_identity_signed() {
        use crate::identity::{Purpose, SecurityLevel};
        let mut t = make_update_v0();
        assert_eq!(t.signature_public_key_id(), 0);
        t.set_signature_public_key_id(42);
        assert_eq!(t.signature_public_key_id(), 42);
        let security = t.security_level_requirement(Purpose::AUTHENTICATION);
        assert_eq!(security, vec![SecurityLevel::MASTER]);
    }

    #[test]
    fn test_user_fee_increase() {
        let mut t = make_update_v0();
        assert_eq!(t.user_fee_increase(), 3);
        t.set_user_fee_increase(10);
        assert_eq!(t.user_fee_increase(), 10);
    }

    #[test]
    fn test_single_signed() {
        let mut t = make_update_v0();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2, 3]));
        assert_eq!(t.signature().as_slice(), &[1, 2, 3]);
        t.set_signature_bytes(vec![4, 5]);
        assert_eq!(t.signature().as_slice(), &[4, 5]);
    }

    #[test]
    fn test_into_state_transition() {
        use crate::state_transition::StateTransition;
        let t = make_update_v0();
        let st: StateTransition = t.into();
        match st {
            StateTransition::IdentityUpdate(_) => {}
            _ => panic!("expected IdentityUpdate"),
        }
    }

    // Legacy `StateTransitionValueConvert` round-trip / cleaned-object /
    // skip-signature tests on the V0 inner struct deleted in Phase D
    // step 9. The canonical `JsonConvertible` / `ValueConvertible`
    // round-trip is exercised on the outer enum derive — these tested
    // methods that no longer exist.

    #[test]
    fn test_get_list_empty() {
        use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
        let mut val = Value::Map(vec![]);
        let result: Result<Vec<IdentityPublicKeyInCreationV0>, _> =
            get_list(&mut val, "nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_remove_integer_list_or_default_empty() {
        let mut val = Value::Map(vec![]);
        let result: Result<Vec<u32>, _> = remove_integer_list_or_default(&mut val, "nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    /// Verifies that `try_from_identity_with_signer` rejects an invalid TRANSFER+HIGH
    /// added key client-side via the structural public-key validation, returning
    /// `ProtocolError::ConsensusError(InvalidIdentityPublicKeySecurityLevelError)`
    /// before any signing work is attempted.
    #[cfg(feature = "state-transition-signing")]
    #[tokio::test]
    async fn try_from_identity_with_signer_rejects_transfer_high_added_key() {
        use crate::address_funds::AddressWitness;
        use crate::consensus::basic::BasicError;
        use crate::consensus::ConsensusError;
        use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use crate::identity::signer::Signer;
        use crate::identity::v0::IdentityV0;
        use crate::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use crate::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use crate::version::PlatformVersion;
        use crate::ProtocolError;
        use async_trait::async_trait;
        use std::collections::BTreeMap;

        /// A signer that should never be invoked: pre-signing validation must fail
        /// before this signer is asked to sign anything.
        #[derive(Debug)]
        struct UnreachableSigner;

        #[async_trait]
        impl Signer<IdentityPublicKey> for UnreachableSigner {
            async fn sign(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<BinaryData, ProtocolError> {
                panic!(
                    "UnreachableSigner::sign must not be called when pre-signing validation rejects the transition"
                );
            }

            async fn sign_create_witness(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<AddressWitness, ProtocolError> {
                panic!(
                    "UnreachableSigner::sign_create_witness must not be called when pre-signing validation rejects the transition"
                );
            }

            fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
                false
            }
        }

        let platform_version = PlatformVersion::latest();

        // Master key on the existing identity (not used here, but the constructor expects
        // an identity to read id/revision from).
        let master_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            disabled_at: None,
        }
        .into();

        let identity: Identity = IdentityV0 {
            id: Identifier::default(),
            public_keys: BTreeMap::from([(0, master_key)]),
            balance: 0,
            revision: 0,
        }
        .into();

        // Invalid combination: TRANSFER purpose only allows CRITICAL security level.
        let invalid_transfer_high_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![1u8; 33]),
            disabled_at: None,
        }
        .into();

        let result = IdentityUpdateTransitionV0::try_from_identity_with_signer(
            &identity,
            &0,
            vec![invalid_transfer_high_key],
            vec![],
            1,
            0,
            &UnreachableSigner,
            platform_version,
            None,
        )
        .await;

        match result {
            Err(ProtocolError::ConsensusError(boxed)) => match *boxed {
                ConsensusError::BasicError(
                    BasicError::InvalidIdentityPublicKeySecurityLevelError(err),
                ) => {
                    assert_eq!(err.purpose(), Purpose::TRANSFER);
                    assert_eq!(err.security_level(), SecurityLevel::HIGH);
                }
                other => panic!(
                    "expected InvalidIdentityPublicKeySecurityLevelError, got {:?}",
                    other
                ),
            },
            other => panic!(
                "expected ConsensusError(InvalidIdentityPublicKeySecurityLevelError), got {:?}",
                other
            ),
        }
    }

    #[cfg(feature = "state-transition-signing")]
    #[tokio::test]
    async fn try_from_identity_with_signer_checks_master_key_before_basic_structure() {
        use crate::address_funds::AddressWitness;
        use crate::consensus::signature::SignatureError;
        use crate::consensus::ConsensusError;
        use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use crate::identity::signer::Signer;
        use crate::identity::v0::IdentityV0;
        use crate::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use crate::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use crate::version::PlatformVersion;
        use crate::ProtocolError;
        use async_trait::async_trait;
        use std::collections::BTreeMap;

        #[derive(Debug)]
        struct UnreachableSigner;

        #[async_trait]
        impl Signer<IdentityPublicKey> for UnreachableSigner {
            async fn sign(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<BinaryData, ProtocolError> {
                panic!("sign should not run when master key lookup fails")
            }

            async fn sign_create_witness(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<AddressWitness, ProtocolError> {
                panic!("sign_create_witness should not run when master key lookup fails")
            }

            fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
                false
            }
        }

        let platform_version = PlatformVersion::latest();
        let identity: Identity = IdentityV0 {
            id: Identifier::default(),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        }
        .into();
        let invalid_transfer_high_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![1u8; 33]),
            disabled_at: None,
        }
        .into();

        let result = IdentityUpdateTransitionV0::try_from_identity_with_signer(
            &identity,
            &999,
            vec![invalid_transfer_high_key],
            vec![],
            1,
            0,
            &UnreachableSigner,
            platform_version,
            None,
        )
        .await;

        assert!(matches!(
            result,
            Err(ProtocolError::ConsensusError(boxed))
                if matches!(
                    *boxed,
                    ConsensusError::SignatureError(SignatureError::MissingPublicKeyError(_))
                )
        ));
    }

    #[cfg(feature = "state-transition-signing")]
    #[tokio::test]
    async fn try_from_identity_with_signer_rejects_bad_added_public_key_signature_locally() {
        use crate::address_funds::AddressWitness;
        use crate::consensus::signature::SignatureError;
        use crate::consensus::ConsensusError;
        use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use crate::identity::signer::Signer;
        use crate::identity::v0::IdentityV0;
        use crate::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use crate::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use crate::version::PlatformVersion;
        use crate::ProtocolError;
        use async_trait::async_trait;
        use dashcore::secp256k1::{PublicKey as RawPublicKey, Secp256k1, SecretKey};
        use std::collections::BTreeMap;

        #[derive(Debug)]
        struct WrongIdentityKeySigner {
            wrong_secret_key: SecretKey,
        }

        #[async_trait]
        impl Signer<IdentityPublicKey> for WrongIdentityKeySigner {
            async fn sign(
                &self,
                _key: &IdentityPublicKey,
                data: &[u8],
            ) -> Result<BinaryData, ProtocolError> {
                Ok(BinaryData::new(
                    dashcore::signer::sign(data, &self.wrong_secret_key.secret_bytes())
                        .expect("wrong-key signing should succeed")
                        .to_vec(),
                ))
            }

            async fn sign_create_witness(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<AddressWitness, ProtocolError> {
                panic!("identity public key signer should not create address witnesses")
            }

            fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
                true
            }
        }

        let secp = Secp256k1::new();
        let master_secret_key =
            SecretKey::from_slice(&[1u8; 32]).expect("valid master key secret key");
        let correct_added_secret_key =
            SecretKey::from_slice(&[2u8; 32]).expect("valid added key secret key");
        let wrong_secret_key =
            SecretKey::from_slice(&[3u8; 32]).expect("valid alternate secret key");
        let master_public_key = RawPublicKey::from_secret_key(&secp, &master_secret_key);
        let correct_added_public_key =
            RawPublicKey::from_secret_key(&secp, &correct_added_secret_key);

        let identity: Identity = IdentityV0 {
            id: Identifier::default(),
            public_keys: BTreeMap::from([(
                0,
                IdentityPublicKeyV0 {
                    id: 0,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::MASTER,
                    contract_bounds: None,
                    key_type: KeyType::ECDSA_SECP256K1,
                    read_only: false,
                    data: BinaryData::new(master_public_key.serialize().to_vec()),
                    disabled_at: None,
                }
                .into(),
            )]),
            balance: 0,
            revision: 0,
        }
        .into();

        let added_public_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(correct_added_public_key.serialize().to_vec()),
            disabled_at: None,
        }
        .into();

        let result = IdentityUpdateTransitionV0::try_from_identity_with_signer(
            &identity,
            &0,
            vec![added_public_key],
            vec![],
            1,
            0,
            &WrongIdentityKeySigner { wrong_secret_key },
            PlatformVersion::latest(),
            None,
        )
        .await;

        match result {
            Err(ProtocolError::ConsensusError(boxed)) => match *boxed {
                ConsensusError::SignatureError(SignatureError::BasicECDSAError(_)) => {}
                other => panic!("expected SignatureError(BasicECDSAError), got {:?}", other),
            },
            Err(ProtocolError::ConsensusErrors(errors)) => {
                assert_eq!(errors.len(), 1, "expected a single consensus error");
                assert!(matches!(
                    errors.as_slice(),
                    [ConsensusError::SignatureError(
                        SignatureError::BasicECDSAError(_)
                    )]
                ));
            }
            other => panic!(
                "expected ConsensusError/ConsensusErrors with BasicECDSAError, got {:?}",
                other
            ),
        }
    }
}

/// if the property isn't present the empty list is returned. If property is defined, the function
/// might return some serialization-related errors
fn remove_integer_list_or_default<T>(
    value: &mut Value,
    property_name: &str,
) -> Result<Vec<T>, ProtocolError>
where
    T: TryFrom<i128>
        + TryFrom<u128>
        + TryFrom<u64>
        + TryFrom<i64>
        + TryFrom<u32>
        + TryFrom<i32>
        + TryFrom<u16>
        + TryFrom<i16>
        + TryFrom<u8>
        + TryFrom<i8>,
{
    value
        .remove_optional_array(property_name)
        .map_err(ProtocolError::ValueError)?
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.to_integer().map_err(ProtocolError::ValueError))
        .collect()
}
