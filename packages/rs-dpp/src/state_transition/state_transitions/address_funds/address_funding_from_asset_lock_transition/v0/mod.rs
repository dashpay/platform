mod proved;
mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use crate::ProtocolError;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::prelude::{AddressNonce, UserFeeIncrease};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

mod property_names {
    pub const ASSET_LOCK_PROOF: &str = "assetLockProof";
    pub const INPUTS: &str = "inputs";
    pub const OUTPUTS: &str = "outputs";
    pub const FEE_STRATEGY: &str = "feeStrategy";
    pub const SIGNATURE: &str = "signature";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TRANSITION_TYPE: &str = "type";
}

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, PartialEq, Encode, Decode, PlatformSignable)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct AddressFundingFromAssetLockTransitionV0 {
    pub asset_lock_proof: AssetLockProof,
    /// Inputs from existing platform addresses (optional, for combining funds)
    #[cfg_attr(
        feature = "json-conversion",
        serde(with = "crate::address_funds::serde_helpers::address_input_map")
    )]
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// Outputs to fund platform addresses.
    /// - `Some(credits)` = explicit amount to send to this address
    /// - `None` = this address receives everything remaining after explicit outputs and fees
    ///   Exactly one output must be `None` to receive the remainder
    ///   (ensures full asset lock consumption).
    #[cfg_attr(
        feature = "json-conversion",
        serde(with = "crate::address_funds::serde_helpers::address_output_map_optional_amount")
    )]
    pub outputs: BTreeMap<PlatformAddress, Option<Credits>>,
    pub fee_strategy: AddressFundsFeeStrategy,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::AddressFundsFeeStrategyStep;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::identity::signer::Signer;
    use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use crate::identity::state_transition::AssetLockProved;
    use crate::state_transition::address_funding_from_asset_lock_transition::methods::AddressFundingFromAssetLockTransitionMethodsV0;
    use crate::state_transition::{StateTransitionLike, StateTransitionType};
    use crate::tests::fixtures::instant_asset_lock_proof_fixture;
    use async_trait::async_trait;
    use dashcore::secp256k1::SecretKey;
    use dashcore::{Network, OutPoint, PrivateKey};
    use platform_version::version::PlatformVersion;
    use std::str::FromStr;

    fn make_transition() -> AddressFundingFromAssetLockTransitionV0 {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0, 1_000_000));

        let mut outputs = BTreeMap::new();
        outputs.insert(PlatformAddress::P2pkh([2u8; 20]), Some(500_000));
        outputs.insert(PlatformAddress::P2pkh([3u8; 20]), None);

        AddressFundingFromAssetLockTransitionV0 {
            asset_lock_proof: AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height: 100,
                out_point: OutPoint::from([11u8; 36]),
            }),
            inputs,
            outputs,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 0,
            signature: BinaryData::new(vec![1u8; 65]),
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            }],
        }
    }

    #[test]
    fn test_state_transition_like_type() {
        let t = make_transition();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::AddressFundingFromAssetLock
        );
    }

    #[test]
    fn test_state_transition_like_protocol_version_is_zero() {
        let t = make_transition();
        assert_eq!(t.state_transition_protocol_version(), 0);
    }

    #[test]
    fn test_state_transition_like_modified_ids_empty() {
        let t = make_transition();
        assert!(t.modified_data_ids().is_empty());
    }

    #[test]
    fn test_state_transition_like_unique_ids_empty() {
        let t = make_transition();
        assert!(t.unique_identifiers().is_empty());
    }

    #[test]
    fn test_asset_lock_proved_accessor() {
        let t = make_transition();
        let proof = t.asset_lock_proof();
        // It's an asset lock proof of Chain variant
        match proof {
            AssetLockProof::Chain(_) => {}
            _ => panic!("expected Chain variant"),
        }
    }

    #[test]
    fn test_set_asset_lock_proof() {
        let mut t = make_transition();
        let new_proof = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 200,
            out_point: OutPoint::from([99u8; 36]),
        });
        t.set_asset_lock_proof(new_proof.clone()).unwrap();
        if let AssetLockProof::Chain(new) = t.asset_lock_proof() {
            assert_eq!(new.core_chain_locked_height, 200);
        } else {
            panic!("expected Chain");
        }
    }

    #[test]
    fn test_state_transition_single_signed() {
        use crate::state_transition::StateTransitionSingleSigned;
        let mut t = make_transition();
        assert_eq!(t.signature().as_slice(), &vec![1u8; 65][..]);
        t.set_signature(BinaryData::new(vec![9u8; 65]));
        assert_eq!(t.signature().as_slice(), &vec![9u8; 65][..]);
        t.set_signature_bytes(vec![5u8; 65]);
        assert_eq!(t.signature().as_slice(), &vec![5u8; 65][..]);
    }

    #[test]
    fn test_state_transition_user_fee_increase() {
        use crate::state_transition::StateTransitionHasUserFeeIncrease;
        let mut t = make_transition();
        assert_eq!(t.user_fee_increase(), 0);
        t.set_user_fee_increase(7);
        assert_eq!(t.user_fee_increase(), 7);
    }

    #[test]
    fn test_state_transition_witness_signed() {
        use crate::state_transition::StateTransitionWitnessSigned;
        let mut t = make_transition();
        assert_eq!(t.inputs().len(), 1);
        assert_eq!(t.witnesses().len(), 1);

        t.inputs_mut().clear();
        assert!(t.inputs().is_empty());

        let mut new_inputs = BTreeMap::new();
        new_inputs.insert(PlatformAddress::P2pkh([4u8; 20]), (1, 500_000));
        t.set_inputs(new_inputs);
        assert_eq!(t.inputs().len(), 1);

        t.set_witnesses(vec![]);
        assert!(t.witnesses().is_empty());
    }

    #[test]
    fn test_accessors_outputs() {
        use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0Signable;
        let mut t = make_transition();
        assert_eq!(t.outputs.len(), 2);

        let mut new_outputs = BTreeMap::new();
        new_outputs.insert(PlatformAddress::P2pkh([9u8; 20]), None);
        t.outputs = new_outputs;
        assert_eq!(t.outputs.len(), 1);

        // signable type just to keep the import alive — it's a generated Signable shadow type
        let _: AddressFundingFromAssetLockTransitionV0Signable = (&t).into();
    }

    #[test]
    fn test_feature_versioned() {
        use crate::state_transition::FeatureVersioned;
        let t = make_transition();
        assert_eq!(t.feature_version(), 0);
    }

    #[test]
    fn test_default_impl() {
        // Default constructs a transition with empty collections and default asset lock proof.
        let t = AddressFundingFromAssetLockTransitionV0::default();
        assert!(t.inputs.is_empty());
        assert!(t.outputs.is_empty());
        assert!(t.fee_strategy.is_empty());
        assert_eq!(t.user_fee_increase, 0);
        assert!(t.input_witnesses.is_empty());
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

    #[tokio::test]
    async fn constructor_returns_not_active_before_structure_validation() {
        let mut low_version = PlatformVersion::get(1)
            .expect("platform version 1 exists")
            .clone();
        low_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .address_funds_from_asset_lock
            .basic_structure = None;
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (1, 1));
        let mut outputs = BTreeMap::new();
        outputs.insert(PlatformAddress::P2pkh([2u8; 20]), None);

        let result =
            AddressFundingFromAssetLockTransitionV0::try_from_asset_lock_with_signer_and_private_key(
                AssetLockProof::Chain(ChainAssetLockProof::new(42, [3u8; 36])),
                &[7u8; 32],
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(99)],
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

    #[tokio::test]
    async fn constructor_rejects_wrong_asset_lock_private_key_locally() {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (1, 1_000_000));
        let mut outputs = BTreeMap::new();
        outputs.insert(PlatformAddress::P2pkh([2u8; 20]), None);

        let correct_private_key =
            PrivateKey::from_str("cSBnVM4xvxarwGQuAfQFwqDg9k5tErHUHzgWsEfD4zdwUasvqRVY")
                .expect("fixture private key");
        let wrong_private_key = PrivateKey::new(
            SecretKey::from_slice(&[2u8; 32]).expect("valid alternate private key"),
            Network::Testnet,
        );

        let result =
            AddressFundingFromAssetLockTransitionV0::try_from_asset_lock_with_signer_and_private_key(
                instant_asset_lock_proof_fixture(Some(correct_private_key), None),
                &wrong_private_key.inner.secret_bytes(),
                inputs,
                outputs,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                &UnreachableAddressSigner,
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
}
