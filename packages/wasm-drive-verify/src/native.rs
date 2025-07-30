//! Native Rust API for proof verification
//!
//! This module provides Rust-native functions for proof verification,
//! allowing other Rust/WASM projects to use wasm-drive-verify as a library.

use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::DriveDocumentQuery;

/// Verify a full identity by identity ID
pub fn verify_full_identity_by_identity_id(
    proof: &[u8],
    is_proof_subset: bool,
    identity_id: [u8; 32],
    platform_version: &PlatformVersion,
) -> Result<([u8; 32], Option<Identity>), drive::error::Error> {
    Drive::verify_full_identity_by_identity_id(
        proof,
        is_proof_subset,
        identity_id,
        platform_version,
    )
}

/// Verify a data contract by contract ID
pub fn verify_contract(
    proof: &[u8],
    contract_known_keeps_history: Option<bool>,
    is_proof_subset: bool,
    in_multiple_contract_proof_form: bool,
    contract_id: [u8; 32],
    platform_version: &PlatformVersion,
) -> Result<([u8; 32], Option<DataContract>), drive::error::Error> {
    Drive::verify_contract(
        proof,
        contract_known_keeps_history,
        is_proof_subset,
        in_multiple_contract_proof_form,
        contract_id,
        platform_version,
    )
}

/// Verify documents using a query
pub fn verify_documents_with_query(
    proof: &[u8],
    query: &DriveDocumentQuery,
    platform_version: &PlatformVersion,
) -> Result<([u8; 32], Vec<Document>), drive::error::Error> {
    query.verify_proof(proof, platform_version)
}
