// System-level queries
pub mod current_quorums_info;
pub mod epochs_info;
pub mod path_elements;
pub mod prefunded_specialized_balance;
pub mod total_credits_in_platform;

// Re-export all public functions for convenient access
pub use current_quorums_info::dash_sdk_system_get_current_quorums_info;
pub use epochs_info::dash_sdk_system_get_epochs_info;
pub use path_elements::dash_sdk_system_get_path_elements;
pub use prefunded_specialized_balance::dash_sdk_system_get_prefunded_specialized_balance;
pub use total_credits_in_platform::dash_sdk_system_get_total_credits_in_platform;
