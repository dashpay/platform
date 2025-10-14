//! Platform wallet with identity management
//!
//! This crate provides a wallet implementation that combines traditional
//! wallet functionality with Dash Platform identity management.

pub mod block_time;
pub mod contact_request;
pub mod established_contact;
pub mod identity_manager;
pub mod managed_identity;
pub mod platform_wallet_info;
pub mod error;

pub use block_time::BlockTime;
pub use contact_request::ContactRequest;
pub use established_contact::EstablishedContact;
pub use identity_manager::IdentityManager;
pub use managed_identity::ManagedIdentity;

#[cfg(feature = "manager")]
pub use key_wallet_manager;

