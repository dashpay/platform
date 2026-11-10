use crate::version::drive_versions::drive_verify_method_versions::v3::DRIVE_VERIFY_METHOD_VERSIONS_V3;
use crate::version::drive_versions::drive_verify_method_versions::{
    DriveVerifyDocumentMethodVersions, DriveVerifyMethodVersions,
    DriveVerifySingleDocumentMethodVersions, DriveVerifyStateTransitionMethodVersions,
    DriveVerifyVoteMethodVersions,
};

/// Version 4 of the Drive verify method versions, introduced in protocol version 15 with the
/// yes/no poll kind: `voting.verify_yes_no_vote_poll_state_proof`,
/// `voting.verify_masternode_yes_no_vote` and `voting.verify_identity_yes_no_vote` are `Some(0)`
/// (`None`, not active, in every earlier table), and so is
/// `document.verify_private_document_entries`, with private document types.
/// `single_document.verify_proof_keep_serialized` is 1: a document that may be contested is
/// verified, and so is a document outside a contest at a block time in a type that keeps
/// history, where generation 0 refused every such query.
/// `state_transition.verify_state_transition_was_executed_with_proof` is 2: the execution proof
/// of a delta contract update is verified against the contract the delta was built on. Every
/// other transition is verified as generation 1 verifies it. Everything else matches
/// `DRIVE_VERIFY_METHOD_VERSIONS_V3`.
pub const DRIVE_VERIFY_METHOD_VERSIONS_V4: DriveVerifyMethodVersions = DriveVerifyMethodVersions {
    voting: DriveVerifyVoteMethodVersions {
        verify_yes_no_vote_poll_state_proof: Some(0),
        verify_masternode_yes_no_vote: Some(0),
        verify_identity_yes_no_vote: Some(0),
        ..DRIVE_VERIFY_METHOD_VERSIONS_V3.voting
    },
    document: DriveVerifyDocumentMethodVersions {
        verify_private_document_entries: Some(0), // changed: a proved page of a private type's store
        ..DRIVE_VERIFY_METHOD_VERSIONS_V3.document
    },
    state_transition: DriveVerifyStateTransitionMethodVersions {
        verify_state_transition_was_executed_with_proof: 2, // changed: verifies a delta contract update against the contract it was built on
    },
    single_document: DriveVerifySingleDocumentMethodVersions {
        verify_proof_keep_serialized: 1, // changed: verifies a document that may be contested, and one outside a contest at a block time
        ..DRIVE_VERIFY_METHOD_VERSIONS_V3.single_document
    },
    ..DRIVE_VERIFY_METHOD_VERSIONS_V3
};
