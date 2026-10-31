use crate::version::drive_versions::drive_contract_method_versions::v5::DRIVE_CONTRACT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractFeePotMethodVersions, DriveContractMethodVersions,
};
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
    DriveBatchOperationsMethodVersion, DriveInitializationMethodVersions, DriveMethodVersions,
    DrivePrefundedSpecializedMethodVersions, DriveVersion,
};

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
/// * **Batch estimates keep every layer description**: `batch_operations.apply_drive_operations` 2
///   lowers an estimated batch into an estimate that keeps every description any write gives a
///   layer, the nested estimators of one operation included, and once the whole batch is lowered
///   describes each layer by the costliest of them (the deepest depth, with the nodes GroveDB
///   prices dearest there, among those describing subtrees when any write does, as a tree
///   operation there needs the subtree sizes), so a batch's estimate no longer depends on the
///   order of its operations and no write is priced at a shallower depth than its own
///   description. It raises the estimate of any
///   batch whose writes describe a shared layer differently and whose last description was not
///   the costliest, an ordinary document transition and a token mint included. Picking the
///   description does not change how a batch is applied.
/// * **Frozen listings**: `DRIVE_TOKEN_METHOD_VERSIONS_V3` reads a frozen identity's open
///   listings (`order_escrow.fetch_frozen_listings`) and `DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5`
///   has `TokenDestroyFrozenFunds` destroy what they hold with its balance.
/// * **Document operations built against their batch**: `apply_drive_operations` 2 also builds
///   every document operation of a batch against the batch's operations built before it, so a
///   deletion sees the batch's earlier writes as pending: two deletions that empty an index
///   tree together remove it and refund it, and an insert into a tree an earlier deletion of
///   the batch empties keeps the tree. Other operation kinds are built as before.
///   `convert_drive_operations_to_grove_operations` 2 builds its batch the same way, and, as
///   `apply_drive_operations` does, refuses a batch writing one token key twice and merges
///   repeated balance writes.
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        initialization: DriveInitializationMethodVersions {
            create_initial_state_structure: 5, // changed in v10: adds the token marketplace escrow trees
        },
        token: DRIVE_TOKEN_METHOD_VERSIONS_V3, // changed in v10: calculate_total_tokens_balance 2 counts the token marketplace escrow; order_escrow active; a frozen identity's listings are read
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
        batch_operations: DriveBatchOperationsMethodVersion {
            convert_drive_operations_to_grove_operations: 2, // changed in v10: each document operation is built against the batch's earlier ones; a batch writing one token balance, supply, pool total or escrow twice is refused, and repeated balance writes are merged
            apply_drive_operations: 2, // changed in v10: an estimate describes each layer by the costliest description any write of the batch gives it; each document operation is built against the batch's earlier ones
        },
        prefunded_specialized_balances: DrivePrefundedSpecializedMethodVersions {
            estimated_cost_for_prefunded_specialized_balance_update: 2, // changed in v10: keeps a deeper prefunded balances layer description
            ..DRIVE_VERSION_V9.methods.prefunded_specialized_balances
        },
        vote: DRIVE_VOTE_METHOD_VERSIONS_V4, // changed in v10: the yes/no poll kind
        verify: DRIVE_VERIFY_METHOD_VERSIONS_V4, // changed in v10: the yes/no proof verifiers
        state_transitions: DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5, // changed in v10: a consume deletes what its lookup read; TokenDestroyFrozenFunds also destroys the frozen identity's escrowed listings
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
