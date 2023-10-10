mod error;

/// The handlers of abci messages
#[cfg(any(feature = "server", test))]
pub mod handler;

// server configuration
pub mod config;
#[cfg(any(feature = "server", test))]
pub(crate) mod server;

pub use error::AbciError;
#[cfg(feature = "server")]
pub use server::start;
#[cfg(any(feature = "server", test))]
pub use server::AbciApplication;
