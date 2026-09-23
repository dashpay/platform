use crate::version::drive_versions::drive_contract_method_versions::v5::DRIVE_CONTRACT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_group_method_versions::v2::DRIVE_GROUP_METHOD_VERSIONS_V2;
use crate::version::drive_versions::drive_identity_method_versions::v3::DRIVE_IDENTITY_METHOD_VERSIONS_V3;
use crate::version::drive_versions::drive_state_transition_method_versions::v5::DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_verify_method_versions::v4::DRIVE_VERIFY_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_vote_method_versions::v4::DRIVE_VOTE_METHOD_VERSIONS_V4;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{
    DriveBatchOperationsMethodVersion, DriveFeesMethodVersions, DriveMethodVersions, DriveVersion,
};

/// Drive version 10, introduced in protocol v15. Differs from v9 in:
///
/// * **Contracts stored compiled**: `DRIVE_CONTRACT_METHOD_VERSIONS_V5` selects
///   `apply_contract_with_serialization` 1, which writes nothing when a contract's format
///   version 1 bytes match the stored contract's, whatever format the stored one is in,
///   as contracts are stored compiled from this version
///   (`contract_storage_serialization_version` 2).
/// * **Yes/no vote polls**: `DRIVE_VOTE_METHOD_VERSIONS_V4` bumps
///   `insert.register_identity_vote` to 1, which registers a vote of either poll kind, and
///   `cleanup.remove_all_votes_given_by_identities` to 1, which also removes the yes/no votes of
///   a masternode that left the list. It sets every `yes_no` method to `Some(0)`, so yes/no
///   polls can be opened and voted on (`None`, not active, in every earlier table), and bumps
///   `contested_resource_insert.add_vote_poll_end_date_query_operations` to 1, which adds only
///   the entry when the batch already queues the end date's tree with other storage flags.
///   `cleanup.remove_vote_poll_end_date_query_operations`, which removes finished polls from
///   the end date index, is `Some(0)` (`None` before). `DRIVE_VERIFY_METHOD_VERSIONS_V4` makes
///   the yes/no proof verifiers `Some(0)` (`None` before).
/// * **A consume deletes what its lookup read**: `DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5`
///   sets `convert_to_high_level_operations.document_create_transition` to 1: a create deletes
///   each document it consumes from what document reference validation 1 read of it, without
///   reading it again.
/// * **Fee history required for refunds** — `fees.calculate_fee` 0 -> 1.
///   The v1 generation consults the block's fee history on every fee
///   version number and returns an internal error for an owner-attributed
///   (sectioned) storage removal without that history, where v0 priced
///   fee version number 1 against an empty map. Every shipped schedule
///   shares number 1 and the same storage rates, so the credits are
///   unchanged; only a missing history is now an error instead of a
///   silent fallback to the first-generation rates.
/// * **Recorded-owner refund credits** — `DRIVE_IDENTITY_METHOD_VERSIONS_V3`
///   turns on `update.credit_storage_refunds_to_owners`, the primitive
///   block lifecycle paths use to credit each recorded owner of a refund
///   and report the amount whose owner has no balance element.
/// * **Price before commit** — `batch_operations.apply_drive_operations`
///   1 -> 2, `DRIVE_GROUP_METHOD_VERSIONS_V2` (`insert.add_group_action`
///   0 -> 1) and `DRIVE_CONTRACT_METHOD_VERSIONS_V5` (the four moderation
///   writers that can free flagged bytes, 0 -> 1). Every fee-returning
///   entry point that owns its transaction when the caller passes none
///   now prices the batch before committing, so the missing-history error
///   above never leaves a write persisted without its fee result. With a
///   caller transaction nothing changes.
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V5, // changed in v10: apply_contract_with_serialization 1 writes nothing for a contract whose format version 1 bytes match the stored one's, as contracts are stored compiled; the moderation removal wrappers price before committing an owned transaction
        vote: DRIVE_VOTE_METHOD_VERSIONS_V4,         // changed in v10: the yes/no poll kind
        fees: DriveFeesMethodVersions { calculate_fee: 1 }, // changed in v10: fee history required and consulted for every storage refund
        verify: DRIVE_VERIFY_METHOD_VERSIONS_V4, // changed in v10: the yes/no proof verifiers
        identity: DRIVE_IDENTITY_METHOD_VERSIONS_V3, // changed in v10: credit_storage_refunds_to_owners primitive for lifecycle refund settlement
        state_transitions: DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5, // changed in v10: a consume deletes what its lookup read
        batch_operations: DriveBatchOperationsMethodVersion {
            apply_drive_operations: 2, // changed in v10: the batch is priced before an owned transaction commits
            ..DRIVE_VERSION_V9.methods.batch_operations
        },
        group: DRIVE_GROUP_METHOD_VERSIONS_V2, // changed in v10: add_group_action prices before committing an owned transaction
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
