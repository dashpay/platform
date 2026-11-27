/// Dash Core RPC
pub mod core;

/// A Core JSON-RPC transport with a deadline for the whole request
pub(crate) mod deadline_http;
/// Speculative Core RPC fetching for consecutive core heights
pub(crate) mod prefetch;
/// Core signatures verification
pub mod signature;
