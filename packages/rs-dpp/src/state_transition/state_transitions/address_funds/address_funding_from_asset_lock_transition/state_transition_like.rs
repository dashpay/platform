use crate::address_funds::AddressWitness;
use crate::prelude::UserFeeIncrease;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::state_transition::{
    StateTransitionLike, StateTransitionSingleSigned, StateTransitionType,
    StateTransitionWitnessSigned,
};
use crate::version::FeatureVersion;
use platform_value::{BinaryData, Identifier};

impl StateTransitionLike for AddressFundingFromAssetLockTransition {
    /// Returns IDs of the modified data - the output addresses
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => transition.modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            AddressFundingFromAssetLockTransition::V0(_) => 0,
        }
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => {
                transition.state_transition_type()
            }
        }
    }

    /// returns the fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => transition.user_fee_increase(),
        }
    }

    /// set a fee multiplier
    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => {
                transition.set_user_fee_increase(user_fee_increase)
            }
        }
    }

    fn unique_identifiers(&self) -> Vec<String> {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => {
                transition.unique_identifiers()
            }
        }
    }
}

impl StateTransitionSingleSigned for AddressFundingFromAssetLockTransition {
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => transition.signature(),
        }
    }

    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => {
                transition.set_signature(signature)
            }
        }
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => {
                transition.set_signature_bytes(signature)
            }
        }
    }
}

impl StateTransitionWitnessSigned for AddressFundingFromAssetLockTransition {
    fn inputs(
        &self,
    ) -> &std::collections::BTreeMap<
        crate::address_funds::PlatformAddress,
        (crate::prelude::AddressNonce, crate::fee::Credits),
    > {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => transition.inputs(),
        }
    }

    fn inputs_mut(
        &mut self,
    ) -> &mut std::collections::BTreeMap<
        crate::address_funds::PlatformAddress,
        (crate::prelude::AddressNonce, crate::fee::Credits),
    > {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => transition.inputs_mut(),
        }
    }

    fn set_inputs(
        &mut self,
        inputs: std::collections::BTreeMap<
            crate::address_funds::PlatformAddress,
            (crate::prelude::AddressNonce, crate::fee::Credits),
        >,
    ) {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => transition.set_inputs(inputs),
        }
    }

    fn witnesses(&self) -> &Vec<AddressWitness> {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => transition.witnesses(),
        }
    }

    fn set_witnesses(&mut self, witnesses: Vec<AddressWitness>) {
        match self {
            AddressFundingFromAssetLockTransition::V0(transition) => {
                transition.set_witnesses(witnesses)
            }
        }
    }
}
