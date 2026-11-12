use crate::version::drive_versions::drive_verify_method_versions::v3::DRIVE_VERIFY_METHOD_VERSIONS_V3;
use crate::version::drive_versions::drive_verify_method_versions::{
    DriveVerifyDocumentMethodVersions, DriveVerifyMethodVersions,
    DriveVerifySingleDocumentMethodVersions, DriveVerifyVoteMethodVersions,
};

/// Version 4 of the Drive verify method versions, introduced in protocol version 15 with the
/// yes/no poll kind: `voting.verify_yes_no_vote_poll_state_proof`,
/// `voting.verify_masternode_yes_no_vote` and `voting.verify_identity_yes_no_vote` are `Some(0)`
/// (`None`, not active, in every earlier table), and so is
/// `document.verify_private_document_entries`, with private document types.
/// `single_document.verify_proof_keep_serialized` is 1: a document that may be contested is
/// verified, and so is a document outside a contest at a block time in a type that keeps
/// history, including time-and-revision keys in the composite layout, where generation 0
/// refused every timed query. Everything else matches
/// `DRIVE_VERIFY_METHOD_VERSIONS_V3`, except the retained-history and cursor verifiers
/// selecting generation 1 for the per-type revision layout.
pub const DRIVE_VERIFY_METHOD_VERSIONS_V4: DriveVerifyMethodVersions = DriveVerifyMethodVersions {
    voting: DriveVerifyVoteMethodVersions {
        verify_yes_no_vote_poll_state_proof: Some(0),
        verify_masternode_yes_no_vote: Some(0),
        verify_identity_yes_no_vote: Some(0),
        ..DRIVE_VERIFY_METHOD_VERSIONS_V3.voting
    },
    document: DriveVerifyDocumentMethodVersions {
        verify_document_history: 1,
        verify_start_at_document_in_proof: 1,
        verify_private_document_entries: Some(0), // changed: a proved page of a private type's store
        ..DRIVE_VERIFY_METHOD_VERSIONS_V3.document
    },
    single_document: DriveVerifySingleDocumentMethodVersions {
        verify_proof: 1, // validates the returned document against its authenticated history key
        verify_proof_keep_serialized: 1, // changed: verifies maybe-contested documents and time-bounded retained revisions outside a contest
    },
    ..DRIVE_VERIFY_METHOD_VERSIONS_V3
};
