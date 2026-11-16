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
use crate::version::mocks::TEST_PROTOCOL_VERSION_SHIFT_BYTES;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v4::SYSTEM_DATA_CONTRACT_VERSIONS_V4;
use crate::version::system_data_contract_versions::SystemDataContractVersions;
use crate::version::v15::PLATFORM_V15;

pub const TEST_PROTOCOL_VERSION_5: u32 = (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES) + 5;

/// Protocol version 15 with the token marketplace, which starts with protocol version 16
/// (`TOKEN_MARKETPLACE_INITIAL_PROTOCOL_VERSION`). That version has no tables yet, so the
/// marketplace's code is tested under this one. No network runs it. What it selects on top of
/// protocol version 15 is what protocol version 16's tables take over:
///
/// - `create_initial_state_structure` 5 creates the two escrow trees at genesis:
///   `[Tokens, TOKEN_ORDER_ESCROW_KEY]`, a BigSumTree of one SumTree per token of the tokens its
///   open listings hold, and `[PreFundedSpecializedBalances,
///   PREFUNDED_BALANCES_FOR_TOKEN_ORDERS]`, a SumTree of the credits open offers hold. An
///   upgrade inserts them through the same helper (`Drive::insert_token_order_escrow_trees`).
/// - `DRIVE_TOKEN_METHOD_VERSIONS_V3`: `calculate_total_tokens_balance` 2 counts the escrowed
///   tokens on the balance side of the token conservation check, and the escrow methods
///   (`order_escrow`), `None` in every other table, are active.
/// - The offers' escrow key sits a level below the other keys of the prefunded specialized
///   balances layer, so `estimated_cost_for_prefunded_specialized_balance_update` 2 and
///   `add_estimation_costs_for_contract_fee_pot_update` 1 keep a deeper description of that
///   layer another write of the batch registered, rather than lowering it to their keys' level.
///   Likewise the listings' escrow key sits deeper in the Tokens layer than the balances key,
///   and both escrows sit deeper in the root tree than the identity and token balances, so
///   `for_balances` 1 and `for_token_balances` 1 keep a deeper description of those layers.
/// - The token market contract v1 (`token_market: 1`), whose `ask` and `bid` types hold the open
///   orders, registered at genesis and stored without storage flags by an upgrade.
pub const TEST_PLATFORM_V5: PlatformVersion = PlatformVersion {
    protocol_version: TEST_PROTOCOL_VERSION_5,
    drive: DriveVersion {
        methods: DriveMethodVersions {
            initialization: DriveInitializationMethodVersions {
                create_initial_state_structure: 5,
            },
            token: DRIVE_TOKEN_METHOD_VERSIONS_V3,
            contract: DriveContractMethodVersions {
                fee_pots: DriveContractFeePotMethodVersions {
                    add_estimation_costs_for_contract_fee_pot_update: 1,
                    ..DRIVE_VERSION_V10.methods.contract.fee_pots
                },
                ..DRIVE_VERSION_V10.methods.contract
            },
            identity: DriveIdentityMethodVersions {
                cost_estimation: DriveIdentityCostEstimationMethodVersions {
                    for_balances: 1,
                    for_token_balances: 1,
                    ..DRIVE_VERSION_V10.methods.identity.cost_estimation
                },
                ..DRIVE_VERSION_V10.methods.identity
            },
            prefunded_specialized_balances: DrivePrefundedSpecializedMethodVersions {
                estimated_cost_for_prefunded_specialized_balance_update: 2,
                ..DRIVE_VERSION_V10.methods.prefunded_specialized_balances
            },
            ..DRIVE_VERSION_V10.methods
        },
        ..DRIVE_VERSION_V10
    },
    system_data_contracts: SystemDataContractVersions {
        token_market: 1,
        ..SYSTEM_DATA_CONTRACT_VERSIONS_V4
    },
    ..PLATFORM_V15
};
