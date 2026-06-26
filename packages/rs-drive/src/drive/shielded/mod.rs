/// Shielded pool paths and constants
#[cfg(any(feature = "server", feature = "verify"))]
pub mod paths;

/// Estimation costs for shielded pool operations
#[cfg(feature = "server")]
pub(crate) mod estimated_costs;

/// Insert the main shielded credit pool and its eight child subtrees.
/// Shared between the genesis-v12 and upgrade-to-v12 paths so both build a
/// byte-identical `[ShieldedBalances]` subtree (consensus-critical).
#[cfg(feature = "server")]
mod insert_shielded_pool_structure;

/// Insert a note into the shielded pool commitment tree
#[cfg(feature = "server")]
mod insert_note;

/// Insert nullifiers into the permanent tree and per-block sync storage
#[cfg(feature = "server")]
mod insert_nullifiers;

/// Update the shielded pool total balance
#[cfg(feature = "server")]
mod update_total_balance;

/// Record the shielded pool anchor if the commitment tree changed this block
#[cfg(feature = "server")]
mod record_anchor_if_changed;

/// Prune shielded pool anchors older than a given cutoff height
#[cfg(feature = "server")]
mod prune_anchors;

/// Check whether a shielded pool anchor exists
#[cfg(feature = "server")]
mod has_anchor;

/// Check whether a nullifier has already been spent
#[cfg(feature = "server")]
mod has_nullifier;

/// Read the shielded pool total balance
#[cfg(feature = "server")]
mod read_total_balance;

/// Count the notes in the shielded pool commitment tree
#[cfg(feature = "server")]
mod notes_count;
