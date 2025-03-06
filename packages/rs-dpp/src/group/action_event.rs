use crate::tokens::token_event::TokenEvent;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

#[derive(
    Debug, PartialEq, PartialOrd, Clone, Eq, Encode, Decode, PlatformDeserialize, PlatformSerialize,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
pub enum GroupActionEvent {
    TokenEvent(TokenEvent),
}
