/// Shielded pool paths and constants
#[cfg(any(feature = "server", feature = "verify"))]
pub mod paths;

/// Estimation costs for shielded pool operations
#[cfg(feature = "server")]
pub(crate) mod estimated_costs;

/// Insert a note into the shielded pool commitment tree
#[cfg(feature = "server")]
mod insert_note;

/// Insert nullifiers into the permanent tree and per-block sync storage
#[cfg(feature = "server")]
mod insert_nullifiers;

/// Update the shielded pool total balance
#[cfg(feature = "server")]
mod update_total_balance;

/// Prove methods for shielded pool queries
#[cfg(feature = "server")]
pub mod prove;

/// Per-block nullifier storage for catch-up sync
#[cfg(any(feature = "server", feature = "verify"))]
pub mod nullifiers;
