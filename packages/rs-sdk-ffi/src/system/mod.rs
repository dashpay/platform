//! System queries module

pub mod queries;
pub mod status;

// Re-export status function
pub use status::dash_sdk_get_status;