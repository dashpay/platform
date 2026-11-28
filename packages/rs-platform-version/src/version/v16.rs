use crate::version::dpp_versions::dpp_method_versions::v5::DPP_METHOD_VERSIONS_V5;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_method_versions::v12::DRIVE_ABCI_METHOD_VERSIONS_V12;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v11::DRIVE_VERSION_V11;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v5::SYSTEM_DATA_CONTRACT_VERSIONS_V5;
use crate::version::system_limits::v6::SYSTEM_LIMITS_V6;
use crate::version::v15::PLATFORM_V15;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_16: ProtocolVersion = 16;

/// Introduced as the activation gate for the consensus changes of the 5.2 line.
///
/// Every table this version does not name is protocol version 15's (`..PLATFORM_V15`), so a
/// change made in place to an unreleased protocol version 15 table reaches this version too.
/// Each change that needs this gate lands in its own follow-up, adds a numbered note here and
/// names the table it bumps in `PLATFORM_V16` with a `// changed:` comment.
///
/// v16 hosts these consensus changes:
///
/// 1. **Token marketplace state**: the token marketplace starts with this version
///    (`TOKEN_MARKETPLACE_INITIAL_PROTOCOL_VERSION`). This version admits tradeable tokens
///    (`marketplaceRules.tradeMode` `Tradeable`) and creates the escrow trees and the token
///    market contract that listings and offers will use; the transitions that place and fill
///    orders come in follow-ups. Protocol v15 and earlier hold none of it (v15 note 17).
///    - A token configuration, or a configuration update, that makes a token tradeable is
///      admitted; before this version it is refused unpaid as not active.
///    - `create_initial_state_structure` 5 (`DRIVE_VERSION_V11`) creates the two escrow trees at
///      genesis, and the upgrade to this version (`transition_to_version_16`) inserts them
///      through the same helper, `Drive::insert_token_order_escrow_trees`.
///    - `calculate_total_tokens_balance` 2 counts the escrowed tokens in the token
///      conservation check, and the escrow methods (`order_escrow`) are active
///      (`DRIVE_TOKEN_METHOD_VERSIONS_V3`).
///    - The escrow keys sit deeper in their layers than the keys beside them, so the cost
///      estimators of the balances, the token balances, the prefunded specialized balances and
///      the contract fee pots keep a deeper description of those layers (`DRIVE_VERSION_V11`).
///    - The token market contract v1 (`token_market: 1`, `SYSTEM_DATA_CONTRACT_VERSIONS_V5`) is
///      registered at genesis and stored without storage flags by the upgrade. Its `ask` and
///      `bid` types hold the open orders.
/// 2. **Largest withdrawal 4000 Dash**: one withdrawal carries at most 4000 Dash
///    (`SYSTEM_LIMITS_V6.max_withdrawal_amount`), where protocol versions 1 to 15 allow 500
///    Dash. The identity, address and shielded withdrawal validations read the limit and are
///    otherwise unchanged; a withdrawal over it is refused as before.
///    - The daily withdrawal limit keeps its numbers. Version 2 floors it at one maximal
///      withdrawal, which would raise the daily limit of a Platform holding less than about
///      26,667 Dash from 15% of its credits to 4000 Dash. `daily_withdrawal_limit` 3
///      (`DPP_METHOD_VERSIONS_V5`) floors it at `SYSTEM_LIMITS_V6.daily_withdrawal_limit_floor`
///      instead, 500 Dash as before, so a withdrawal can now be larger than the daily maximum.
///    - The Core-anchored limit of protocol version 14 (v14 note 74) is unchanged: the pool
///      may drop by 15% of its window start balance, at least 1500 Dash. Its floor cannot
///      follow the largest withdrawal, because Core's own is 2000 Dash, so a withdrawal can
///      now be larger than the limit ever gets: a 4000 Dash withdrawal fits only while Core's
///      credit pool holds about 26,667 Dash or received as much inside the window.
///    - `pool_withdrawals_into_transactions_queue` 3 (`DRIVE_ABCI_METHOD_VERSIONS_V12`) keeps
///      such a withdrawal from holding the queue. It considers the 16 oldest queued
///      withdrawals (`SYSTEM_LIMITS_V6.withdrawal_pooling_candidates_per_block_limit`), passes
///      over one that does not fit what the daily limit and the Core-anchored limit have left,
///      and pools the ones behind it that fit, up to the 4 a block pools; version 2 stops at
///      the first that does not fit. A withdrawal passed over stays queued, is considered
///      first again in the next block, and is pooled once the limits admit it. The queue is
///      therefore no longer served strictly in order: while the limits are short, a smaller
///      withdrawal is pooled before an older, larger one.
///    - Dash Core before its V24 fork admits no asset unlock above 2000 Dash
///      (`LimitAmountV22`), so it does not mine a larger withdrawal. This version does not
///      wait for V24: a network must have V24 active on Core before it activates protocol
///      version 16.
pub const PLATFORM_V16: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_16,
    drive: DRIVE_VERSION_V11, // changed: create_initial_state_structure 5 creates the token order escrow trees; the token methods of the marketplace and the deeper escrow layer estimates (note 1)
    dpp: DPPVersion {
        methods: DPP_METHOD_VERSIONS_V5, // changed: daily_withdrawal_limit 3 floors the daily limit at its own 500 Dash, not at the largest withdrawal (note 2)
        ..PLATFORM_V15.dpp
    },
    drive_abci: DriveAbciVersion {
        methods: DRIVE_ABCI_METHOD_VERSIONS_V12, // changed: pool_withdrawals_into_transactions_queue 3 passes over a queued withdrawal that does not fit the limits (note 2)
        ..PLATFORM_V15.drive_abci
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V5, // changed: the token market contract v1 (note 1)
    system_limits: SYSTEM_LIMITS_V6, // changed: max_withdrawal_amount is 4000 Dash, daily_withdrawal_limit_floor keeps the daily floor at 500 Dash, and pooling considers 16 queued withdrawals per block (note 2)
    ..PLATFORM_V15
};
