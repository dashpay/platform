mod proved;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use std::convert::TryFrom;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::Identity;
use crate::prelude::{Identifier, UserFeeIncrease};

use crate::identity::accessors::IdentityGettersV0;
use crate::identity::state_transition::AssetLockProved;
use crate::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreationSignable;
use crate::version::PlatformVersion;
use crate::ProtocolError;

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, PartialEq, Encode, Decode, PlatformSignable)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase"),
    serde(try_from = "IdentityCreateTransitionV0Inner")
)]
// There is a problem deriving bincode for a borrowed vector
// Hence we set to do it somewhat manually inside the PlatformSignable proc macro
// Instead of inside of bincode_derive
#[platform_signable(derive_bincode_with_borrowed_vec)]
#[derive(Default)]
pub struct IdentityCreateTransitionV0 {
    // When signing, we don't sign the signatures for keys
    #[platform_signable(into = "Vec<IdentityPublicKeyInCreationSignable>")]
    pub public_keys: Vec<IdentityPublicKeyInCreation>,
    pub asset_lock_proof: AssetLockProof,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
    #[cfg_attr(feature = "serde-conversion", serde(skip))]
    #[platform_signable(exclude_from_sig_hash)]
    pub identity_id: Identifier,
}

#[cfg_attr(
    feature = "serde-conversion",
    derive(Deserialize),
    serde(rename_all = "camelCase")
)]
struct IdentityCreateTransitionV0Inner {
    // Own ST fields
    public_keys: Vec<IdentityPublicKeyInCreation>,
    asset_lock_proof: AssetLockProof,
    // Generic identity ST fields
    user_fee_increase: UserFeeIncrease,
    signature: BinaryData,
}

impl TryFrom<IdentityCreateTransitionV0Inner> for IdentityCreateTransitionV0 {
    type Error = ProtocolError;

    fn try_from(value: IdentityCreateTransitionV0Inner) -> Result<Self, Self::Error> {
        let IdentityCreateTransitionV0Inner {
            public_keys,
            asset_lock_proof,
            user_fee_increase,
            signature,
        } = value;
        let identity_id = asset_lock_proof.create_identifier()?;
        Ok(Self {
            public_keys,
            asset_lock_proof,
            user_fee_increase,
            signature,
            identity_id,
        })
    }
}

impl IdentityCreateTransitionV0 {
    pub fn try_from_identity_v0(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
    ) -> Result<Self, ProtocolError> {
        let mut identity_create_transition = IdentityCreateTransitionV0::default();

        let public_keys = identity
            .public_keys()
            .values()
            .map(|public_key| public_key.into())
            .collect::<Vec<IdentityPublicKeyInCreation>>();
        identity_create_transition.set_public_keys(public_keys);

        identity_create_transition.set_asset_lock_proof(asset_lock_proof)?;

        Ok(identity_create_transition)
    }

    pub fn try_from_identity(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_conversion_versions
            .identity_to_identity_create_transition
        {
            0 => Self::try_from_identity_v0(identity, asset_lock_proof),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreateTransitionV0::try_from_identity".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
    use crate::state_transition::{
        StateTransitionHasUserFeeIncrease, StateTransitionLike, StateTransitionOwned,
        StateTransitionSingleSigned, StateTransitionType,
    };
    use platform_value::BinaryData;

    fn make_create_v0() -> IdentityCreateTransitionV0 {
        IdentityCreateTransitionV0 {
            public_keys: vec![],
            asset_lock_proof: AssetLockProof::default(),
            user_fee_increase: 0,
            signature: [0u8; 65].to_vec().into(),
            identity_id: Identifier::random(),
        }
    }

    #[test]
    fn test_default() {
        let t = IdentityCreateTransitionV0::default();
        assert_eq!(t.user_fee_increase, 0);
        assert!(t.public_keys.is_empty());
        assert!(t.signature.is_empty());
    }

    #[test]
    fn test_state_transition_like() {
        let t = make_create_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityCreate
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.identity_id]);
    }

    #[test]
    fn test_unique_identifiers() {
        let t = make_create_v0();
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
        assert!(!ids[0].is_empty());
    }

    #[test]
    fn test_owner_id() {
        let t = make_create_v0();
        assert_eq!(t.owner_id(), t.identity_id);
    }

    #[test]
    fn test_user_fee_increase() {
        let mut t = make_create_v0();
        assert_eq!(t.user_fee_increase(), 0);
        t.set_user_fee_increase(7);
        assert_eq!(t.user_fee_increase(), 7);
    }

