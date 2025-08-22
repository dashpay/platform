//! Epochs Mod File.
//!

#[cfg(feature = "server")]
pub mod credit_distribution_pools;
#[cfg(feature = "server")]
mod get_epochs_infos;
#[cfg(feature = "server")]
mod get_epochs_protocol_versions;

/// Epoch key constants module
pub mod epoch_key_constants;
/// Epochs root tree key constants module
pub mod epochs_root_tree_key_constants;
#[cfg(feature = "server")]
pub mod operations_factory;
/// Paths module
pub mod paths;
#[cfg(feature = "server")]
pub mod proposers;
#[cfg(feature = "server")]
mod prove_epochs_infos;
#[cfg(feature = "server")]
pub mod start_block;
#[cfg(feature = "server")]
pub mod start_time;

#[cfg(feature = "server")]
mod get_epoch_protocol_version;
#[cfg(feature = "server")]
mod has_epoch_tree_exists;

#[cfg(feature = "server")]
mod get_finalized_epoch_info;
#[cfg(feature = "server")]
mod prove_finalized_epoch_infos;

/// Queries for epochs
pub mod queries;
