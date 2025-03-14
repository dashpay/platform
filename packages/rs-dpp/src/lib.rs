#![cfg_attr(docsrs, feature(doc_cfg))]
// Coding conventions .
// #![forbid(unsafe_code)]
//#![deny(missing_docs)]
#![allow(dead_code)]

extern crate core;

pub use dashcore;

#[cfg(feature = "client")]
pub use dash_platform_protocol::DashPlatformProtocol;
pub use errors::{
    CompatibleProtocolVersionIsNotDefinedError, DPPError, DashPlatformProtocolInitError,
    InvalidVectorSizeError, NonConsensusError, ProtocolError, PublicKeyValidationError,
    SerdeParsingError,
};

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

pub mod validation;

#[cfg(feature = "client")]
pub mod dash_platform_protocol;

mod bls;

#[cfg(feature = "fixtures-and-mocks")]
pub mod tests;

pub mod asset_lock;
pub mod balances;
pub mod block;
/// Core subsidy
pub mod core_subsidy;
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

pub mod tokens;

pub mod voting;

#[cfg(feature = "core-types")]
pub mod core_types;

pub mod group;
pub mod withdrawal;

pub use async_trait;

pub use bls::*;

pub mod prelude {
    pub use crate::identifier::Identifier;
    #[cfg(feature = "validation")]
    pub use crate::validation::ConsensusValidationResult;

    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type EpochInterval = u16;

    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type BlockHeight = u64;

    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type FeeMultiplier = u64;

    pub type BlockHeightInterval = u64;

    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type CoreBlockHeight = u32;
    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type TimestampMillis = u64;

    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type TimestampMillisInterval = u64;

    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type StartAtIncluded = bool;

    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type TimestampIncluded = bool;
    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type Revision = u64;
    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type IdentityNonce = u64;
    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type SenderKeyIndex = u32;
    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type RecipientKeyIndex = u32;

    /// The index of the user's key that is used to derive keys that will be used to encrypt the contact's user id in encToUserId and the private data.
    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type RootEncryptionKeyIndex = u32;

    /// The index at which to derive the root encryption key.
    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type DerivationEncryptionKeyIndex = u32;

    /// UserFeeIncrease is the additional percentage of the processing fee.
    /// A 1 here means we pay 1% more in processing fees. A 100 means we pay 100% more.
    #[cfg_attr(feature = "apple", ferment_macro::export)]
    pub type UserFeeIncrease = u16;
}

pub use bincode;
#[cfg(feature = "bls-signatures")]
pub use dashcore::blsful as bls_signatures;
#[cfg(feature = "ed25519-dalek")]
pub use dashcore::ed25519_dalek;
#[cfg(feature = "system_contracts")]
pub use data_contracts;
#[cfg(feature = "jsonschema")]
pub use jsonschema;
pub use platform_serialization;
pub use platform_serialization_derive;
pub use platform_value;
