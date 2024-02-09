mod error;

// server configuration
pub mod config;

#[cfg(any(feature = "server", test))]
/// ABCI applications
pub mod app;
#[cfg(any(feature = "server", test))]
mod handler;
#[cfg(any(feature = "server", test))]
pub(crate) mod server;

pub use error::AbciError;
#[cfg(feature = "server")]
pub use server::start;
