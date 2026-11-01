use crate::version::drive_versions::drive_contract_method_versions::v5::DRIVE_CONTRACT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_document_method_versions::v5::DRIVE_DOCUMENT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_state_transition_method_versions::v5::DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_verify_method_versions::v4::DRIVE_VERIFY_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_vote_method_versions::v4::DRIVE_VOTE_METHOD_VERSIONS_V4;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{
    DriveBatchOperationsMethodVersion, DriveMethodVersions, DriveVersion,
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
///   document store at its key `[0]`.
/// * **Batch estimates keep every layer description**: `batch_operations.apply_drive_operations` 2
///   lowers an estimated batch into an estimate that keeps every description any write gives a
///   layer, the nested estimators of one operation included, and once the whole batch is lowered
///   describes each layer by the costliest of them (the deepest depth, with the nodes GroveDB
///   prices dearest there, among those describing subtrees when any write does, as a tree
///   operation there needs the subtree sizes), so a batch's estimate no longer depends on the
///   order of its operations and no write is priced at a shallower depth than its own
///   description. It raises the estimate of any
///   batch whose writes describe a shared layer differently and whose last description was not
///   the costliest, an ordinary document transition and a token mint included. Applying a batch is
///   unchanged.
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V5, // changed in v10: compiled contract storage
        document: DRIVE_DOCUMENT_METHOD_VERSIONS_V5, // changed in v10: private document type stores
        vote: DRIVE_VOTE_METHOD_VERSIONS_V4,         // changed in v10: the yes/no poll kind
        verify: DRIVE_VERIFY_METHOD_VERSIONS_V4,     // changed in v10: the yes/no proof verifiers
        state_transitions: DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5, // changed in v10: a consume deletes what its lookup read
        batch_operations: DriveBatchOperationsMethodVersion {
            apply_drive_operations: 2, // changed in v10: an estimate describes each layer by the costliest description any write of the batch gives it
            ..DRIVE_VERSION_V9.methods.batch_operations
        },
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
