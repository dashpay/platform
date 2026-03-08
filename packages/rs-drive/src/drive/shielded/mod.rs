/// Shielded pool paths and constants
#[cfg(any(feature = "server", feature = "verify"))]
pub mod paths;

/// Estimation costs for shielded pool operations
#[cfg(feature = "server")]
pub(crate) mod estimated_costs;

/// Prove methods for shielded pool queries
#[cfg(feature = "server")]
pub mod prove;

/// Per-block nullifier storage for catch-up sync
#[cfg(any(feature = "server", feature = "verify"))]
pub mod nullifiers;
