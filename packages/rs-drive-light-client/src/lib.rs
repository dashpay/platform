/// Bindings generation using [uniffi]
#[cfg(feature = "uniffi")]
pub mod uniffi_bindings;

/// Error definitions, compatible with bindings
mod error;
/// Implementation of proof verification
pub mod proof;

pub use error::Error;

uniffi::include_scaffolding!("dash_drive_v0");
