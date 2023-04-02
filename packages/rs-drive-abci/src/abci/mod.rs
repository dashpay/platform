mod error;

// old code - handlers and messages
// #[deprecated = "logic moved to [server] and [proposal] mod"]
pub mod handlers;
// #[deprecated = "use tenderdash-proto crate whenever possible"]
pub mod messages;

// new code - config,
#[cfg(feature = "server")]
pub mod config;
#[cfg(feature = "server")]
mod server;
#[cfg(test)]
mod mimic;

pub use error::AbciError;
#[cfg(feature = "server")]
pub use server::start;
