#[cfg(feature = "json-conversion")]
mod json_conversion;
mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "value-conversion")]
mod value_conversion;
mod version;

use std::collections::BTreeMap;
use std::convert::TryFrom;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::{AddressNonce, UserFeeIncrease};
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreationSignable;
use crate::ProtocolError;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Encode, Decode, PlatformSignable)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase"),
    serde(try_from = "IdentityCreateFromAddressesTransitionV0Inner")
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
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// Optional output to send remaining credits to an address
    pub output: Option<(PlatformAddress, Credits)>,
    pub fee_strategy: AddressFundsFeeStrategy,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}

#[cfg_attr(
    feature = "serde-conversion",
    derive(Deserialize),
    serde(rename_all = "camelCase")
)]
struct IdentityCreateFromAddressesTransitionV0Inner {
    // Own ST fields
    public_keys: Vec<IdentityPublicKeyInCreation>,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    output: Option<(PlatformAddress, Credits)>,
    fee_strategy: AddressFundsFeeStrategy,
    user_fee_increase: UserFeeIncrease,
    input_witnesses: Vec<AddressWitness>,
}

impl TryFrom<IdentityCreateFromAddressesTransitionV0Inner>
    for IdentityCreateFromAddressesTransitionV0
{
    type Error = ProtocolError;

    fn try_from(value: IdentityCreateFromAddressesTransitionV0Inner) -> Result<Self, Self::Error> {
        let IdentityCreateFromAddressesTransitionV0Inner {
            public_keys,
            inputs,
            output,
            fee_strategy,
            user_fee_increase,
            input_witnesses,
        } = value;

        Ok(Self {
            public_keys,
            inputs,
            output,
            fee_strategy,
            user_fee_increase,
            input_witnesses,
        })
    }
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
        t.output = Some((addr.clone(), 500_000));
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
    fn try_from_inner_roundtrip() {
        let t = make_valid();
        let inner = IdentityCreateFromAddressesTransitionV0Inner {
            public_keys: t.public_keys.clone(),
            inputs: t.inputs.clone(),
            output: t.output.clone(),
            fee_strategy: t.fee_strategy.clone(),
            user_fee_increase: t.user_fee_increase,
            input_witnesses: t.input_witnesses.clone(),
        };
        let back = IdentityCreateFromAddressesTransitionV0::try_from(inner).expect("try_from");
        assert_eq!(back, t);
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
}
