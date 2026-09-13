//! Native Rust API for proof verification
//!
//! This module provides Rust-native functions for proof verification,
//! allowing other Rust/WASM projects to use wasm-drive-verify as a library.

use crate::utils::proof::validate_current_grovedb_proof;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::error::proof::ProofError;
use drive::query::DriveDocumentQuery;

fn current_grovedb_proof(proof: &[u8]) -> Result<&[u8], drive::error::Error> {
    validate_current_grovedb_proof(proof)
        .map_err(|error| drive::error::Error::Proof(ProofError::CorruptedProof(error)))?;
    Ok(proof)
}

/// Verify a full identity by identity ID
pub fn verify_full_identity_by_identity_id(
    proof: &[u8],
    is_proof_subset: bool,
    identity_id: [u8; 32],
    platform_version: &PlatformVersion,
) -> Result<([u8; 32], Option<Identity>), drive::error::Error> {
    Drive::verify_full_identity_by_identity_id(
        current_grovedb_proof(proof)?,
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
        current_grovedb_proof(proof)?,
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
    query.verify_proof(current_grovedb_proof(proof)?, platform_version)
}

#[cfg(test)]
mod tests {
    use super::{
        verify_contract, verify_documents_with_query, verify_full_identity_by_identity_id,
    };
    use dpp::data_contract::DataContract;
    use dpp::data_contracts::SystemDataContract;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;
    use drive::error::proof::ProofError;
    use drive::error::Error;
    use drive::query::DriveDocumentQuery;

    /// A bincode-encoded GroveDB proof envelope discriminant with no payload.
    fn envelope_only_proof(version: u32) -> Vec<u8> {
        bincode::encode_to_vec(version, bincode::config::standard().with_big_endian())
            .expect("encode envelope version")
    }

    fn assert_rejected_envelope(result: Result<(), Error>, version: u32) {
        match result {
            Err(Error::Proof(ProofError::CorruptedProof(message))) => assert!(
                message.contains(&format!(
                    "unsupported GroveDB proof envelope version {version}"
                )),
                "unexpected message: {message}"
            ),
            other => panic!("expected envelope rejection, got {other:?}"),
        }
    }

    fn dpns_domain_query<'a>(
        contract: &'a DataContract,
        platform_version: &PlatformVersion,
    ) -> DriveDocumentQuery<'a> {
        DriveDocumentQuery::from_sql_expr(
            "select * from domain limit 1",
            contract,
            None,
            platform_version,
        )
        .expect("DPNS domain query")
    }

    #[test]
    fn identity_entry_point_rejects_legacy_and_unknown_envelopes() {
        let platform_version = PlatformVersion::latest();

        for version in [0u32, 2u32] {
            let result = verify_full_identity_by_identity_id(
                &envelope_only_proof(version),
                false,
                [0u8; 32],
                platform_version,
            )
            .map(|_| ());
            assert_rejected_envelope(result, version);
        }
    }

    #[test]
    fn contract_entry_point_rejects_legacy_and_unknown_envelopes() {
        let platform_version = PlatformVersion::latest();

        for version in [0u32, 2u32] {
            let result = verify_contract(
                &envelope_only_proof(version),
                None,
                false,
                false,
                [0u8; 32],
                platform_version,
            )
            .map(|_| ());
            assert_rejected_envelope(result, version);
        }
    }

    #[test]
    fn documents_entry_point_rejects_legacy_and_unknown_envelopes() {
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("DPNS contract");
        let query = dpns_domain_query(&contract, platform_version);

        for version in [0u32, 2u32] {
            let result = verify_documents_with_query(
                &envelope_only_proof(version),
                &query,
                platform_version,
            )
            .map(|_| ());
            assert_rejected_envelope(result, version);
        }
    }

    #[test]
    fn entry_points_pass_v1_envelopes_through_to_drive() {
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("DPNS contract");
        let query = dpns_domain_query(&contract, platform_version);
        let truncated_v1 = envelope_only_proof(1);

        // A V1 discriminant with no payload clears the envelope gate and is
        // then rejected by Drive's own decoder, never by the envelope policy.
        let results: Vec<Result<(), Error>> = vec![
            verify_full_identity_by_identity_id(&truncated_v1, false, [0u8; 32], platform_version)
                .map(|_| ()),
            verify_contract(
                &truncated_v1,
                None,
                false,
                false,
                [0u8; 32],
                platform_version,
            )
            .map(|_| ()),
            verify_documents_with_query(&truncated_v1, &query, platform_version).map(|_| ()),
        ];

        for result in results {
            let error = result.expect_err("truncated V1 payload cannot verify");
            assert!(
                !error.to_string().contains("GroveDB proof envelope"),
                "V1 envelope must not be rejected by the envelope policy: {error}"
            );
        }
    }
}
