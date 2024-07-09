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

pub mod asset_lock;
pub mod balances;
pub mod block;
pub mod fee;
pub mod nft;
pub mod prefunded_specialized_balance;
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

    pub type BlockHeight = u64;

    pub type CoreBlockHeight = u32;
    pub type TimestampMillis = u64;

    pub type TimestampIncluded = bool;
    pub type Revision = u64;
    pub type IdentityNonce = u64;

    /// UserFeeIncrease is the additional percentage of the processing fee.
    /// A 1 here means we pay 1% more in processing fees. A 100 means we pay 100% more.
    pub type UserFeeIncrease = u16;
}

pub use bincode;
#[cfg(all(not(target_arch = "wasm32"), feature = "bls-signatures"))]
pub use bls_signatures;
#[cfg(feature = "system_contracts")]
pub use data_contracts;
#[cfg(feature = "ed25519-dalek")]
pub use ed25519_dalek;
#[cfg(feature = "jsonschema")]
pub use jsonschema;
pub use platform_serialization;
pub use platform_value;
