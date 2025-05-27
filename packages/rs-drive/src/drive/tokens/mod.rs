/// Handles operations related to adding transaction history.
#[cfg(feature = "server")]
mod add_transaction_history_operations;

/// Defines logic for applying status updates within the system.
#[cfg(feature = "server")]
pub mod apply_status;

/// Manages operations related to balance handling.
pub mod balance;

/// Implements functionality for burning tokens.
#[cfg(feature = "server")]
pub mod burn;

/// Computes estimated costs for various operations.
#[cfg(feature = "server")]
pub mod estimated_costs;

/// Manages freezing operations in the system.
#[cfg(feature = "server")]
pub mod freeze;

/// Identity token info module, like if someone is frozen
pub mod info;

/// Implements minting operations for creating new tokens.
#[cfg(feature = "server")]
pub mod mint;

/// Implements minting operations for creating new tokens towards many recipients at the same time.
#[cfg(feature = "server")]
pub mod mint_many;

/// Manages system-level operations and utilities.
#[cfg(feature = "server")]
pub mod system;

/// Handles transfer operations, including token movement.
#[cfg(feature = "server")]
pub mod transfer;

/// Manages unfreezing operations within the system.
#[cfg(feature = "server")]
pub mod unfreeze;

/// Calculates the total token balance across all accounts.
#[cfg(feature = "server")]
pub mod calculate_total_tokens_balance;

mod contract_info;
mod direct_purchase;
/// Distribution module
pub mod distribution;
/// Token paths
pub mod paths;
/// Token status module, like if the token is paused
pub mod status;
