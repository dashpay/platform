//! Document verification module
//!
//! This module provides functions for verifying document-related proofs including:
//! - Document query verification with various return formats
//! - Single document verification
//! - Start-at document verification for pagination
//!
//! Document IDs and contract IDs are returned as base58-encoded strings.

pub mod verify_proof;
pub mod verify_proof_keep_serialized;
pub mod verify_start_at_document_in_proof;

pub use verify_proof::*;
pub use verify_proof_keep_serialized::*;
pub use verify_start_at_document_in_proof::*;
