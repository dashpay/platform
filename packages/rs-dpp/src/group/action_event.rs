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

use std::fmt;

impl fmt::Display for GroupActionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroupActionEvent::TokenEvent(event) => write!(f, "Token event: {}", event),
        }
    }
}

impl GroupActionEvent {
    /// Returns a reference to the public note if the variant includes one.
    pub fn public_note(&self) -> Option<&str> {
        match self {
            GroupActionEvent::TokenEvent(token_event) => token_event.public_note(),
        }
    }

    /// Returns a name of the event
    pub fn event_name(&self) -> String {
        match self {
            GroupActionEvent::TokenEvent(token_event) => {
                format!("Token: {}", token_event.associated_document_type_name())
            }
        }
    }
}
