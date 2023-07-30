use std::fmt::Debug;

use dashcore::signer;

use platform_value::{BinaryData, ReplacementType, Value, ValueMapHelper};

use crate::consensus::signature::InvalidStateTransitionSignatureError;
use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;

use crate::serialization::{PlatformSerializable, Signable};
use crate::version::FeatureVersion;
use crate::{
    identity::KeyType,
    prelude::{Identifier, ProtocolError},
    util::hash,
    BlsModule,
};

use crate::identity::KeyID;
#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
use crate::state_transition::errors::InvalidIdentityPublicKeyTypeError;
#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
use crate::state_transition::errors::StateTransitionIsNotSignedError;
use crate::state_transition::StateTransitionType;
use crate::state_transition::{StateTransition, StateTransitionFieldTypes};

pub const DOCUMENT_TRANSITION_TYPES: [StateTransitionType; 1] =
    [StateTransitionType::DocumentsBatch];

pub const IDENTITY_TRANSITION_TYPE: [StateTransitionType; 4] = [
    StateTransitionType::IdentityCreate,
    StateTransitionType::IdentityTopUp,
    StateTransitionType::IdentityUpdate,
    StateTransitionType::IdentityCreditTransfer,
];

pub const DATA_CONTRACT_TRANSITION_TYPES: [StateTransitionType; 2] = [
    StateTransitionType::DataContractCreate,
    StateTransitionType::DataContractUpdate,
];

/// The StateTransitionLike represents set of methods that are shared for all types of State Transition.
/// Every type of state transition should also implement Debug, Clone, and support conversion to compounded [`StateTransition`]
pub trait StateTransitionLike:
    StateTransitionFieldTypes + Clone + Debug + Into<StateTransition>
{
    /// returns the protocol version
    fn state_transition_protocol_version(&self) -> FeatureVersion;
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType;
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData;
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData);
    /// get modified ids list
    fn modified_data_ids(&self) -> Vec<Identifier>;

    /// returns true if state transition is a document state transition
    fn is_document_state_transition(&self) -> bool {
        DOCUMENT_TRANSITION_TYPES.contains(&self.state_transition_type())
    }
    /// returns true if state transition is a data contract state transition
    fn is_data_contract_state_transition(&self) -> bool {
        DATA_CONTRACT_TRANSITION_TYPES.contains(&self.state_transition_type())
    }
    /// return true if state transition is an identity state transition
    fn is_identity_state_transition(&self) -> bool {
        IDENTITY_TRANSITION_TYPE.contains(&self.state_transition_type())
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>);

    /// Get owner ID
    fn owner_id(&self) -> &Identifier;
}
