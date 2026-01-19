//! Address query operations

pub mod info;
pub mod infos;
pub mod trunk_state;

// Re-export main functions for convenient access
pub use info::dash_sdk_address_fetch_info;
pub use infos::dash_sdk_addresses_fetch_infos;
pub use trunk_state::dash_sdk_address_fetch_trunk_state;
