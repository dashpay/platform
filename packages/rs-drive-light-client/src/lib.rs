#[cfg(feature = "bindgen")]
pub mod bindgen;
pub mod bindings;
mod error;
pub mod proof;

pub use error::Error;

uniffi::include_scaffolding!("dash_drive_v0");
