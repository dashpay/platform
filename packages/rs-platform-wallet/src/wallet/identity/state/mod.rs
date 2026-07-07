//! In-memory identity state + per-identity mutations.
//!
//! - `managed_identity/`: the `ManagedIdentity` struct and the
//!   mutation impls that maintain its invariants (identity-side +
//!   DashPay-side both live here today).
//! - `manager/`: the per-wallet collection of managed identities.

pub mod managed_identity;
pub mod manager;

pub use managed_identity::{BlockTime, DashPayState, ManagedIdentity};
pub use manager::IdentityLocation;
pub use manager::IdentityManager;
pub use manager::RegistrationIndex;
