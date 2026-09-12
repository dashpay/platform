use crate::version::drive_versions::drive_contract_method_versions::v5::DRIVE_CONTRACT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_document_method_versions::v5::DRIVE_DOCUMENT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_identity_method_versions::v3::DRIVE_IDENTITY_METHOD_VERSIONS_V3;
use crate::version::drive_versions::drive_state_transition_method_versions::v5::DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_verify_method_versions::v4::DRIVE_VERIFY_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_vote_method_versions::v4::DRIVE_VOTE_METHOD_VERSIONS_V4;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{
    DriveBatchOperationsMethodVersion, DriveFeesMethodVersions, DriveMethodVersions,
    DrivePlatformSystemMethodVersions, DriveSystemEstimationCostsMethodVersions, DriveVersion,
};

/// Drive version 10.
/// Introduced in protocol v15:
///
/// * **Compiled contract storage**: `DRIVE_CONTRACT_METHOD_VERSIONS_V5`: applying a contract
///   writes nothing when its format version 1 bytes match the stored contract's, whatever
///   format the stored one is in, as contracts are stored compiled from this version
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
/// * **Private document types**: `DRIVE_DOCUMENT_METHOD_VERSIONS_V5` sets
///   `insert_empty_private_document_store_operation` to `Some(0)` (`None` before): a contract
///   registering a private document type, or an update adding one, creates the type's private
///   document store at its key `[0]`; `fetch_private_document_entries` and
///   `prove_private_document_entries` there, and `verify_private_document_entries` in
///   `DRIVE_VERIFY_METHOD_VERSIONS_V4`, are `Some(0)` (`None` before): a page of the store
///   is read, proved and verified.
/// * **A batch's fee estimate is resolved** (protocol v15 note 14): `apply_drive_operations` 2
///   resolves an estimated batch's layer descriptions once the whole batch is lowered, the root
///   also kept at the level of the deepest root key the batch writes under, so the estimate no
///   longer depends on the order of the batch's operations. `DRIVE_DOCUMENT_METHOD_VERSIONS_V5`
///   also sets `remove_reference_for_index_level_for_contract_operations` to 2 and
///   `stateless_delete_of_non_tree_for_costs` to 1, and
///   `platform_system.estimation_costs.for_total_system_credits_update` is 1, so a delete and a
///   system credits update describe their layers as they are. Applying a batch is unchanged.
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
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V5, // changed in v10: apply_contract_with_serialization 1 writes nothing for a contract whose format version 1 bytes match the stored one's, as contracts are stored compiled
        document: DRIVE_DOCUMENT_METHOD_VERSIONS_V5, // changed in v10: private document type stores, delete estimates
        vote: DRIVE_VOTE_METHOD_VERSIONS_V4,         // changed in v10: the yes/no poll kind
        verify: DRIVE_VERIFY_METHOD_VERSIONS_V4, // changed in v10: the yes/no and private document entries proof verifiers
        state_transitions: DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5, // changed in v10: a consume deletes what its lookup read
        batch_operations: DriveBatchOperationsMethodVersion {
            apply_drive_operations: 2, // changed in v10: a batch's estimate is resolved (note 14)
            ..DRIVE_VERSION_V9.methods.batch_operations
        },
        platform_system: DrivePlatformSystemMethodVersions {
            estimation_costs: DriveSystemEstimationCostsMethodVersions {
                for_total_system_credits_update: 1, // changed in v10: `Misc` described with its subtrees
            },
        },
        fees: DriveFeesMethodVersions { calculate_fee: 1 }, // changed in v10: fee history required and consulted for every storage refund
        identity: DRIVE_IDENTITY_METHOD_VERSIONS_V3, // changed in v10: credit_storage_refunds_to_owners primitive for lifecycle refund settlement
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
