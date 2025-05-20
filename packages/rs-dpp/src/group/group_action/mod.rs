pub mod v0;

use crate::data_contract::TokenContractPosition;
use crate::group::action_event::GroupActionEvent;
use crate::group::group_action::v0::GroupActionV0;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;

#[derive(
    Debug, PartialEq, PartialOrd, Clone, Eq, Encode, Decode, PlatformDeserialize, PlatformSerialize,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
pub enum GroupAction {
    V0(GroupActionV0),
}

pub trait GroupActionAccessors {
    fn contract_id(&self) -> Identifier;

    fn proposer_id(&self) -> Identifier;
    fn token_contract_position(&self) -> TokenContractPosition;
    fn event(&self) -> &GroupActionEvent;
}
impl GroupActionAccessors for GroupAction {
    fn contract_id(&self) -> Identifier {
        match self {
            GroupAction::V0(inner) => inner.contract_id(),
        }
    }

    fn proposer_id(&self) -> Identifier {
        match self {
            GroupAction::V0(inner) => inner.proposer_id(),
        }
    }

    fn token_contract_position(&self) -> TokenContractPosition {
        match self {
            GroupAction::V0(inner) => inner.token_contract_position(),
        }
    }

    fn event(&self) -> &GroupActionEvent {
        match self {
            GroupAction::V0(inner) => inner.event(),
        }
    }
}
