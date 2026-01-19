//! Address query operations

pub mod info;
pub mod infos;

// Re-export main functions for convenient access
pub use info::dash_sdk_address_fetch_info;
pub use infos::dash_sdk_addresses_fetch_infos;
