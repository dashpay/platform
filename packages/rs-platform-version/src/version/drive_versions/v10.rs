use super::drive_contract_method_versions::v5::DRIVE_CONTRACT_METHOD_VERSIONS_V5;
use super::drive_document_method_versions::v5::DRIVE_DOCUMENT_METHOD_VERSIONS_V5;
use super::drive_verify_method_versions::v3::DRIVE_VERIFY_METHOD_VERSIONS_V3;
use super::v9::DRIVE_VERSION_V9;
use super::{DriveMethodVersions, DriveVersion};

/// Drive version 10, introduced in protocol v15.
///
/// Moves keep-history documents to per-type history trees and selects the
/// matching contract writers, document operations, queries, and proof verifier.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        document: DRIVE_DOCUMENT_METHOD_VERSIONS_V5,
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V5,
        verify: DRIVE_VERIFY_METHOD_VERSIONS_V3,
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
