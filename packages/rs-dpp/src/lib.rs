#![allow(dead_code)]

extern crate core;

pub use convertible::Convertible;
pub use dash_platform_protocol::DashPlatformProtocol;
pub use errors::*;

mod contracts;
pub mod data_contract;

mod convertible;
pub mod data_trigger;
pub mod decode_protocol_entity_factory;
pub mod document;
pub mod identifier;
pub mod identity;
pub mod metadata;
pub mod state_repository;
pub mod state_transition;
pub mod util;
pub mod version;

pub mod errors;

pub mod schema;
pub mod validation;

mod dash_platform_protocol;

pub mod block_time_window;
pub mod mocks;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::data_contract::DataContract;
    pub use crate::data_trigger::DataTrigger;
    pub use crate::document::document_transition::DocumentTransition;
    pub use crate::document::Document;
    pub use crate::errors::ProtocolError;
    pub use crate::identifier::Identifier;
    pub use crate::identity::Identity;
    pub use crate::identity::IdentityPublicKey;
    pub use crate::validation::ValidationResult;

    pub use super::convertible::Convertible;
    pub type TimestampMillis = u64;
    pub type Revision = u64;
}
