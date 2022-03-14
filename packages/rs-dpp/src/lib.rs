pub mod identifier;
pub mod identity;
pub mod metadata;
pub mod util;

pub mod errors;

pub mod schema;
pub mod validation;

mod dash_platform_protocol;

pub use dash_platform_protocol::DashPlatformProtocol;

#[cfg(test)]
mod tests;
