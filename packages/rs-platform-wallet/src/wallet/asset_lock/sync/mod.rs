//! Asset lock tracking and synchronization.
//!
//! Contains methods for tracking asset locks, advancing their lifecycle status,
//! recovering locks, waiting for proofs and chain locks, resuming interrupted
//! locks, and re-deriving private keys.

mod proof;
mod recovery;
mod tracking;
