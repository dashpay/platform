mod error;

// old code - handlers and messages
// #[deprecated = "logic moved to [server] and [proposal] mod"]
pub mod handlers;
// #[deprecated = "use tenderdash-proto crate whenever possible"]
pub mod messages;

// new code - config,
#[cfg(feature = "server")]
pub mod config;
// #[cfg(test)]
/// Mimic of block execution for tests
pub mod mimic;
#[cfg(any(feature = "server", test))]
mod server;

pub mod commit;
pub mod withdrawal;

pub use error::AbciError;
#[cfg(feature = "server")]
pub use server::start;
pub use server::AbciApplication;