    #[test]
    fn test_single_signed() {
        let mut t = make_create_v0();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2, 3]));
        assert_eq!(t.signature().as_slice(), &[1, 2, 3]);
        t.set_signature_bytes(vec![4, 5]);
        assert_eq!(t.signature().as_slice(), &[4, 5]);
    }

    #[test]
    fn test_into_state_transition() {
        use crate::state_transition::StateTransition;
        let t = make_create_v0();
        let st: StateTransition = t.into();
        match st {
            StateTransition::IdentityCreate(_) => {}
            _ => panic!("expected IdentityCreate"),
        }
    }

    #[test]
    fn test_accessors() {
        let mut t = make_create_v0();
        assert!(t.public_keys().is_empty());
        assert_eq!(t.identity_id(), t.identity_id);

        // Test set_public_keys and add_public_keys
        t.set_public_keys(vec![]);
        assert!(t.public_keys().is_empty());
    }

    // Legacy `StateTransitionValueConvert` round-trip tests deleted in
    // Phase D step 9. The canonical `JsonConvertible` / `ValueConvertible`
    // round-trip is exercised via the outer enum derive — these tested
    // methods that no longer exist.

    fn chain_proof() -> AssetLockProof {
        use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        AssetLockProof::Chain(ChainAssetLockProof::new(42, [3u8; 36]))
    }

    #[test]
    fn try_from_inner_populates_identity_id_from_asset_lock_proof() {
        let proof = chain_proof();
        let inner = IdentityCreateTransitionV0Inner {
            public_keys: vec![],
            asset_lock_proof: proof.clone(),
            user_fee_increase: 0,
            signature: BinaryData::new(vec![]),
        };
        let t = IdentityCreateTransitionV0::try_from(inner).expect("try_from");
        let expected = proof.create_identifier().expect("create_identifier");
        assert_eq!(t.identity_id, expected);
    }

    #[test]
    fn asset_lock_proved_sets_identity_id_and_proof() {
        let mut t = IdentityCreateTransitionV0 {
            public_keys: vec![],
            asset_lock_proof: chain_proof(),
            user_fee_increase: 0,
            signature: [0u8; 65].to_vec().into(),
            identity_id: Identifier::random(),
        };
        let original_identity_id = t.identity_id;
        // Use a different chain proof (different outpoint) to produce a new identity_id.
        use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        let proof = AssetLockProof::Chain(ChainAssetLockProof::new(99, [7u8; 36]));
        t.set_asset_lock_proof(proof.clone())
            .expect("set_asset_lock_proof");
        let expected_id = proof.create_identifier().expect("create_identifier");
        assert_eq!(t.identity_id, expected_id);
        assert_ne!(original_identity_id, t.identity_id);
        assert_eq!(t.asset_lock_proof(), &proof);
    }

    #[test]
    fn accessors_manipulate_public_keys() {
        use crate::identity::{KeyType, Purpose, SecurityLevel};
        use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
        use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
        let mut t = make_create_v0();
        assert!(t.public_keys().is_empty());

        let key = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: 0,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            signature: BinaryData::new(vec![]),
        });
        t.set_public_keys(vec![key.clone()]);
        assert_eq!(t.public_keys().len(), 1);

        // Access mutable
        let key2 = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: 1,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![1u8; 33]),
            signature: BinaryData::new(vec![]),
        });
        let mut more = vec![key2.clone()];
        t.add_public_keys(&mut more);
        assert_eq!(t.public_keys().len(), 2);
        assert!(more.is_empty(), "add_public_keys drains input Vec");
    }

    #[test]
    fn try_from_identity_v0_constructs_from_identity() {
        use crate::identity::accessors::IdentityGettersV0;
        use crate::identity::{Identity, IdentityV0};
        use std::collections::BTreeMap;

        let identity_v0 = IdentityV0 {
            id: Identifier::random(),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        };
        let identity: Identity = identity_v0.into();
        let proof = chain_proof();
        let t = IdentityCreateTransitionV0::try_from_identity_v0(&identity, proof.clone())
            .expect("try_from_identity_v0");
        assert!(t.public_keys.is_empty());
        assert_eq!(t.asset_lock_proof, proof);
        // identity_id is from the proof, NOT the identity itself.
        let expected = proof.create_identifier().expect("create_identifier");
        assert_eq!(t.identity_id, expected);
        // Demonstrate: the transition's identity_id may differ from identity.id()
        let _ = identity.id();
    }

    #[test]
    fn try_from_identity_dispatches_v0_version() {
        use crate::identity::{Identity, IdentityV0};
        use crate::version::LATEST_PLATFORM_VERSION;
        use std::collections::BTreeMap;

        let identity_v0 = IdentityV0 {
            id: Identifier::random(),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        };
        let identity: Identity = identity_v0.into();
        let t = IdentityCreateTransitionV0::try_from_identity(
            &identity,
            chain_proof(),
            LATEST_PLATFORM_VERSION,
        )
        .expect("try_from_identity");
        assert!(t.public_keys.is_empty());
    }

    /// Verifies that `try_from_identity_with_signer` rejects an identity that
    /// contains an invalid TRANSFER+HIGH public key via the structural
    /// public-key validation, returning
    /// `ProtocolError::ConsensusError(InvalidIdentityPublicKeySecurityLevelError)`
    /// before any signing or BLS work is attempted.
    #[cfg(feature = "state-transition-signing")]
    #[tokio::test]
    async fn try_from_identity_with_signer_rejects_transfer_high_key() {
        use crate::address_funds::AddressWitness;
        use crate::consensus::basic::BasicError;
        use crate::consensus::ConsensusError;
        use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use crate::identity::signer::Signer;
        use crate::identity::v0::IdentityV0;
        use crate::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use crate::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
        use crate::version::PlatformVersion;
        use crate::{BlsModule, ProtocolError, PublicKeyValidationError};
        use async_trait::async_trait;
        use std::collections::BTreeMap;

        /// A signer that should never be invoked: pre-signing validation must
        /// fail before any signing is attempted.
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
                    "UnreachableSigner::sign must not be called when pre-signing \
                     validation rejects the transition"
                );
            }

            async fn sign_create_witness(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<AddressWitness, ProtocolError> {
                panic!(
                    "UnreachableSigner::sign_create_witness must not be called \
                     when pre-signing validation rejects the transition"
                );
            }

            fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
                false
            }
        }

        /// A BLS module that should never be invoked.
        struct UnreachableBls;

        impl BlsModule for UnreachableBls {
            fn validate_public_key(&self, _pk: &[u8]) -> Result<(), PublicKeyValidationError> {
                panic!("UnreachableBls::validate_public_key must not be called");
            }

            fn verify_signature(
                &self,
                _signature: &[u8],
                _data: &[u8],
                _public_key: &[u8],
            ) -> Result<bool, ProtocolError> {
                panic!("UnreachableBls::verify_signature must not be called");
            }

            fn private_key_to_public_key(
                &self,
                _private_key: &[u8],
            ) -> Result<Vec<u8>, ProtocolError> {
                panic!("UnreachableBls::private_key_to_public_key must not be called");
            }

            fn sign(&self, _data: &[u8], _private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
                panic!("UnreachableBls::sign must not be called");
            }
        }

        let platform_version = PlatformVersion::latest();

        // Required master key so that the identity satisfies the master-key
        // presence requirement; the failure must come specifically from the
        // invalid TRANSFER+HIGH key below.
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

        let identity: Identity = IdentityV0 {
            id: Identifier::default(),
            public_keys: BTreeMap::from([(0, master_key), (1, invalid_transfer_high_key)]),
            balance: 0,
            revision: 0,
        }
        .into();

        let result = IdentityCreateTransitionV0::try_from_identity_with_signer_and_private_key(
            &identity,
            chain_proof(),
            &[0u8; 32],
            &UnreachableSigner,
            &UnreachableBls,
            0,
            platform_version,
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
    async fn try_from_identity_with_signer_rejects_wrong_instant_asset_lock_private_key_locally() {
        use crate::address_funds::AddressWitness;
        use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use crate::identity::signer::Signer;
        use crate::identity::v0::IdentityV0;
        use crate::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use crate::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
        use crate::tests::fixtures::instant_asset_lock_proof_fixture;
        use crate::version::PlatformVersion;
        use crate::{BlsModule, ProtocolError, PublicKeyValidationError};
        use async_trait::async_trait;
        use dashcore::secp256k1::SecretKey;
        use dashcore::{Network, PrivateKey};
        use std::collections::BTreeMap;
        use std::str::FromStr;

        #[derive(Debug)]
        struct UnreachableSigner;

        #[async_trait]
        impl Signer<IdentityPublicKey> for UnreachableSigner {
            async fn sign(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<BinaryData, ProtocolError> {
                panic!("UnreachableSigner::sign must not be called");
            }

            async fn sign_create_witness(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<AddressWitness, ProtocolError> {
                panic!("UnreachableSigner::sign_create_witness must not be called");
            }

            fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
                false
            }
        }

        struct UnreachableBls;

        impl BlsModule for UnreachableBls {
            fn validate_public_key(&self, _pk: &[u8]) -> Result<(), PublicKeyValidationError> {
                panic!("UnreachableBls::validate_public_key must not be called");
            }

            fn verify_signature(
                &self,
                _signature: &[u8],
                _data: &[u8],
                _public_key: &[u8],
            ) -> Result<bool, ProtocolError> {
                panic!("UnreachableBls::verify_signature must not be called");
            }

            fn private_key_to_public_key(
                &self,
                _private_key: &[u8],
            ) -> Result<Vec<u8>, ProtocolError> {
                panic!("UnreachableBls::private_key_to_public_key must not be called");
            }

            fn sign(&self, _data: &[u8], _private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
                panic!("UnreachableBls::sign must not be called");
            }
        }

        let correct_private_key =
            PrivateKey::from_str("cSBnVM4xvxarwGQuAfQFwqDg9k5tErHUHzgWsEfD4zdwUasvqRVY")
                .expect("fixture private key");
        let wrong_private_key = PrivateKey::new(
            SecretKey::from_slice(&[2u8; 32]).expect("valid alternate private key"),
            Network::Testnet,
        );

        let identity: Identity = IdentityV0 {
            id: Identifier::default(),
            public_keys: BTreeMap::from([(
                0,
                IdentityPublicKeyV0 {
                    id: 0,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::MASTER,
                    contract_bounds: None,
                    key_type: KeyType::ECDSA_HASH160,
                    read_only: false,
                    data: BinaryData::new(vec![0u8; 20]),
                    disabled_at: None,
                }
                .into(),
            )]),
            balance: 0,
            revision: 0,
        }
        .into();

        let result = IdentityCreateTransitionV0::try_from_identity_with_signer_and_private_key(
            &identity,
            instant_asset_lock_proof_fixture(Some(correct_private_key), None),
            &wrong_private_key.inner.secret_bytes(),
            &UnreachableSigner,
            &UnreachableBls,
            0,
            PlatformVersion::latest(),
        )
        .await;

        assert!(
            matches!(result, Err(ProtocolError::Generic(ref message)) if message.contains("does not match the locked output")),
            "unexpected result: {:?}",
            result
        );
    }

    #[cfg(feature = "state-transition-signing")]
    #[tokio::test]
    async fn try_from_identity_with_signer_rejects_bad_identity_public_key_signature_locally() {
        use crate::address_funds::AddressWitness;
        use crate::consensus::signature::SignatureError;
        use crate::consensus::ConsensusError;
        use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use crate::identity::signer::Signer;
        use crate::identity::v0::IdentityV0;
        use crate::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use crate::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
        use crate::version::PlatformVersion;
        use crate::{BlsModule, ProtocolError, PublicKeyValidationError};
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

        struct UnreachableBls;

        impl BlsModule for UnreachableBls {
            fn validate_public_key(&self, _pk: &[u8]) -> Result<(), PublicKeyValidationError> {
                panic!("UnreachableBls::validate_public_key must not be called");
            }

            fn verify_signature(
                &self,
                _signature: &[u8],
                _data: &[u8],
                _public_key: &[u8],
            ) -> Result<bool, ProtocolError> {
                panic!("UnreachableBls::verify_signature must not be called");
            }

            fn private_key_to_public_key(
                &self,
                _private_key: &[u8],
            ) -> Result<Vec<u8>, ProtocolError> {
                panic!("UnreachableBls::private_key_to_public_key must not be called");
            }

            fn sign(&self, _data: &[u8], _private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
                panic!("UnreachableBls::sign must not be called");
            }
        }

        let secp = Secp256k1::new();
        let correct_secret_key =
            SecretKey::from_slice(&[1u8; 32]).expect("valid identity secret key");
        let wrong_secret_key =
            SecretKey::from_slice(&[2u8; 32]).expect("valid alternate secret key");
        let correct_public_key = RawPublicKey::from_secret_key(&secp, &correct_secret_key);

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
                    data: BinaryData::new(correct_public_key.serialize().to_vec()),
                    disabled_at: None,
                }
                .into(),
            )]),
            balance: 0,
            revision: 0,
        }
        .into();

        let result = IdentityCreateTransitionV0::try_from_identity_with_signer_and_private_key(
            &identity,
            chain_proof(),
            &[3u8; 32],
            &WrongIdentityKeySigner { wrong_secret_key },
            &UnreachableBls,
            0,
            PlatformVersion::latest(),
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
