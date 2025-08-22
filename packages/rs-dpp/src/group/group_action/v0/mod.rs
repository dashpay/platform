use crate::data_contract::TokenContractPosition;
use crate::group::action_event::GroupActionEvent;
use crate::group::group_action::GroupActionAccessors;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;

#[derive(
    Debug, PartialEq, PartialOrd, Clone, Eq, Encode, Decode, PlatformDeserialize, PlatformSerialize,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
pub struct GroupActionV0 {
    pub contract_id: Identifier,
    pub proposer_id: Identifier,
    pub token_contract_position: TokenContractPosition,
    pub event: GroupActionEvent,
}

impl GroupActionAccessors for GroupActionV0 {
    fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    fn proposer_id(&self) -> Identifier {
        self.proposer_id
    }

    fn token_contract_position(&self) -> TokenContractPosition {
        self.token_contract_position
    }

    fn event(&self) -> &GroupActionEvent {
        &self.event
    }
}
