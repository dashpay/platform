// Protocol version queries
pub mod upgrade_state;
pub mod upgrade_vote_status;

// Re-export all public functions for convenient access
pub use upgrade_state::dash_sdk_protocol_version_get_upgrade_state;
pub use upgrade_vote_status::dash_sdk_protocol_version_get_upgrade_vote_status;
