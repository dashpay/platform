use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{
    data_contract::DataContract,
    identity::KeyID,
    prelude::Identifier,
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    Convertible, NonConsensusError, ProtocolError,
};

use crate::identity::signer::Signer;
use crate::identity::PartialIdentity;
use crate::serialization_traits::{PlatformDeserializable, Signable};
use crate::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionV0,
};
use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use bincode::{config, Decode, Encode};

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::{IdentityCreate, IdentityCreditTransfer};
use crate::version::FeatureVersion;

impl From<IdentityCreditTransferTransitionV0> for StateTransition {
    fn from(value: IdentityCreditTransferTransitionV0) -> Self {
        let identity_credit_transfer_transition: IdentityCreditTransferTransition = value.into();
        identity_credit_transfer_transition.into()
    }
}

impl StateTransitionLike for IdentityCreditTransferTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityCreditTransfer
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id]
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }

    /// Get owner ID
    fn owner_id(&self) -> &Identifier {
        &self.identity_id
    }
}
