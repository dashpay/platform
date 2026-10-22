use crate::version::drive_versions::drive_contract_method_versions::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractFeePotMethodVersions, DriveContractMethodVersions,
};
use crate::version::drive_versions::drive_token_method_versions::v3::DRIVE_TOKEN_METHOD_VERSIONS_V3;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{
    DriveInitializationMethodVersions, DriveMethodVersions,
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
///   layer another write of the batch registered, rather than lowering it to their keys' level.
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        initialization: DriveInitializationMethodVersions {
            create_initial_state_structure: 5, // changed in v10: adds the token marketplace escrow trees
        },
        token: DRIVE_TOKEN_METHOD_VERSIONS_V3, // changed in v10: calculate_total_tokens_balance 2 counts the token marketplace escrow; order_escrow active
        contract: DriveContractMethodVersions {
            fee_pots: DriveContractFeePotMethodVersions {
                add_estimation_costs_for_contract_fee_pot_update: 1, // changed in v10: keeps a deeper prefunded balances layer description
                ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.fee_pots
            },
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4
        },
        prefunded_specialized_balances: DrivePrefundedSpecializedMethodVersions {
            estimated_cost_for_prefunded_specialized_balance_update: 2, // changed in v10: keeps a deeper prefunded balances layer description
            ..DRIVE_VERSION_V9.methods.prefunded_specialized_balances
        },
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
