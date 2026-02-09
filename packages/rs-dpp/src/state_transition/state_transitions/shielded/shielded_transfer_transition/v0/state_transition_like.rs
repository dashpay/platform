use crate::prelude::UserFeeIncrease;
use crate::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
use crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::ShieldedTransfer;
use crate::version::FeatureVersion;

impl From<ShieldedTransferTransitionV0> for StateTransition {
    fn from(value: ShieldedTransferTransitionV0) -> Self {
        let transition: ShieldedTransferTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for ShieldedTransferTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        ShieldedTransfer
    }

    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![]
    }

    /// For ZK-only transitions, uniqueness comes from nullifiers in the bundle,
    /// but at the DPP level we use the hash of the orchard bundle as a unique identifier.
    fn unique_identifiers(&self) -> Vec<String> {
        use crate::util::hash::hash_single;
        vec![hex::encode(hash_single(&self.orchard_bundle))]
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}
