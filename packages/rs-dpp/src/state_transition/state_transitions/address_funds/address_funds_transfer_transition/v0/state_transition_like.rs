use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::prelude::UserFeeIncrease;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;

use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use crate::state_transition::StateTransitionType::{IdentityCreditTransfer, UTXOTransfer};
use crate::state_transition::{
    StateTransition, StateTransitionMultiSigned, StateTransitionSingleSigned,
};
use crate::version::FeatureVersion;

impl From<AddressFundsTransferTransitionV0> for StateTransition {
    fn from(value: AddressFundsTransferTransitionV0) -> Self {
        let utxo_transfer_transition: AddressFundsTransferTransition = value.into();
        utxo_transfer_transition.into()
    }
}

impl StateTransitionLike for AddressFundsTransferTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        UTXOTransfer
    }

    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![]
    }

    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }

    /// We want things to be unique based on the nonce, so we don't add the transition type
    fn unique_identifiers(&self) -> Vec<String> {
        vec![format!(
            "{}-{:x}",
            BASE64_STANDARD.encode(self.identity_id),
            self.nonce
        )]
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.user_fee_increase = user_fee_increase
    }
}

impl StateTransitionMultiSigned for AddressFundsTransferTransitionV0 {
    fn signatures(&self) -> &Vec<BinaryData> {
        &self.input_signatures
    }

    fn set_signatures(&mut self, signatures: Vec<BinaryData>) {
        self.input_signatures = signatures;
    }
}
