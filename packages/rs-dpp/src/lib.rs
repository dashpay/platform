mod contracts;
pub mod data_contract;
extern crate core;

pub mod data_trigger;
pub mod document;
pub mod dpp;
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

pub use dash_platform_protocol::DashPlatformProtocol;
pub use errors::*;
pub mod mocks;

#[cfg(test)]
mod tests;

mod prelude {
    pub use crate::data_contract::DataContract;
    pub use crate::data_trigger::DataTrigger;
    pub use crate::document::Document;
    pub use crate::dpp::DashPlatformProtocolLike;
    pub use crate::identifier::Identifier;
    pub use crate::identity::Identity;
}
