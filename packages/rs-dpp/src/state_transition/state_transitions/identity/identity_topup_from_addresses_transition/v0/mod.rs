mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::{AddressNonce, Identifier, UserFeeIncrease};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::ProtocolError;

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, Encode, Decode, PlatformSignable, PartialEq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct IdentityTopUpFromAddressesTransitionV0 {
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
    pub identity_id: Identifier,
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
    use crate::consensus::ConsensusError;
    use crate::identity::signer::Signer;
    use crate::identity::v0::IdentityV0;
    use crate::identity::{Identity, IdentityPublicKey};
    use crate::state_transition::identity_topup_from_addresses_transition::methods::IdentityTopUpFromAddressesTransitionMethodsV0;
    use crate::state_transition::StateTransitionStructureValidation;
    use async_trait::async_trait;
    use platform_value::BinaryData;
    use platform_value::Identifier;
    use platform_version::version::PlatformVersion;

    fn make_witness() -> AddressWitness {
        AddressWitness::P2pkh {
            signature: platform_value::BinaryData::new(vec![0u8; 65]),
        }
    }

    fn make_valid_v0() -> IdentityTopUpFromAddressesTransitionV0 {
        // LATEST_PLATFORM_VERSION uses STATE_TRANSITION_VERSIONS_V3 which has
        // max_address_inputs = 16. Earlier (v1) state transition versions set it to 0,
        // which effectively disables these code paths.
        let pv = pv();
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        let min_funding = pv
            .dpp
            .state_transitions
            .address_funds
            .min_identity_funding_amount;
        let mut inputs = BTreeMap::new();
        inputs.insert(
            PlatformAddress::P2pkh([1u8; 20]),
            (1u32, min_input.max(min_funding) * 2),
        );
        IdentityTopUpFromAddressesTransitionV0 {
            inputs,
            output: None,
            identity_id: Identifier::random(),
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 0,
            input_witnesses: vec![make_witness()],
        }
    }

    fn pv() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    #[test]
    fn validate_structure_valid() {
        let t = make_valid_v0();
        let result = t.validate_structure(pv());
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn validate_structure_no_inputs() {
        let mut t = make_valid_v0();
        t.inputs.clear();
        t.input_witnesses.clear();
        let result = t.validate_structure(pv());
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::TransitionNoInputsError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_input_witness_count_mismatch() {
        let mut t = make_valid_v0();
        t.input_witnesses.clear();
        let result = t.validate_structure(pv());
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputWitnessCountMismatchError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_output_is_input_address() {
        let mut t = make_valid_v0();
        let (addr, _) = t.inputs.iter().next().unwrap();
        t.output = Some((*addr, 500_000));
        let result = t.validate_structure(pv());
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::OutputAddressAlsoInputError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_fee_strategy_empty() {
        let mut t = make_valid_v0();
        t.fee_strategy.clear();
        let result = t.validate_structure(pv());
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyEmptyError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_fee_strategy_duplicate() {
        let mut t = make_valid_v0();
        t.fee_strategy = vec![
            AddressFundsFeeStrategyStep::DeductFromInput(0),
            AddressFundsFeeStrategyStep::DeductFromInput(0),
        ];
        let result = t.validate_structure(pv());
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyDuplicateError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_fee_strategy_input_out_of_bounds() {
        let mut t = make_valid_v0();
        t.fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(99)];
        let result = t.validate_structure(pv());
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyIndexOutOfBoundsError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_fee_strategy_reduce_output_out_of_bounds_when_no_output() {
        let mut t = make_valid_v0();
        // No output set => ReduceOutput(0) must be out of bounds.
        t.fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];
        let result = t.validate_structure(pv());
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyIndexOutOfBoundsError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_input_below_minimum() {
        let mut t = make_valid_v0();
        let addr = t.inputs.keys().next().cloned().unwrap();
        t.inputs.insert(addr, (1, 1)); // below min_input_amount
        let result = t.validate_structure(pv());
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputBelowMinimumError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_output_below_minimum() {
        let mut t = make_valid_v0();
        // Use a valid output address, but amount below min_output_amount (500_000)
        t.output = Some((PlatformAddress::P2pkh([2u8; 20]), 1));
        let result = t.validate_structure(pv());
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::OutputBelowMinimumError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_input_sum_less_than_required() {
        let mut t = make_valid_v0();
        // Set input to min_input_amount (well below min_identity_funding_amount=200_000? No, min is 100k, funding is 200k)
        let pv = pv();
        let addr = t.inputs.keys().next().cloned().unwrap();
        t.inputs.insert(
            addr,
            (1, pv.dpp.state_transitions.address_funds.min_input_amount),
        );
        let result = t.validate_structure(pv);
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputsNotLessThanOutputsError(_)
            )]
        ));
    }

    #[test]
    fn validate_structure_inputs_overflow_when_summing() {
        let mut t = make_valid_v0();
        t.inputs.clear();
        t.input_witnesses.clear();
        // Two u64::MAX inputs will overflow on addition.
        t.inputs
            .insert(PlatformAddress::P2pkh([10u8; 20]), (1, u64::MAX));
        t.inputs
            .insert(PlatformAddress::P2pkh([11u8; 20]), (1, u64::MAX));
        t.input_witnesses.push(make_witness());
        t.input_witnesses.push(make_witness());
        let result = t.validate_structure(pv());
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::OverflowError(_))]
        ));
    }

    #[test]
    fn validate_structure_too_many_fee_strategies() {
        let mut t = make_valid_v0();
        // max_address_fee_strategies = 4 in state-transitions v3. We need >4 steps.
        // Start fresh: expand inputs + witnesses consistently, then install 5 distinct
        // fee-strategy steps whose indices are all within bounds.
        t.inputs.clear();
        t.input_witnesses.clear();
        for i in 0..5u8 {
            t.inputs
                .insert(PlatformAddress::P2pkh([20 + i; 20]), (1, 500_000));
            t.input_witnesses.push(make_witness());
        }
        t.fee_strategy = vec![
            AddressFundsFeeStrategyStep::DeductFromInput(0),
            AddressFundsFeeStrategyStep::DeductFromInput(1),
            AddressFundsFeeStrategyStep::DeductFromInput(2),
            AddressFundsFeeStrategyStep::DeductFromInput(3),
            AddressFundsFeeStrategyStep::DeductFromInput(4),
        ];
        let result = t.validate_structure(pv());
        assert!(
            matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::FeeStrategyTooManyStepsError(_)
                )]
            ),
            "{:?}",
            result.errors
        );
    }

    #[test]
    fn state_transition_like_basic() {
        use crate::state_transition::{
            StateTransitionHasUserFeeIncrease, StateTransitionLike, StateTransitionOwned,
            StateTransitionType,
        };
        let mut t = make_valid_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityTopUpFromAddresses
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.identity_id]);
        assert_eq!(t.owner_id(), t.identity_id);
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
        assert!(!ids[0].is_empty());
        assert_eq!(t.user_fee_increase(), 0);
        t.set_user_fee_increase(77);
        assert_eq!(t.user_fee_increase(), 77);
    }

    #[test]
    fn witness_signed_accessors() {
        use crate::state_transition::StateTransitionWitnessSigned;
        let mut t = make_valid_v0();
        let original_inputs = StateTransitionWitnessSigned::inputs(&t).clone();
        assert_eq!(original_inputs.len(), 1);
        let mut_ref = StateTransitionWitnessSigned::inputs_mut(&mut t);
        mut_ref.clear();
        assert!(StateTransitionWitnessSigned::inputs(&t).is_empty());
        StateTransitionWitnessSigned::set_inputs(&mut t, original_inputs.clone());
        assert_eq!(StateTransitionWitnessSigned::inputs(&t).len(), 1);
        let wits = StateTransitionWitnessSigned::witnesses(&t);
        assert_eq!(wits.len(), 1);
        StateTransitionWitnessSigned::set_witnesses(&mut t, vec![]);
        assert_eq!(StateTransitionWitnessSigned::witnesses(&t).len(), 0);
    }

    #[test]
    fn into_state_transition_wraps_correctly() {
        use crate::state_transition::StateTransition;
        let t = make_valid_v0();
        let st: StateTransition = t.into();
        assert!(matches!(st, StateTransition::IdentityTopUpFromAddresses(_)));
    }

    #[test]
    fn default_accessors() {
        use crate::state_transition::identity_topup_from_addresses_transition::accessors::IdentityTopUpFromAddressesTransitionAccessorsV0;
        let mut t = IdentityTopUpFromAddressesTransitionV0::default();
        assert!(t.inputs.is_empty());
        let new_id = Identifier::random();
        t.set_identity_id(new_id);
        assert_eq!(t.identity_id(), &new_id);
        assert!(t.output().is_none());
        t.set_output(Some((PlatformAddress::P2pkh([9u8; 20]), 1)));
        assert!(t.output().is_some());
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
    async fn try_from_inputs_with_signer_returns_not_active_before_structure_validation() {
        let mut low_version = PlatformVersion::get(1)
            .expect("platform version 1 exists")
            .clone();
        low_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_top_up_from_addresses_state_transition
            .basic_structure = None;
        let identity: Identity = IdentityV0 {
            id: Identifier::random(),
            public_keys: BTreeMap::<u32, IdentityPublicKey>::new(),
            balance: 0,
            revision: 0,
        }
        .into();
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (1u32, 1));

        let result = IdentityTopUpFromAddressesTransitionV0::try_from_inputs_with_signer(
            &identity,
            inputs,
            &UnreachableAddressSigner,
            0,
            &low_version,
            None,
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
