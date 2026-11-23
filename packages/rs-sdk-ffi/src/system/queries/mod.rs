// System-level queries
pub mod blocks;
pub mod current_epoch;
pub mod current_quorums_info;
pub mod epochs_info;
pub mod path_elements;
pub mod platform_status;
pub mod prefunded_specialized_balance;
pub mod total_credits_in_platform;
pub mod transition_status;

// Re-export all public functions for convenient access
pub use blocks::{dash_sdk_system_get_blocks, dash_sdk_system_get_owner_transactions_in_block};
pub use current_epoch::dash_sdk_system_get_current_epoch;
pub use current_quorums_info::dash_sdk_system_get_current_quorums_info;
pub use epochs_info::dash_sdk_system_get_epochs_info;
pub use path_elements::dash_sdk_system_get_path_elements;
pub use platform_status::dash_sdk_get_platform_status;
pub use prefunded_specialized_balance::dash_sdk_system_get_prefunded_specialized_balance;
pub use total_credits_in_platform::dash_sdk_system_get_total_credits_in_platform;
pub use transition_status::{
    dash_sdk_system_get_recent_state_transition_status,
    dash_sdk_system_get_state_transition_status_by_hash,
};
