use crate::version::drive_versions::drive_contract_method_versions::v5::DRIVE_CONTRACT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractFeePotMethodVersions, DriveContractMethodVersions,
};
use crate::version::drive_versions::drive_document_method_versions::v5::DRIVE_DOCUMENT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_identity_method_versions::v2::DRIVE_IDENTITY_METHOD_VERSIONS_V2;
use crate::version::drive_versions::drive_identity_method_versions::{
    DriveIdentityCostEstimationMethodVersions, DriveIdentityMethodVersions,
};
use crate::version::drive_versions::drive_state_transition_method_versions::v5::DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_token_method_versions::v3::DRIVE_TOKEN_METHOD_VERSIONS_V3;
use crate::version::drive_versions::drive_verify_method_versions::v4::DRIVE_VERIFY_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_vote_method_versions::v4::DRIVE_VOTE_METHOD_VERSIONS_V4;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{
    DriveBatchOperationsMethodVersion, DriveFeesMethodVersions, DriveInitializationMethodVersions,
    DriveMethodVersions, DrivePlatformSystemMethodVersions,
    DrivePrefundedSpecializedMethodVersions, DriveSystemEstimationCostsMethodVersions,
    DriveVersion,
};
use grovedb_version::version::v5::GROVE_V5;

/// Drive version 10, introduced in protocol v15:
///
/// * **Compiled contracts**: with `DRIVE_CONTRACT_METHOD_VERSIONS_V5`, applying a contract writes
///   nothing when its format version 1 bytes match the stored contract's, whatever format the
///   stored one is in, as contracts are stored compiled from this version
///   (`contract_storage_serialization_version` 2).
/// * **Escrow trees**: `create_initial_state_structure` 5 adds the two escrow trees of the
///   token marketplace: `[Tokens, TOKEN_ORDER_ESCROW_KEY]`, the BigSumTree of the tokens open
///   listings hold, and `[PreFundedSpecializedBalances, PREFUNDED_BALANCES_FOR_TOKEN_ORDERS]`,
///   the SumTree of the credits open offers hold. The upgrade path
///   (`Platform::transition_to_version_15`) inserts them through the same helper.
/// * **Token conservation**: `DRIVE_TOKEN_METHOD_VERSIONS_V3` bumps
///   `calculate_total_tokens_balance` to 2, which counts the escrowed listings on the balance
///   side; its escrow methods, `None` before, are active.
/// * **Random token amounts**: `DRIVE_TOKEN_METHOD_VERSIONS_V3` makes
///   `settle_random_token_amounts` `Some(0)` (`None` before): the random token transfers and
///   mints of a block are paid out from the block's random beacon.
/// * **Escrow fee estimates**: the offers' escrow key sits a level below the other keys of the
///   prefunded specialized balances layer, so `estimated_cost_for_prefunded_specialized_balance_update`
///   2 and `add_estimation_costs_for_contract_fee_pot_update` 1 keep a deeper description of that
///   layer another write of the batch registered, rather than lowering it to their keys' level;
///   likewise the listings' escrow key sits deeper in the Tokens layer than the balances key, and
///   `for_token_balances` 1 keeps a deeper description of the Tokens layer. The escrows also sit
///   deeper in the root tree than the identity balances and the token balances, so
///   `for_balances` 1, `for_token_balances` 1 and the voting balance estimation 2 keep a deeper
///   description of the root as well. Beyond the estimators, `apply_drive_operations` 2 merges
///   the descriptions of every write of a batch (below), so no other write of the batch (a
///   document, a nonce) lowers what the escrows describe, whichever is built first.
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
/// * **Private document entries** (protocol v15 note 25): `DRIVE_DOCUMENT_METHOD_VERSIONS_V5`
///   sets `append_private_document_entries_operations` to `Some(0)` (`None` before): a
///   `PrivateDocumentCreateFromShieldedPool` appends its entries to the store, each behind
///   Drive's 16-byte header.
/// * **A batch's fee estimate is resolved** (protocol v15 note 14): `apply_drive_operations` 2
///   resolves an estimated batch's layer descriptions once the whole batch is lowered, the root
///   also kept at the level of the deepest root key the batch writes under, so the estimate no
///   longer depends on the order of the batch's operations. `DRIVE_DOCUMENT_METHOD_VERSIONS_V5`
///   also sets `remove_reference_for_index_level_for_contract_operations` to 2 and
///   `stateless_delete_of_non_tree_for_costs` to 1, and
///   `platform_system.estimation_costs.for_total_system_credits_update` is 1, so a delete and a
///   system credits update describe their layers as they are. Applying a batch is unchanged.
/// * **Core fork activations**: `platform_system.fetch_core_fork_activation_height` and
///   `store_core_fork_activation_height` are `Some(0)` (`None` before): Drive keeps, in `Misc`,
///   the chain locked Core height at which Platform first saw each tracked Core fork active.
/// * **Refunds at the storing epoch's rate** (protocol v15 note 27): `fees.calculate_fee` 1
///   prices the bytes a refund returns at the storage rate of the protocol version that ran the
///   epoch that stored them, which that epoch's tree records.
///
/// * **GroveDB V5** (protocol v15 note 26): `grove_version` is `GROVE_V5` (`GROVE_V4` before).
///   GroveDB reads and proves a private document store's entries by position range only under
///   V5 (`private_document_store.get_range` and `range_proof`), prices a store's creation as it
///   applies it (`merk_insert_element` 1 in both estimators), and lets a batch carry the same
///   keyless append more than once (`apply_batch.keyless_append_repeats`).
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        initialization: DriveInitializationMethodVersions {
            create_initial_state_structure: 5, // changed in v10: adds the token marketplace escrow trees
        },
        token: DRIVE_TOKEN_METHOD_VERSIONS_V3, // changed in v10: calculate_total_tokens_balance 2 counts the token marketplace escrow; order_escrow active; settle_random_token_amounts Some(0)
        contract: DriveContractMethodVersions {
            fee_pots: DriveContractFeePotMethodVersions {
                add_estimation_costs_for_contract_fee_pot_update: 1, // changed in v10: keeps a deeper prefunded balances layer description
                ..DRIVE_CONTRACT_METHOD_VERSIONS_V5.fee_pots
            },
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V5 // changed in v10: apply_contract_with_serialization 1 (compiled contract storage)
        },
        identity: DriveIdentityMethodVersions {
            cost_estimation: DriveIdentityCostEstimationMethodVersions {
                for_balances: 1,       // changed in v10: keeps a deeper root description
                for_token_balances: 1, // changed in v10: keeps a deeper root and Tokens layer description
                ..DRIVE_IDENTITY_METHOD_VERSIONS_V2.cost_estimation
            },
            ..DRIVE_IDENTITY_METHOD_VERSIONS_V2
        },
        document: DRIVE_DOCUMENT_METHOD_VERSIONS_V5, // changed in v10: private document type stores, delete estimates
        batch_operations: DriveBatchOperationsMethodVersion {
            apply_drive_operations: 2, // changed in v10: a batch's estimate is resolved (note 14)
            ..DRIVE_VERSION_V9.methods.batch_operations
        },
        prefunded_specialized_balances: DrivePrefundedSpecializedMethodVersions {
            estimated_cost_for_prefunded_specialized_balance_update: 2, // changed in v10: keeps a deeper prefunded balances layer description
            ..DRIVE_VERSION_V9.methods.prefunded_specialized_balances
        },
        vote: DRIVE_VOTE_METHOD_VERSIONS_V4, // changed in v10: the yes/no poll kind
        verify: DRIVE_VERIFY_METHOD_VERSIONS_V4, // changed in v10: the yes/no and private document entries proof verifiers
        state_transitions: DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5, // changed in v10: a consume deletes what its lookup read
        platform_system: DrivePlatformSystemMethodVersions {
            estimation_costs: DriveSystemEstimationCostsMethodVersions {
                for_total_system_credits_update: 1, // changed in v10: `Misc` described with its subtrees
            },
            fetch_core_fork_activation_height: Some(0), // changed in v10: the Core fork activations
            store_core_fork_activation_height: Some(0), // changed in v10: the Core fork activations
        },
        fees: DriveFeesMethodVersions { calculate_fee: 1 }, // changed in v10: refunds at the storing epoch's rate (note 27)
        ..DRIVE_VERSION_V9.methods
    },
    // changed in v10: GROVE_V5 (protocol v15 note 26), whose gates protocol v15 needs: a private
    // document store's entries are read and proved by position range, a store's creation is
    // estimated as it is applied, and a batch may repeat a keyless append. GROVE_V4, which v9
    // keeps, refuses all three.
    grove_version: GROVE_V5,
    ..DRIVE_VERSION_V9
};
