//! Deterministic admission, structural bounding and logical-stack instrumentation of DashVM
//! contract code.
//!
//! This crate turns the canonical WebAssembly an author submits into the prepared WebAssembly
//! the engine compiles, as a pure function of the bytes and the protocol version's DashVM
//! table. It has no engine, no storage and no state: the node and the developer tooling share
//! it so that "will this bundle be admitted" has one answer, and the authority and capability
//! checks that need state live above it.
//!
//! The entry point is [`validate_and_prepare_bundle`]. Per module it applies the byte cap, the
//! admitted feature allowlist, the structural caps, the memory and table shape rules, the
//! import allowlist and the export rules, rewrites the module with the portable logical-stack
//! accounting, re-validates the output and checks its provenance, and records the canonical and
//! prepared hashes separately. Per bundle it validates names, resolves every internal binding
//! against the target's exports, requires the dependency graph to be acyclic, checks the
//! entries and computes the bundle digest.
//!
//! Every limit comes from [`PreparationProfile`], the projection of
//! `PlatformVersion::dashvm`; the crate defines no number of its own.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod abi_names;
mod abi_validation;
pub mod admission;
pub mod bundle;
pub mod errors;
pub mod hashing;
pub mod instrumentation;
pub mod prepared_module;
pub mod profile;
pub mod stack;
mod structure;
pub mod wasm_features;

mod bundle_preparation;
#[cfg(test)]
mod tests;

pub use bundle_preparation::{validate_and_prepare_bundle, BundleInput, DeclaredBinding};
pub use errors::{BundleError, ModuleError, PreparationError, ProfileError};
pub use profile::PreparationProfile;
