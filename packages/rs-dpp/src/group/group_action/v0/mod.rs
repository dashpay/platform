use crate::group::action_event::GroupActionEvent;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

#[derive(
    Debug, PartialEq, PartialOrd, Clone, Eq, Encode, Decode, PlatformDeserialize, PlatformSerialize,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
pub struct GroupActionV0 {
    pub event: GroupActionEvent,
}
