use super::dpp_versions::dpp_contract_versions::v7::CONTRACT_VERSIONS_V7;
use super::dpp_versions::DPPVersion;
use super::drive_abci_versions::drive_abci_method_versions::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use super::drive_abci_versions::drive_abci_validation_versions::v11::DRIVE_ABCI_VALIDATION_VERSIONS_V11;
use super::drive_abci_versions::DriveAbciVersion;
use super::drive_versions::v10::DRIVE_VERSION_V10;
use super::v14::PLATFORM_V14;
use super::{PlatformVersion, ProtocolVersion};

pub const PROTOCOL_VERSION_15: ProtocolVersion = 15;

/// v15 hosts one consensus change:
///
/// 1. **Token shielded pools**: a token configuration in format version 1
///    (`TokenConfiguration::V1`, admitted by `CONTRACT_VERSIONS_V7`'s
///    `token_configuration_format` bounds) can set `hasShieldedPool`, which
///    gives the token its own Orchard pool under
///    `[Tokens, TOKEN_SHIELDED_POOLS_KEY, token_id]` laid out like the credit
///    pool. A pooled token must leave its freeze, unfreeze and destroy-frozen-
///    funds rules unassigned, since notes have no owner to freeze. Seven batch
///    token transitions (`TokenShield`, `TokenUnshield`,
///    `TokenShieldedTransfer`, `TokenMintToPool`, `TokenBurnFromPool`,
///    `TokenClaimToPool` and `TokenDirectPurchaseToPool`, validated through
///    `DRIVE_ABCI_VALIDATION_VERSIONS_V11` and gated by
///    `TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION`) move tokens between an
///    identity balance, the supply and the pool or inside it; the identity
///    signs and pays the fee in credits, and every spend bundle binds the
///    token id and owner id (plus the recipient and amount where tokens leave
///    the pool) into the Orchard sighash. The pool balances are a term of the
///    token conservation check (`calculate_total_tokens_balance` v1 in
///    `DRIVE_TOKEN_METHOD_VERSIONS_V3`, reached through `DRIVE_VERSION_V10`).
///    `record_token_shielded_pool_anchors` (`DRIVE_ABCI_METHOD_VERSIONS_V11`)
///    records and prunes the anchors of the pools a block touched. The pools
///    root tree is inserted by the version 15 upgrade transition and by
///    `create_initial_state_structure` v5; the six shielded queries accept an
///    optional `token_id` to target a token pool.
///
/// Everything else matches v14.
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    drive: DRIVE_VERSION_V10,
    drive_abci: DriveAbciVersion {
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11,
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V11,
        ..PLATFORM_V14.drive_abci
    },
    dpp: DPPVersion {
        contract_versions: CONTRACT_VERSIONS_V7,
        ..PLATFORM_V14.dpp
    },
    ..PLATFORM_V14
};
