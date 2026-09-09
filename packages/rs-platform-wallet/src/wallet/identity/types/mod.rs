//! Identity-domain data types.
//!
//! Pure data — no logic, no mutations, no network. Split into:
//! - top-level: identity-only types
//! - `dashpay/`: types specific to the DashPay contract (contacts,
//!   requests, profile, payments).

pub mod block_time;
pub mod dashpay;
pub mod identity_status;

pub use block_time::BlockTime;
pub use dashpay::{
    ContactProfileEntry, ContactRequest, DashPayProfile, EstablishedContact, PaymentDirection,
    PaymentEntry, PaymentStatus, ProfileUpdate,
};
pub use identity_status::{DpnsNameInfo, IdentityStatus};
