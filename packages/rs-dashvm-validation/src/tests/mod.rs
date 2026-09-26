//! Shared fixtures for the crate's tests: WAT builders and the profile of the latest protocol
//! version.

use crate::profile::PreparationProfile;
use platform_version::version::dashvm_versions::DashVmLimits;
use platform_version::version::PlatformVersion;

pub mod admission_tests;
pub mod bundle_tests;
pub mod instrumentation_tests;

/// The profile of the latest protocol version, which carries the DashVM table.
pub fn latest_profile() -> PreparationProfile {
    PreparationProfile::try_from(PlatformVersion::latest())
        .expect("the latest protocol version carries the DashVM table")
}

/// The latest profile with one limit replaced, for cap tests.
pub fn profile_with(edit: impl FnOnce(&mut DashVmLimits)) -> PreparationProfile {
    let mut profile = latest_profile();
    edit(&mut profile.limits);
    profile
}

/// Assembles WAT into canonical bytes.
pub fn wasm(wat: &str) -> Vec<u8> {
    wat::parse_str(wat).expect("fixture WAT assembles")
}

/// The smallest module admission accepts: a memory export, `dash_alloc` and one entry.
pub const MINIMAL: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "dash_alloc") (param i32) (result i32) i32.const 0)
  (func (export "run") (param i32 i32) (result i64) i64.const 0)
)"#;

/// Wraps a body fragment into a module that also has the required exports.
pub fn module_with(body: &str) -> String {
    format!(
        r#"(module
  {body}
  (memory (export "memory") 1)
  (func (export "dash_alloc") (param i32) (result i32) i32.const 0)
  (func (export "run") (param i32 i32) (result i64) i64.const 0)
)"#
    )
}
