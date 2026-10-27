use super::drive_contract_method_versions::v5::DRIVE_CONTRACT_METHOD_VERSIONS_V5;
use super::drive_document_method_versions::v5::DRIVE_DOCUMENT_METHOD_VERSIONS_V5;
use super::drive_verify_method_versions::v4::DRIVE_VERIFY_METHOD_VERSIONS_V4;
use super::v9::DRIVE_VERSION_V9;
use super::{DriveMethodVersions, DriveVersion};
use grovedb_version::version::v5::GROVE_V5;

/// Drive version 10, introduced in protocol v15.
///
/// Moves keep-history documents to per-type history trees and selects the
/// matching contract writers, document operations, queries, and proof verifier.
/// Grove V5 lets a history page share one proof with its lifecycle and count.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        document: DRIVE_DOCUMENT_METHOD_VERSIONS_V5,
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V5,
        verify: DRIVE_VERIFY_METHOD_VERSIONS_V4,
        ..DRIVE_VERSION_V9.methods
    },
    grove_version: GROVE_V5,
    ..DRIVE_VERSION_V9
};
