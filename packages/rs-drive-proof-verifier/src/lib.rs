/// Bindings generation using [uniffi]
#[cfg(feature = "uniffi")]
pub mod uniffi_bindings;

/// Error definitions, compatible with bindings
mod error;
/// Implementation of proof verification
pub mod proof;

pub use error::Error;
pub use proof::from_proof::FromProof;

#[cfg(feature = "uniffi")]
uniffi::include_scaffolding!("dash_drive_v0");
