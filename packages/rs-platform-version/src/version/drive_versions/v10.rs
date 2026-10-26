use crate::version::drive_versions::drive_contract_method_versions::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractFeePotMethodVersions, DriveContractMethodVersions,
};
use crate::version::drive_versions::drive_identity_method_versions::v2::DRIVE_IDENTITY_METHOD_VERSIONS_V2;
use crate::version::drive_versions::drive_identity_method_versions::{
    DriveIdentityCostEstimationMethodVersions, DriveIdentityMethodVersions,
};
use crate::version::drive_versions::drive_state_transition_method_versions::v5::DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_token_method_versions::v3::DRIVE_TOKEN_METHOD_VERSIONS_V3;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{
    DriveBatchOperationsMethodVersion, DriveInitializationMethodVersions, DriveMethodVersions,
    DrivePrefundedSpecializedMethodVersions, DriveVersion,
};

/// Drive version 10.
/// Introduced in protocol v15 for the token marketplace:
///
/// * **Escrow trees** — `create_initial_state_structure` 5 adds the two escrow trees of the
///   token marketplace: `[Tokens, TOKEN_ORDER_ESCROW_KEY]`, the BigSumTree of the tokens open
///   listings hold, and `[PreFundedSpecializedBalances, PREFUNDED_BALANCES_FOR_TOKEN_ORDERS]`,
///   the SumTree of the credits open offers hold. The upgrade path
///   (`Platform::transition_to_version_15`) inserts them through the same helper.
/// * **Token conservation** — `DRIVE_TOKEN_METHOD_VERSIONS_V3` bumps
///   `calculate_total_tokens_balance` to 2, which counts the escrowed listings on the balance
///   side; its escrow methods, `None` before, are active.
/// * **Escrow fee estimates** — the offers' escrow key sits a level below the other keys of the
///   prefunded specialized balances layer, so `estimated_cost_for_prefunded_specialized_balance_update`
///   2 and `add_estimation_costs_for_contract_fee_pot_update` 1 keep a deeper description of that
///   layer another write of the batch registered, rather than lowering it to their keys' level;
///   likewise the listings' escrow key sits deeper in the Tokens layer than the balances key, and
///   `for_token_balances` 1 keeps a deeper description of the Tokens layer. The escrows also sit
///   deeper in the root tree than the identity balances and the token balances, so
///   `for_balances` 1, `for_token_balances` 1 and the voting balance estimation 2 keep a deeper
///   description of the root as well. Beyond the estimators, `apply_drive_operations` 2 lowers
///   each operation of an estimated batch with layer descriptions of its own and merges them
///   into one covering both (the deeper depth, with the nodes priced dearer there), so no other
///   write of the batch (a document, a nonce) lowers what the escrows describe, whichever is
///   built first. It raises the estimate of any batch whose operations describe a shared layer
///   differently, an ordinary document transition included.
/// * **Frozen listings** — `DRIVE_TOKEN_METHOD_VERSIONS_V3` reads a frozen identity's open
///   listings (`order_escrow.fetch_frozen_listings`) and `DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5`
///   has `TokenDestroyFrozenFunds` destroy what they hold with its balance.
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        initialization: DriveInitializationMethodVersions {
            create_initial_state_structure: 5, // changed in v10: adds the token marketplace escrow trees
        },
        token: DRIVE_TOKEN_METHOD_VERSIONS_V3, // changed in v10: calculate_total_tokens_balance 2 counts the token marketplace escrow; order_escrow active; a frozen identity's listings are read
        state_transitions: DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5, // changed in v10: TokenDestroyFrozenFunds also destroys the frozen identity's escrowed listings
        contract: DriveContractMethodVersions {
            fee_pots: DriveContractFeePotMethodVersions {
                add_estimation_costs_for_contract_fee_pot_update: 1, // changed in v10: keeps a deeper prefunded balances layer description
                ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.fee_pots
            },
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4
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
            convert_drive_operations_to_grove_operations: 2, // changed in v10: each operation is built against the batch's earlier ones
            apply_drive_operations: 2, // changed in v10: an estimate merges each operation's layer descriptions into one covering both
        },
        prefunded_specialized_balances: DrivePrefundedSpecializedMethodVersions {
            estimated_cost_for_prefunded_specialized_balance_update: 2, // changed in v10: keeps a deeper prefunded balances layer description
            ..DRIVE_VERSION_V9.methods.prefunded_specialized_balances
        },
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
