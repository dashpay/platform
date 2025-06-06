//! Integration tests for rs-sdk-ffi
//!
//! These tests use the same test vectors as rs-sdk to ensure compatibility

#[path = "integration_tests/config.rs"]
mod config;
#[path = "integration_tests/ffi_utils.rs"]
mod ffi_utils;

// Test modules
#[path = "integration_tests/data_contract.rs"]
mod data_contract;
#[path = "integration_tests/document.rs"]
mod document;
#[path = "integration_tests/identity.rs"]
mod identity;
#[path = "integration_tests/contested_resource.rs"]
mod contested_resource;
#[path = "integration_tests/token.rs"]
mod token;
#[path = "integration_tests/system.rs"]
mod system;
#[path = "integration_tests/protocol_version.rs"]
mod protocol_version;
#[path = "integration_tests/evonode.rs"]
mod evonode;
#[path = "integration_tests/voting.rs"]
mod voting;