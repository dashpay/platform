use crate::version::drive_versions::drive_verify_method_versions::v2::DRIVE_VERIFY_METHOD_VERSIONS_V2;
use crate::version::drive_versions::drive_verify_method_versions::{
    DriveVerifyCompositeDocumentMethodVersions, DriveVerifyDocumentCountMethodVersions,
    DriveVerifyDocumentSumMethodVersions, DriveVerifyMethodVersions,
    DriveVerifyStateTransitionMethodVersions,
};

/// Version 3 of the Drive verify method versions.
///
/// Changed from v2:
/// - `verify_state_transition_was_executed_with_proof` 0 → 1, with
///   `DriveProveMethodVersions::prove_state_transition` 0 → 1: the proof of an
///   owned, fee-paying transition carries the owner's credit balance (a document
///   batch's as one strictly verified merged query). The verifier reads that
///   layout from the producer's `prove_state_transition` generation, so feature
///   version 1 checks what feature version 0 checks, and either verifies both
///   layouts.
/// - The six range-total verifiers (`verify_aggregate_count_proof`,
///   `verify_carrier_aggregate_count_proof`, `verify_aggregate_sum_proof`,
///   `verify_carrier_aggregate_sum_proof`, `verify_aggregate_count_and_sum_proof`,
///   `verify_carrier_aggregate_count_and_sum_proof`) 0 → 1: a proof showing the
///   range holds nothing (an equality value no document holds, or an empty tree
///   of a kind the read does not aggregate), which grovedb's aggregate verifiers
///   refuse, verifies to a zero total or no carrier branch. The prover is
///   unchanged. The count and sum verifiers move together: a count over a
///   `summableOffCountIndex` index is verified by the sum one.
/// - `verify_composite_documents_proof` 0 → 1: the sum-bearing items of a
///   `documentsSummable` type (and a `summable` indexOnly index) are read as
///   documents, where v0 reads them as counts and refuses the proof.
pub const DRIVE_VERIFY_METHOD_VERSIONS_V3: DriveVerifyMethodVersions = DriveVerifyMethodVersions {
    state_transition: DriveVerifyStateTransitionMethodVersions {
        verify_state_transition_was_executed_with_proof: 1,
    },
    composite_document: DriveVerifyCompositeDocumentMethodVersions {
        verify_composite_documents_proof: 1,
    },
    document_count: DriveVerifyDocumentCountMethodVersions {
        verify_aggregate_count_proof: 1,
        verify_carrier_aggregate_count_proof: 1,
        verify_distinct_count_proof: 0,
        verify_point_lookup_count_proof: 0,
        verify_primary_key_count_tree_proof: 0,
    },
    document_sum: DriveVerifyDocumentSumMethodVersions {
        verify_aggregate_sum_proof: 1,
        verify_carrier_aggregate_sum_proof: 1,
        verify_carrier_aggregate_count_and_sum_proof: 1,
        verify_aggregate_count_and_sum_proof: 1,
        verify_primary_key_sum_tree_proof: 0,
        verify_primary_key_count_sum_tree_proof: 0,
        verify_point_lookup_sum_proof: 0,
        verify_distinct_sum_proof: 0,
        verify_distinct_count_and_sum_proof: 0,
        verify_point_lookup_count_and_sum_proof: 0,
    },
    ..DRIVE_VERIFY_METHOD_VERSIONS_V2
};
