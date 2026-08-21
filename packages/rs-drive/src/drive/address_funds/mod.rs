#[cfg(feature = "server")]
mod add_balance_to_address;
/// State-aware fee estimation for address funding from an asset lock.
#[cfg(feature = "server")]
pub mod estimate_funding_fee;
/// Cost estimation for address balance operations.
#[cfg(feature = "server")]
mod estimated_costs;
/// Functionality for fetching data from GroveDB.
#[cfg(feature = "server")]
pub mod fetch;

/// Tools for generating and verifying cryptographic proofs.
#[cfg(feature = "server")]
pub mod prove;

/// Query-building and execution utilities for GroveDB.
pub mod queries;
#[cfg(feature = "server")]
mod remove_balance_from_address;
#[cfg(feature = "server")]
mod set_balance_to_address;
