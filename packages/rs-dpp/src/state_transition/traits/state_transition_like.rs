use std::fmt::Debug;

use platform_value::BinaryData;

use crate::prelude::{Identifier, UserFeeIncrease};
use crate::version::FeatureVersion;

use crate::state_transition::StateTransitionType;
use crate::state_transition::{StateTransition, StateTransitionFieldTypes};

pub const DOCUMENT_TRANSITION_TYPES: [StateTransitionType; 1] =
    [StateTransitionType::DocumentsBatch];

pub const IDENTITY_TRANSITION_TYPE: [StateTransitionType; 5] = [
    StateTransitionType::IdentityCreate,
    StateTransitionType::IdentityTopUp,
    StateTransitionType::IdentityUpdate,
    StateTransitionType::IdentityCreditTransfer,
    StateTransitionType::IdentityCreditWithdrawal,
];

pub const VOTING_TRANSITION_TYPE: [StateTransitionType; 1] = [StateTransitionType::MasternodeVote];

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
    /// returns the fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease;
    /// set a fee multiplier
    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease);
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

    /// return true if state transition is a voting state transition
    fn is_voting_state_transition(&self) -> bool {
        VOTING_TRANSITION_TYPE.contains(&self.state_transition_type())
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>);

    /// Get owner ID
    fn owner_id(&self) -> Identifier;

    /// unique identifiers for the state transition
    /// This is often only one String except in the case of a documents batch state transition
    fn unique_identifiers(&self) -> Vec<String>;
}
