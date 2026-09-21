/// Functions related to withdrawal documents
pub mod document;
#[cfg(all(feature = "server", any(test, feature = "structure")))]
pub(crate) mod structure;

/// The withdrawal limit; from protocol version 14 it mirrors Core's credit pool rule
pub mod calculate_current_withdrawal_limit;
/// Functions and constants related to GroveDB paths
pub mod paths;
/// Functions related to withdrawal transactions
pub mod transaction;
