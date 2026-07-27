//! Identity-domain data types.
//!
//! Pure data — no logic, no mutations, no network. Split into:
//! - top-level: identity-only types
//! - `dashpay/`: types specific to the DashPay contract (contacts,
//!   requests, profile, payments).

pub mod block_time;
pub mod dashpay;
pub mod key_storage;

pub use block_time::BlockTime;
pub use dashpay::{
    ContactProfileEntry, ContactRequest, DashPayProfile, DashpayAddressMatch, EstablishedContact,
    PaymentDirection, PaymentEntry, PaymentStatus, ProfileUpdate,
};
pub use key_storage::{DpnsNameInfo, IdentityStatus, KeyStorage, PrivateKeyData};
