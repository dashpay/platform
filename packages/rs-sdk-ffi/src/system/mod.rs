//! System queries module

mod queries;
mod status;

// Re-export all query functions
pub use queries::*;
// Re-export status function
pub use status::dash_sdk_get_status;
