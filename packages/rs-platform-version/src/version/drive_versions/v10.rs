use super::drive_contract_method_versions::v5::DRIVE_CONTRACT_METHOD_VERSIONS_V5;
use super::drive_document_method_versions::v5::DRIVE_DOCUMENT_METHOD_VERSIONS_V5;
use super::drive_state_transition_method_versions::v5::DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5;
use super::drive_verify_method_versions::v3::DRIVE_VERIFY_METHOD_VERSIONS_V3;
use super::v9::DRIVE_VERSION_V9;
use super::{DriveMethodVersions, DriveProveMethodVersions, DriveVersion};

/// Drive version 10, introduced in protocol v15.
///
/// Moves keep-history documents to per-type history trees and selects the
/// matching contract writers, document operations, queries, and proof verifier.
/// It also selects the delete and erase lifecycle of keep-history documents:
/// the delete wrappers and action conversion that record and consume a
/// lifecycle entry, the batch conversion that knows action format 1, and the
/// state transition prover and verifier that know batch wire format 2.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        document: DRIVE_DOCUMENT_METHOD_VERSIONS_V5,
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V5,
        verify: DRIVE_VERIFY_METHOD_VERSIONS_V3,
        state_transitions: DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5,
        prove: DriveProveMethodVersions {
            prove_state_transition: 1,
            ..DRIVE_VERSION_V9.methods.prove
        },
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
