mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::{AddressNonce, UserFeeIncrease};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreationSignable;
use crate::ProtocolError;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, PartialEq, Encode, Decode, PlatformSignable)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
// There is a problem deriving bincode for a borrowed vector
// Hence we set to do it somewhat manually inside the PlatformSignable proc macro
// Instead of inside "bincode_derive"
#[platform_signable(derive_bincode_with_borrowed_vec)]
#[derive(Default)]
pub struct IdentityCreateFromAddressesTransitionV0 {
    // When signing, we don't sign the signatures for keys
    #[platform_signable(into = "Vec<IdentityPublicKeyInCreationSignable>")]
    pub public_keys: Vec<IdentityPublicKeyInCreation>,
    #[cfg_attr(
        feature = "json-conversion",
        serde(with = "crate::address_funds::serde_helpers::address_input_map")
    )]
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// Optional output to send remaining credits to an address
    #[cfg_attr(
        feature = "json-conversion",
        serde(with = "crate::address_funds::serde_helpers::address_output_singular")
    )]
    pub output: Option<(PlatformAddress, Credits)>,
    pub fee_strategy: AddressFundsFeeStrategy,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::AddressFundsFeeStrategyStep;
    use crate::consensus::basic::BasicError;
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use crate::state_transition::StateTransitionStructureValidation;
    use platform_value::BinaryData;
    use platform_version::version::PlatformVersion;

    fn pv() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    fn make_master_key() -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: 0,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            signature: BinaryData::new(vec![]),
        })
    }

    fn make_witness() -> AddressWitness {
        AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0u8; 65]),
        }
    }

    fn make_valid() -> IdentityCreateFromAddressesTransitionV0 {
        let v = pv();
        let min_input = v.dpp.state_transitions.address_funds.min_input_amount;
        let min_funding = v
            .dpp
            .state_transitions
            .address_funds
            .min_identity_funding_amount;
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlatformAddress::P2pkh([1u8; 20]),
            (1u32, min_input.max(min_funding) * 2),
        );
        IdentityCreateFromAddressesTransitionV0 {
            public_keys: vec![make_master_key()],
            inputs,
            output: None,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 0,
            input_witnesses: vec![make_witness()],
        }
    }

    #[test]
    fn validate_structure_valid() {
        let t = make_valid();
        let r = t.validate_structure(pv());
        assert!(r.is_valid(), "{:?}", r.errors);
    }

    #[test]
    fn validate_structure_no_inputs() {
        let mut t = make_valid();
        t.inputs.clear();
        t.input_witnesses.clear();
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::TransitionNoInputsError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_no_public_keys() {
        let mut t = make_valid();
        t.public_keys.clear();
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::MissingMasterPublicKeyError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_too_many_public_keys() {
        let mut t = make_valid();
        let max = pv()
            .dpp
            .state_transitions
            .identities
            .max_public_keys_in_creation as usize;
        // Populate > max with distinct data
        for i in 0..=max {
            t.public_keys.push(IdentityPublicKeyInCreation::V0(
                IdentityPublicKeyInCreationV0 {
                    id: i as u32 + 10,
                    key_type: KeyType::ECDSA_SECP256K1,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::HIGH,
                    contract_bounds: None,
                    read_only: false,
                    data: BinaryData::new(vec![i as u8 + 1; 33]),
                    signature: BinaryData::new(vec![]),
                },
            ));
        }
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::StateError(
                StateError::MaxIdentityPublicKeyLimitReachedError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_input_witness_mismatch() {
        let mut t = make_valid();
        t.input_witnesses.clear();
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputWitnessCountMismatchError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_output_is_input() {
        let mut t = make_valid();
        let (addr, _) = t.inputs.iter().next().unwrap();
        t.output = Some((*addr, 500_000));
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::OutputAddressAlsoInputError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_fee_strategy_empty() {
        let mut t = make_valid();
        t.fee_strategy.clear();
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyEmptyError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_fee_strategy_duplicate() {
        let mut t = make_valid();
        t.fee_strategy = vec![
            AddressFundsFeeStrategyStep::DeductFromInput(0),
            AddressFundsFeeStrategyStep::DeductFromInput(0),
        ];
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyDuplicateError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_fee_strategy_index_out_of_bounds_input() {
        let mut t = make_valid();
        t.fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(42)];
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyIndexOutOfBoundsError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_fee_strategy_index_out_of_bounds_output() {
        let mut t = make_valid();
        t.fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyIndexOutOfBoundsError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_input_below_minimum() {
        let mut t = make_valid();
        let addr = t.inputs.keys().next().cloned().unwrap();
        t.inputs.insert(addr, (1, 1));
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputBelowMinimumError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_output_below_minimum() {
        let mut t = make_valid();
        t.output = Some((PlatformAddress::P2pkh([2u8; 20]), 1));
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::OutputBelowMinimumError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_input_sum_less_than_required() {
        let mut t = make_valid();
        let addr = t.inputs.keys().next().cloned().unwrap();
        t.inputs.insert(
            addr,
            (1, pv().dpp.state_transitions.address_funds.min_input_amount),
        );
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputsNotLessThanOutputsError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_overflow_on_input_sum() {
        let mut t = make_valid();
        t.inputs.clear();
        t.input_witnesses.clear();
        t.inputs
            .insert(PlatformAddress::P2pkh([10u8; 20]), (1, u64::MAX));
        t.inputs
            .insert(PlatformAddress::P2pkh([11u8; 20]), (1, u64::MAX));
        t.input_witnesses.push(make_witness());
        t.input_witnesses.push(make_witness());
        let r = t.validate_structure(pv());
        assert!(matches!(
            r.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::OverflowError(_))]
        ));
    }

    #[test]
    fn into_state_transition_wraps_correctly() {
        use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
        let t = make_valid();
        let outer: IdentityCreateFromAddressesTransition = t.into();
        assert!(matches!(
            outer,
            IdentityCreateFromAddressesTransition::V0(_)
        ));
    }

    /// Verifies that `try_from_inputs_with_signer` rejects an identity whose
    /// public keys violate purpose/security-level constraints (TRANSFER + HIGH)
    /// via the structural public-key validation, returning
    /// `ProtocolError::ConsensusError(InvalidIdentityPublicKeySecurityLevelError)`
    /// before any signer is invoked.
    #[cfg(feature = "state-transition-signing")]
    #[tokio::test]
    async fn try_from_inputs_with_signer_rejects_transfer_high_key() {
        use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
        use crate::consensus::ConsensusError;
        use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use crate::identity::signer::Signer;
        use crate::identity::v0::IdentityV0;
        use crate::identity::{Identity, IdentityPublicKey, KeyType, SecurityLevel};
        use crate::prelude::Identifier;
        use crate::state_transition::identity_create_from_addresses_transition::methods::IdentityCreateFromAddressesTransitionMethodsV0;
        use crate::version::PlatformVersion;
        use async_trait::async_trait;

        /// A signer over `IdentityPublicKey` that should never be invoked.
        #[derive(Debug)]
        struct UnreachableIdentityKeySigner;

        #[async_trait]
        impl Signer<IdentityPublicKey> for UnreachableIdentityKeySigner {
            async fn sign(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<BinaryData, ProtocolError> {
                panic!(
                    "UnreachableIdentityKeySigner::sign must not be called when \
                     pre-signing validation rejects the transition"
                );
            }

            async fn sign_create_witness(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<AddressWitness, ProtocolError> {
                panic!(
                    "UnreachableIdentityKeySigner::sign_create_witness must not \
                     be called when pre-signing validation rejects the transition"
                );
            }

            fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
                false
            }
        }

        /// A signer over `PlatformAddress` that should never be invoked.
        #[derive(Debug)]
        struct UnreachableAddressSigner;

        #[async_trait]
        impl Signer<PlatformAddress> for UnreachableAddressSigner {
            async fn sign(
                &self,
                _key: &PlatformAddress,
                _data: &[u8],
            ) -> Result<BinaryData, ProtocolError> {
                panic!(
                    "UnreachableAddressSigner::sign must not be called when \
                     pre-signing validation rejects the transition"
                );
            }

            async fn sign_create_witness(
                &self,
                _key: &PlatformAddress,
                _data: &[u8],
            ) -> Result<AddressWitness, ProtocolError> {
                panic!(
                    "UnreachableAddressSigner::sign_create_witness must not be \
                     called when pre-signing validation rejects the transition"
                );
            }

            fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
                false
            }
        }

        let platform_version = PlatformVersion::latest();

        // Required master key so that the identity satisfies the master-key
        // presence requirement; the failure must come specifically from the
        // invalid TRANSFER+HIGH key below.
        let master_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 0,
            purpose: crate::identity::Purpose::AUTHENTICATION,
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
            purpose: crate::identity::Purpose::TRANSFER,
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

        // Inputs themselves are structurally valid; pre-signing validation must
        // still fail because of the invalid public key on the identity.
        let min_input = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount;
        let min_funding = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_identity_funding_amount;
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlatformAddress::P2pkh([1u8; 20]),
            (1u32, min_input.max(min_funding) * 2),
        );

        let result = IdentityCreateFromAddressesTransitionV0::try_from_inputs_with_signer(
            &identity,
            inputs,
            None,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            &UnreachableIdentityKeySigner,
            &UnreachableAddressSigner,
            0,
            platform_version,
        )
        .await;

        match result {
            Err(ProtocolError::ConsensusError(boxed)) => match *boxed {
                ConsensusError::BasicError(
                    BasicError::InvalidIdentityPublicKeySecurityLevelError(err),
                ) => {
                    assert_eq!(err.purpose(), crate::identity::Purpose::TRANSFER);
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
    async fn try_from_inputs_with_signer_rejects_bad_identity_public_key_signature_locally() {
        use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
        use crate::consensus::signature::SignatureError;
        use crate::consensus::ConsensusError;
        use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use crate::identity::signer::Signer;
        use crate::identity::v0::IdentityV0;
        use crate::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use crate::prelude::Identifier;
        use crate::state_transition::identity_create_from_addresses_transition::methods::IdentityCreateFromAddressesTransitionMethodsV0;
        use crate::version::PlatformVersion;
        use async_trait::async_trait;
        use dashcore::secp256k1::{PublicKey as RawPublicKey, Secp256k1, SecretKey};
        use std::collections::BTreeMap;
        use std::sync::atomic::{AtomicUsize, Ordering};

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

        #[derive(Debug, Default)]
        struct RecordingAddressSigner {
            sign_create_witness_calls: AtomicUsize,
        }

        #[async_trait]
        impl Signer<PlatformAddress> for RecordingAddressSigner {
            async fn sign(
                &self,
                _key: &PlatformAddress,
                _data: &[u8],
            ) -> Result<BinaryData, ProtocolError> {
                Err(ProtocolError::Generic(
                    "address signer should not be called before PoP verification".to_string(),
                ))
            }

            async fn sign_create_witness(
                &self,
                _key: &PlatformAddress,
                _data: &[u8],
            ) -> Result<AddressWitness, ProtocolError> {
                self.sign_create_witness_calls
                    .fetch_add(1, Ordering::SeqCst);
                Err(ProtocolError::Generic(
                    "address signer should not be called before PoP verification".to_string(),
                ))
            }

            fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
                true
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

        let platform_version = PlatformVersion::latest();
        let min_input = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount;
        let min_funding = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_identity_funding_amount;
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlatformAddress::P2pkh([1u8; 20]),
            (1u32, min_input.max(min_funding) * 2),
        );

        let identity_public_key_signer = WrongIdentityKeySigner { wrong_secret_key };
        let address_signer = RecordingAddressSigner::default();

        let result = IdentityCreateFromAddressesTransitionV0::try_from_inputs_with_signer(
            &identity,
            inputs,
            None,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            &identity_public_key_signer,
            &address_signer,
            0,
            platform_version,
        )
        .await;

        assert_eq!(
            address_signer
                .sign_create_witness_calls
                .load(Ordering::SeqCst),
            0,
            "address signer should not be reached when PoP self-check fails"
        );

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

    #[tokio::test]
    async fn try_from_inputs_with_signer_returns_not_active_before_structure_validation() {
        use crate::address_funds::AddressFundsFeeStrategyStep;
        use crate::address_funds::AddressWitness;
        use crate::consensus::basic::BasicError;
        use crate::consensus::ConsensusError;
        use crate::identity::signer::Signer;
        use crate::identity::v0::IdentityV0;
        use crate::identity::{Identity, IdentityPublicKey};
        use crate::state_transition::identity_create_from_addresses_transition::methods::IdentityCreateFromAddressesTransitionMethodsV0;
        use async_trait::async_trait;
        use platform_value::{BinaryData, Identifier};
        use platform_version::version::PlatformVersion;
        use std::collections::BTreeMap;

        #[derive(Debug)]
        struct UnreachableIdentityKeySigner;

        #[async_trait]
        impl Signer<IdentityPublicKey> for UnreachableIdentityKeySigner {
            async fn sign(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<BinaryData, ProtocolError> {
                panic!("sign should not run when protocol gating rejects the constructor")
            }

            async fn sign_create_witness(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<AddressWitness, ProtocolError> {
                panic!(
                    "sign_create_witness should not run when protocol gating rejects the constructor"
                )
            }

            fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
                false
            }
        }

        #[derive(Debug)]
        struct UnreachableAddressSigner;

        #[async_trait]
        impl Signer<PlatformAddress> for UnreachableAddressSigner {
            async fn sign(
                &self,
                _key: &PlatformAddress,
                _data: &[u8],
            ) -> Result<BinaryData, ProtocolError> {
                panic!("sign should not run when protocol gating rejects the constructor")
            }

            async fn sign_create_witness(
                &self,
                _key: &PlatformAddress,
                _data: &[u8],
            ) -> Result<AddressWitness, ProtocolError> {
                panic!(
                    "sign_create_witness should not run when protocol gating rejects the constructor"
                )
            }

            fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
                false
            }
        }

        let mut low_version = PlatformVersion::get(1)
            .expect("platform version 1 exists")
            .clone();
        low_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_from_addresses_state_transition
            .basic_structure = None;
        let identity: Identity = IdentityV0 {
            id: Identifier::default(),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        }
        .into();
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (1u32, 1));

        let result = IdentityCreateFromAddressesTransitionV0::try_from_inputs_with_signer(
            &identity,
            inputs,
            None,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(99)],
            &UnreachableIdentityKeySigner,
            &UnreachableAddressSigner,
            0,
            &low_version,
        )
        .await;

        assert!(matches!(
            result,
            Err(ProtocolError::ConsensusError(boxed))
                if matches!(
                    *boxed,
                    ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(_))
                )
        ));
    }
}
