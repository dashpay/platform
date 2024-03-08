#![cfg_attr(docsrs, feature(doc_cfg))]
// Coding conventions
#![forbid(unsafe_code)]
//#![deny(missing_docs)]
#![allow(dead_code)]

extern crate core;

pub use dashcore;

#[cfg(feature = "client")]
pub use dash_platform_protocol::DashPlatformProtocol;
pub use errors::*;

pub mod data_contract;
pub mod document;
pub mod identifier;
pub mod identity;
pub mod metadata;
#[cfg(feature = "state-transitions")]
pub mod state_transition;
pub mod util;
pub mod version;

pub mod errors;

pub mod schema;
pub mod validation;

#[cfg(feature = "client")]
pub mod dash_platform_protocol;

mod bls;

#[cfg(feature = "fixtures-and-mocks")]
pub mod tests;

pub mod balances;
pub mod block;
pub mod fee;
pub mod serialization;
#[cfg(any(
    feature = "message-signing",
    feature = "message-signature-verification"
))]
pub mod signing;
#[cfg(feature = "system_contracts")]
pub mod system_data_contracts;
pub mod voting;
pub mod withdrawal;

pub use async_trait;
pub use bls::*;

pub mod prelude {
    pub use crate::data_contract::DataContract;
    #[cfg(feature = "extended-document")]
    pub use crate::document::ExtendedDocument;
    pub use crate::errors::ProtocolError;
    pub use crate::identifier::Identifier;
    pub use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
    pub use crate::identity::Identity;
    pub use crate::identity::IdentityPublicKey;
    #[cfg(feature = "validation")]
    pub use crate::validation::ConsensusValidationResult;
    pub type TimestampMillis = u64;
    pub type Revision = u64;
    pub type IdentityNonce = u64;
}

pub use bincode;
pub use bls_signatures;
pub use data_contracts;
pub use ed25519_dalek;
pub use jsonschema;
pub use platform_serialization;
pub use platform_value;
