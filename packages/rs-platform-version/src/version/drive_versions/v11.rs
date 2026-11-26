use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractFeePotMethodVersions, DriveContractMethodVersions,
};
use crate::version::drive_versions::drive_identity_method_versions::{
    DriveIdentityCostEstimationMethodVersions, DriveIdentityMethodVersions,
};
use crate::version::drive_versions::drive_token_method_versions::v3::DRIVE_TOKEN_METHOD_VERSIONS_V3;
use crate::version::drive_versions::v10::DRIVE_VERSION_V10;
use crate::version::drive_versions::{
    DriveInitializationMethodVersions, DriveMethodVersions,
    DrivePrefundedSpecializedMethodVersions, DriveVersion,
};

/// Drive version 11, introduced in protocol v16:
///
/// * **Token marketplace** (protocol v16 note 1): `create_initial_state_structure` 5 creates
///   the two escrow trees at genesis: `[Tokens, TOKEN_ORDER_ESCROW_KEY]`, a BigSumTree of one
///   SumTree per token of the tokens its open listings hold, and
///   `[PreFundedSpecializedBalances, PREFUNDED_BALANCES_FOR_TOKEN_ORDERS]`, a SumTree of the
///   credits open offers hold. The upgrade to protocol v16 inserts them through the same
///   helper (`Drive::insert_token_order_escrow_trees`). `DRIVE_TOKEN_METHOD_VERSIONS_V3`:
///   `calculate_total_tokens_balance` 2 counts the escrowed tokens on the balance side of the
///   token conservation check, and the escrow methods (`order_escrow`), `None` in every earlier
///   table, are active.
/// * **Escrow layers described as deep as they are**: the offers' escrow key sits a level
///   below the other keys of the prefunded specialized balances layer, so
///   `estimated_cost_for_prefunded_specialized_balance_update` 2 and
///   `add_estimation_costs_for_contract_fee_pot_update` 1 keep a deeper description of that
///   layer another write of the batch registered, rather than lowering it to their keys' level.
///   Likewise the listings' escrow key sits deeper in the Tokens layer than the balances key,
///   and both escrows sit deeper in the root tree than the identity and token balances, so
///   `for_balances` 1 and `for_token_balances` 1 keep a deeper description of those layers.
///
/// Everything else matches `DRIVE_VERSION_V10`.
pub const DRIVE_VERSION_V11: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        initialization: DriveInitializationMethodVersions {
            create_initial_state_structure: 5, // changed in v11: the token order escrow trees
        },
        token: DRIVE_TOKEN_METHOD_VERSIONS_V3, // changed in v11: escrow methods, escrow in the token conservation check
        contract: DriveContractMethodVersions {
            fee_pots: DriveContractFeePotMethodVersions {
                add_estimation_costs_for_contract_fee_pot_update: 1, // changed in v11: keeps a deeper layer description
                ..DRIVE_VERSION_V10.methods.contract.fee_pots
            },
            ..DRIVE_VERSION_V10.methods.contract
        },
        identity: DriveIdentityMethodVersions {
            cost_estimation: DriveIdentityCostEstimationMethodVersions {
                for_balances: 1,       // changed in v11: keeps a deeper layer description
                for_token_balances: 1, // changed in v11: keeps a deeper layer description
                ..DRIVE_VERSION_V10.methods.identity.cost_estimation
            },
            ..DRIVE_VERSION_V10.methods.identity
        },
        prefunded_specialized_balances: DrivePrefundedSpecializedMethodVersions {
            estimated_cost_for_prefunded_specialized_balance_update: 2, // changed in v11: keeps a deeper layer description
            ..DRIVE_VERSION_V10.methods.prefunded_specialized_balances
        },
        ..DRIVE_VERSION_V10.methods
    },
    ..DRIVE_VERSION_V10
};
