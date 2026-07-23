//! Wave G token-harness extensions.
//!
//! Helpers for the TK-NNN test column: deploy permissive
//! token contracts, mint/transfer/freeze, and read back token state
//! through the SDK without per-test plumbing. Mirrors DET's
//! `tests/backend-e2e/framework/token_helpers.rs` but composes
//! against the e2e harness's [`E2eContext`] and Wave A
//! [`RegisteredIdentity`].
//!
//! All read accessors come in two shapes: the high-level "of"
//! variant operates on a deployed [`TokenContractFixture`] / typed
//! `RegisteredIdentity`, and a lower-level `*_raw` variant accepts
//! raw 32-byte ids for tests that probe across contracts.
//!
//! Status: Wave G framework helpers only — Wave 2 wires up TK-NNN
//! test cases that exercise these. Runtime correctness is verified
//! in Wave 4 against a live testnet.
//!
//! Editorial notes:
//! - `register_token_contract_via_sdk` signs with the
//!   [`RegisteredIdentity::high_key`] (HIGH, KeyID 1).
//!   `DataContractCreateTransitionV0::security_level_requirement`
//!   accepts only CRITICAL or HIGH (see
//!   `rs-dpp/.../data_contract_create_transition/v0/identity_signed.rs`),
//!   so signing with MASTER triggers
//!   `InvalidSignaturePublicKeySecurityLevelError` at chain validation.
//! - All token-batch state transitions (`mint_to` and the per-case
//!   `token_*` calls in TK-NNN) MUST sign with
//!   [`RegisteredIdentity::critical_key`] (AUTHENTICATION + CRITICAL,
//!   KeyID 3). `TokenBaseTransition`'s
//!   `IdentitySignedV0::security_level_requirement` returns only
//!   `vec![SecurityLevel::CRITICAL]`; HIGH or MASTER yields
//!   `InvalidSignaturePublicKeySecurityLevelError` at chain validation.
//! - `token_frozen_balance_of` returns a [`TokenAmount`] (the
//!   identity's full token balance when `IdentityTokenInfo.frozen`
//!   is `true`, else `0`). DPP only stores a `frozen: bool`; the
//!   "frozen-balance" framing in TK-009/010/011 means "balance
//!   that would be unspendable due to the freeze flag".

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::transition::put_contract::PutContract;
use dash_sdk::platform::{Fetch, FetchMany};
use dash_sdk::Sdk;
use dpp::balances::credits::TokenAmount;
use dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::{Identifier, TimestampMillis};
use dpp::tokens::calculate_token_id;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::tokens::status::v0::TokenStatusV0Accessors;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::version::PlatformVersion;
use serde_json::json;

use dash_sdk::platform::tokens::builders::mint::TokenMintTransitionBuilder;
use dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalancesQuery;
use dash_sdk::platform::tokens::token_info::IdentityTokenInfosQuery;

use super::harness::E2eContext;
use super::wallet_factory::RegisteredIdentity;
use super::{FrameworkError, FrameworkResult, MultiIdentitySetupGuard};

/// Default TK-NNN token slot. The permissive owner-only contract
/// always deploys a single token at position `0`.
pub const DEFAULT_TOKEN_POSITION: TokenContractPosition = 0;

/// Default TK-NNN base supply (zero — owner mints in-test).
pub const DEFAULT_BASE_SUPPLY: TokenAmount = 0;

/// Default TK-NNN max supply (`1e15`, mirrors DET).
pub const DEFAULT_MAX_SUPPLY: TokenAmount = 1_000_000_000_000_000;

/// Default TK-NNN decimals (8, mirrors DET).
pub const DEFAULT_DECIMALS: u8 = 8;

/// Owner funding for permissive owner-only token contracts (TK-001,
/// 003, 005, 007, 008, 009, 010, 011, 014). Covers the chain-enforced
/// `base_contract_registration_fee + token_registration_fee` floor
/// (20B credits) plus 1B follow-up headroom. Observed v42 shortfall
/// was ~67M against a ~205M typical mint; 1B headroom gives ~15×
/// margin against future protocol fee changes.
pub const TK_OWNER_FUNDING_SIMPLE: dpp::fee::Credits = 21_000_000_000;

/// Owner funding for token contracts with a perpetual or pre-programmed
/// distribution (TK-002, TK-013). Adds the `distribution_fee × 1`
/// charge on top of [`TK_OWNER_FUNDING_SIMPLE`]'s 20B floor (→ 30B
/// chain floor) plus the same 1B follow-up headroom.
pub const TK_OWNER_FUNDING_DISTRIBUTION: dpp::fee::Credits = 31_000_000_000;

/// Owner funding for token contracts that follow up with a
/// token-config-update transition (TK-012). Token-config-update costs
/// ~664M on testnet (3× a typical mint), so the 1B follow-up headroom
/// in [`TK_OWNER_FUNDING_SIMPLE`] doesn't cover it. 20B chain floor
/// + 2B follow-up headroom.
pub const TK_OWNER_FUNDING_CONFIG_UPDATE: dpp::fee::Credits = 22_000_000_000;

/// Peer funding for passive receivers — identities that never create a
/// contract and never sign their own state transitions (TK-001's
/// transfer destination, TK-005b's mint recipient). Passive peers need
/// ~200M for basic state transitions; 500M gives safety headroom
/// against fee-tick noise.
pub const TK_PEER_FUNDING: dpp::fee::Credits = 500_000_000;

/// Peer funding for "active" peers — identities that themselves sign
/// state transitions during the test body (TK-007 frozen-transfer
/// attempt, TK-008 post-unfreeze transfer, TK-011 token purchase,
/// TK-014 group co-sign). Group co-sign (TK-014) needs up to ~230M
/// post-registration; 2.5B leaves comfortable headroom.
pub const TK_PEER_FUNDING_ACTIVE: dpp::fee::Credits = 2_500_000_000;

/// Per-step propagation budget used by the TK-NNN suite. The TK
/// setup funds ~35 B credits per identity in a single hop and runs
/// under high parallel churn on the process-shared bank wallet
/// (`worker_threads = 12`); the 60 s `DEFAULT_SETUP_STEP_TIMEOUT`
/// undershoots the cross-replica replication lag we see when sibling
/// guards are simultaneously draining the bank's funding pool.
/// (QA-V39-002.)
pub const TK_SETUP_WAIT_TIMEOUT: Duration = Duration::from_secs(120);

/// Pre-programmed distribution rule passed to
/// [`setup_with_token_pre_programmed_distribution`].
///
/// Each entry says: at `timestamp_ms`, credit `recipient` with
/// `amount`. The harness embeds this verbatim into the V1
/// `tokens["0"].distributionRules.preProgrammedDistribution.distributions`
/// node so `token_claim_with_signer` can claim against a past-timestamp
/// epoch without waiting on live block time.
#[derive(Debug, Clone)]
pub struct PreProgrammedDistribution {
    /// Distribution timeline. Each timestamp may credit one or more
    /// identities — Wave 2 TK-013 uses a single past timestamp with
    /// the owner as the sole recipient.
    pub distributions: BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>,
}

/// Perpetual distribution rule passed to
/// [`setup_with_token_perpetual_distribution`].
///
/// Wraps the simplest workable BlockBasedDistribution config (fixed
/// amount per N-block interval, recipient = ContractOwner). The
/// harness embeds this under
/// `tokens["0"].distributionRules.perpetualDistribution` in the V1
/// JSON envelope so `token_claim` with `TokenDistributionType::
/// Perpetual` can claim once `interval_blocks` of platform block
/// height have elapsed since contract creation.
///
/// Only the BlockBased shape is exposed — TimeBased and EpochBased
/// would need their own min-interval headroom (testnet floors:
/// 600_000 ms / 1 epoch) and aren't required by TK-002.
///
/// Testnet enforces a minimum of 5 blocks for BlockBased intervals
/// (see `RewardDistributionType::validate_structure_interval_v0`);
/// passing a smaller value will trip
/// `InvalidTokenDistributionBlockIntervalTooShortError` at chain
/// validation.
#[derive(Debug, Clone)]
pub struct PerpetualDistribution {
    /// Block interval between emissions. Platform block height —
    /// not Core chain height. Must be ≥ 5 on testnet.
    pub interval_blocks: u64,
    /// Tokens emitted to the contract owner per interval.
    pub amount_per_interval: TokenAmount,
}

/// Single-identity TK setup. Returned by
/// [`setup_with_token_contract`] /
/// [`setup_with_token_pre_programmed_distribution`].
///
/// Holds the [`MultiIdentitySetupGuard`] so test bodies can `await
/// guard.teardown()`. The contract id is the canonical
/// chain-derived id (owner + nonce) returned by
/// [`register_token_contract_via_sdk`].
pub struct TokenSetup {
    /// Owns the test wallet + the bank loan. Caller must
    /// `setup_guard.teardown()` at the end of the test body.
    pub setup_guard: MultiIdentitySetupGuard,
    /// Contract owner — funded with `owner_funding` credits at
    /// registration time.
    pub owner: RegisteredIdentity,
    /// Chain-derived data-contract id.
    pub contract_id: Identifier,
    /// Token slot inside the contract; pinned to
    /// [`DEFAULT_TOKEN_POSITION`] for the permissive default.
    pub token_position: TokenContractPosition,
}

impl TokenSetup {
    /// Convenience id for the token at `token_position` —
    /// `calculate_token_id(contract_id, position)`.
    pub fn token_id(&self) -> Identifier {
        Identifier::from(calculate_token_id(
            self.contract_id.as_bytes(),
            self.token_position,
        ))
    }
}

/// Two-identity TK setup — owner + peer.
pub struct TokenTwoIdentitiesSetup {
    /// Underlying single-identity setup (owns the contract id +
    /// teardown guard).
    pub setup: TokenSetup,
    /// Second identity registered alongside the owner.
    pub peer: RegisteredIdentity,
}

/// Three-identity TK setup — owner + two peers (TK-014 group co-sign).
pub struct TokenThreeIdentitiesSetup {
    /// Underlying single-identity setup.
    pub setup: TokenSetup,
    /// Two extra identities (peer_a, peer_b).
    pub peers: [RegisteredIdentity; 2],
}

// ---------------------------------------------------------------------------
// 14. register_token_contract_via_sdk — SDK-direct deploy
// ---------------------------------------------------------------------------

/// Build a V1 token-contract document from `contract_json` and
/// broadcast it via [`PutContract::put_to_platform_and_wait_for_response`].
///
/// `contract_json` is the V1 `tokens` object, keyed by stringified
/// slot index (`"0"`, `"1"`, …). The helper wraps it in the rest of
/// the V1 envelope (`$formatVersion`, `id`, `ownerId`, `version`,
/// empty `documentSchemas`) before round-tripping through
/// [`DataContractInSerializationFormat`] — mirrors the wallet's
/// `create_data_contract_with_signer` path so the schema-drift
/// surface stays in one shape.
///
/// Signs with [`RegisteredIdentity::high_key`] (HIGH) — the chain
/// rejects MASTER on `DataContractCreate` (CRITICAL or HIGH only).
pub async fn register_token_contract_via_sdk(
    ctx: &E2eContext,
    owner: &RegisteredIdentity,
    contract_json: serde_json::Value,
) -> FrameworkResult<Identifier> {
    let placeholder_id = Identifier::default();

    let mut envelope = serde_json::Map::new();
    envelope.insert("$formatVersion".into(), json!("1"));
    envelope.insert(
        "id".into(),
        json!(bs58::encode(placeholder_id.to_buffer()).into_string()),
    );
    envelope.insert(
        "ownerId".into(),
        json!(bs58::encode(owner.id.to_buffer()).into_string()),
    );
    envelope.insert("version".into(), json!(1u32));
    envelope.insert("documentSchemas".into(), json!({}));
    envelope.insert("tokens".into(), contract_json);

    let serialized = serde_json::to_string(&serde_json::Value::Object(envelope))
        .map_err(|err| FrameworkError::Sdk(format!("token-contract serialize: {err}")))?;
    let format: DataContractInSerializationFormat = serde_json::from_str(&serialized)
        .map_err(|err| FrameworkError::Sdk(format!("token-contract deserialize: {err}")))?;

    let platform_version = PlatformVersion::latest();
    let mut errors = vec![];
    let data_contract =
        DataContract::try_from_platform_versioned(format, true, &mut errors, platform_version)
            .map_err(|err| {
                FrameworkError::Sdk(format!("token-contract build: {err} (errors={errors:?})"))
            })?;

    // SDK fetches+bumps the identity nonce internally and overwrites
    // the placeholder id with the canonical (owner, nonce) derivation.
    let confirmed = data_contract
        .put_to_platform_and_wait_for_response(
            ctx.sdk(),
            owner.high_key.clone(),
            owner.signer.as_ref(),
            None,
        )
        .await
        .map_err(|err| FrameworkError::Sdk(format!("put_to_platform: {err}")))?;

    let contract_id = confirmed.id();

    // Gate against DAPI propagation lag: a follow-up state transition
    // (e.g. token_mint) may land on a replica that hasn't replicated
    // the new contract yet. Wait until 2 consecutive fetches succeed.
    crate::framework::wait::wait_for_data_contract_visible(
        ctx.sdk(),
        contract_id,
        Duration::from_secs(60),
        2,
    )
    .await?;

    // QA-900 — register the just-deployed contract (and any token
    // configurations it carries) with the SDK's
    // `TrustedHttpContextProvider`. Without this, the next proof
    // verification that resolves the contract id (e.g. the chain
    // round-trip on `Sdk::token_mint`) walks the static system-contract
    // map, misses, and surfaces
    // `DriveProofError(UnknownContract("... in token verification"))`.
    register_contract_with_context_provider(ctx, &confirmed);

    Ok(contract_id)
}

/// Register a freshly-deployed [`DataContract`] (plus all of its V1
/// token slots) with the harness's shared
/// [`TrustedHttpContextProvider`]. Idempotent — repeated calls just
/// re-insert the same entries. Lifts the post-deploy registration step
/// that otherwise needs to be repeated at every contract-creating
/// site. (QA-900)
pub fn register_contract_with_context_provider(ctx: &E2eContext, contract: &DataContract) {
    let contract_id = contract.id();
    ctx.context_provider().add_known_contract(contract.clone());

    // Token-slot configurations let the proof verifier resolve
    // per-token settings (decimals, freeze rules, etc.) without a
    // round-trip through the (still-unfetched) contract. Mirrors the
    // same canonical token-id derivation used by the read accessors
    // below — `calculate_token_id(contract_id, position)`.
    let positions: Vec<TokenContractPosition> = contract.tokens().keys().copied().collect();
    for position in positions {
        let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), position));
        if let Some(config) = contract.tokens().get(&position).cloned() {
            ctx.context_provider()
                .add_known_token_configuration(token_id, config);
        }
    }

    tracing::debug!(
        target: "platform_wallet::e2e::tokens",
        ?contract_id,
        token_positions = ?contract.tokens().keys().copied().collect::<Vec<_>>(),
        "registered freshly-deployed contract with TrustedHttpContextProvider (QA-900)"
    );
}

// ---------------------------------------------------------------------------
// 18. permissive_owner_token_contract_json — V1 JSON template
// ---------------------------------------------------------------------------

/// Build the V1 `tokens` JSON node for a permissive owner-only token
/// contract, mirroring DET's
/// `tests/backend-e2e/framework/token_helpers.rs:33`
/// (`build_register_token_task`): 8 decimals, owner-only
/// ChangeControlRules across every gate, no perpetual distribution,
/// `mintingAllowChoosingDestination = true`,
/// `allowTransferToFrozenBalance = false`,
/// `marketplaceTradeMode = 1`.
///
/// The returned [`serde_json::Value`] is the
/// `tokens` map (`{"0": {...}}`) ready to drop into
/// [`register_token_contract_via_sdk`].
pub fn permissive_owner_token_contract_json(
    owner_id: Identifier,
    position: u16,
    supply: TokenAmount,
) -> serde_json::Value {
    let owner_b58 = bs58::encode(owner_id.to_buffer()).into_string();
    let owner_only = json!({
        "$formatVersion": "0",
        "authorizedToMakeChange": {"$type": "contractOwner"},
        "adminActionTakers": {"$type": "contractOwner"},
        "changingAuthorizedActionTakersToNoOneAllowed": false,
        "changingAdminActionTakersToNoOneAllowed": false,
        "selfChangingAdminActionTakersAllowed": false,
    });

    let token_slot = json!({
        "$formatVersion": "0",
        "conventions": {
            "$formatVersion": "0",
            "decimals": DEFAULT_DECIMALS,
            "localizations": {
                "en": {
                    "$formatVersion": "0",
                    "shouldCapitalize": false,
                    "singularForm": "E2ETestToken",
                    "pluralForm": "E2ETestTokens",
                }
            },
        },
        "conventionsChangeRules": owner_only,
        "baseSupply": DEFAULT_BASE_SUPPLY,
        "maxSupply": supply,
        "keepsHistory": {
            "$formatVersion": "0",
            "keepsTransferHistory": true,
            "keepsFreezingHistory": true,
            "keepsMintingHistory": true,
            "keepsBurningHistory": true,
            "keepsDirectPricingHistory": true,
            "keepsDirectPurchaseHistory": true,
        },
        "startAsPaused": false,
        "allowTransferToFrozenBalance": false,
        "maxSupplyChangeRules": owner_only,
        "distributionRules": {
            "$formatVersion": "0",
            "perpetualDistribution": null,
            "perpetualDistributionRules": owner_only,
            "preProgrammedDistribution": null,
            "newTokensDestinationIdentity": owner_b58,
            "newTokensDestinationIdentityRules": owner_only,
            "mintingAllowChoosingDestination": true,
            "mintingAllowChoosingDestinationRules": owner_only,
            "changeDirectPurchasePricingRules": owner_only,
        },
        "manualMintingRules": owner_only,
        "manualBurningRules": owner_only,
        "freezeRules": owner_only,
        "unfreezeRules": owner_only,
        "destroyFrozenFundsRules": owner_only,
        "emergencyActionRules": owner_only,
        "mainControlGroup": null,
        "mainControlGroupCanBeModified": {"$type": "contractOwner"},
        "description": "Permissive owner-only token deployed by rs-platform-wallet e2e (Wave G).",
        "marketplaceRules": {
            "$formatVersion": "0",
            "tradeMode": "NotTradeable",
            "tradeModeChangeRules": owner_only,
        },
    });

    let mut tokens = serde_json::Map::new();
    tokens.insert(position.to_string(), token_slot);
    serde_json::Value::Object(tokens)
}

// ---------------------------------------------------------------------------
// 12. setup_with_token_contract — single-identity bootstrap
// ---------------------------------------------------------------------------

/// Register one identity (via [`setup_with_n_identities`]) and
/// deploy a permissive owner-only token contract owned by it.
/// Returns the [`TokenSetup`] guard so the test body can `setup.
/// setup_guard.teardown()` at the end.
///
/// Non-async sync wrapper so `#[track_caller]` captures the test
/// file before the async state machine is created.
#[track_caller]
pub fn setup_with_token_contract(
    ctx: &E2eContext,
    owner_funding: dpp::fee::Credits,
) -> impl std::future::Future<Output = FrameworkResult<TokenSetup>> + '_ {
    let site_label = super::label_from_file(std::panic::Location::caller().file());
    async move {
        let existing = super::funding_ledger::CURRENT_TEST_LABEL
            .try_with(|o| o.clone())
            .ok()
            .flatten();
        let label = existing.or(site_label);
        super::funding_ledger::maybe_with_test_label(
            label,
            setup_with_token_contract_with_step_timeout(ctx, owner_funding, TK_SETUP_WAIT_TIMEOUT),
        )
        .await
    }
}

/// Per-test override of [`setup_with_token_contract`]'s propagation budget.
///
/// Routes through [`super::setup_with_n_identities_with_step_timeout`] so
/// each waiter inside the identity-bootstrap loop honours `step_timeout`.
/// TK-005 — the only test that funds 35 B credits in a single hop — uses
/// this entry point with a 120 s budget; the 60 s default remains in force
/// for every other token-suite caller.
///
/// Non-async sync wrapper so `#[track_caller]` captures the test
/// file before the async state machine is created.
#[track_caller]
pub fn setup_with_token_contract_with_step_timeout(
    ctx: &E2eContext,
    owner_funding: dpp::fee::Credits,
    step_timeout: Duration,
) -> impl std::future::Future<Output = FrameworkResult<TokenSetup>> + '_ {
    let site_label = super::label_from_file(std::panic::Location::caller().file());
    async move {
        let _ = ctx; // ctx unused; kept for API compat. Captures the reference so lifetime '_ is respected.
        let existing = super::funding_ledger::CURRENT_TEST_LABEL
            .try_with(|o| o.clone())
            .ok()
            .flatten();
        let label = existing.or(site_label);
        super::funding_ledger::maybe_with_test_label(label, async move {
            let setup_guard =
                super::setup_with_n_identities_with_step_timeout(1, owner_funding, step_timeout)
                    .await?;
            let owner = setup_guard
                .identities
                .first()
                .ok_or_else(|| {
                    FrameworkError::Wallet(
                        "setup_with_n_identities returned empty identities vec".into(),
                    )
                })?
                .clone_for_token_setup();

            let json = permissive_owner_token_contract_json(
                owner.id,
                DEFAULT_TOKEN_POSITION,
                DEFAULT_MAX_SUPPLY,
            );
            let contract_id =
                register_token_contract_via_sdk(setup_guard.base.ctx, &owner, json).await?;

            Ok(TokenSetup {
                setup_guard,
                owner,
                contract_id,
                token_position: DEFAULT_TOKEN_POSITION,
            })
        })
        .await
    }
}

// ---------------------------------------------------------------------------
// 13. setup_with_token_and_two_identities
// ---------------------------------------------------------------------------

/// Two-identity TK setup. Identity #0 owns the contract, identity
/// #1 is a peer for transfer / freeze / purchase scenarios. Owner and
/// peer are funded independently — typically
/// [`TK_OWNER_FUNDING_SIMPLE`] + [`TK_PEER_FUNDING`] (or
/// [`TK_PEER_FUNDING_ACTIVE`] when the peer itself signs transitions).
///
/// Non-async sync wrapper so `#[track_caller]` captures the test
/// file before the async state machine is created.
#[track_caller]
pub fn setup_with_token_and_two_identities(
    ctx: &E2eContext,
    owner_funding: dpp::fee::Credits,
    peer_funding: dpp::fee::Credits,
) -> impl std::future::Future<Output = FrameworkResult<TokenTwoIdentitiesSetup>> + '_ {
    let site_label = super::label_from_file(std::panic::Location::caller().file());
    async move {
        let existing = super::funding_ledger::CURRENT_TEST_LABEL
            .try_with(|o| o.clone())
            .ok()
            .flatten();
        let label = existing.or(site_label);
        super::funding_ledger::maybe_with_test_label(
            label,
            setup_with_token_and_two_identities_with_step_timeout(
                ctx,
                owner_funding,
                peer_funding,
                TK_SETUP_WAIT_TIMEOUT,
            ),
        )
        .await
    }
}

/// Per-test override of [`setup_with_token_and_two_identities`]'s
/// propagation budget. Routes through
/// [`super::setup_with_per_identity_funding`] so each waiter
/// inside the identity-bootstrap loop honours `step_timeout`. Used by
/// the round-trip cases that fund 35 B+ credits across two identities
/// concurrently under `--test-threads=14` — the 60 s default is too
/// tight when sibling guards compete for the bank lane.
///
/// Non-async sync wrapper so `#[track_caller]` captures the test
/// file before the async state machine is created.
#[track_caller]
pub fn setup_with_token_and_two_identities_with_step_timeout(
    ctx: &E2eContext,
    owner_funding: dpp::fee::Credits,
    peer_funding: dpp::fee::Credits,
    step_timeout: Duration,
) -> impl std::future::Future<Output = FrameworkResult<TokenTwoIdentitiesSetup>> + '_ {
    let site_label = super::label_from_file(std::panic::Location::caller().file());
    async move {
        let _ = ctx; // ctx unused; kept for API compat.
        let existing = super::funding_ledger::CURRENT_TEST_LABEL
            .try_with(|o| o.clone())
            .ok()
            .flatten();
        let label = existing.or(site_label);
        super::funding_ledger::maybe_with_test_label(label, async move {
            let setup_guard = super::setup_with_per_identity_funding(
                &[owner_funding, peer_funding],
                step_timeout,
            )
            .await?;
            let owner = setup_guard.identities[0].clone_for_token_setup();
            let peer = setup_guard.identities[1].clone_for_token_setup();

            let json = permissive_owner_token_contract_json(
                owner.id,
                DEFAULT_TOKEN_POSITION,
                DEFAULT_MAX_SUPPLY,
            );
            let contract_id =
                register_token_contract_via_sdk(setup_guard.base.ctx, &owner, json).await?;

            Ok(TokenTwoIdentitiesSetup {
                setup: TokenSetup {
                    setup_guard,
                    owner,
                    contract_id,
                    token_position: DEFAULT_TOKEN_POSITION,
                },
                peer,
            })
        })
        .await
    }
}

// ---------------------------------------------------------------------------
// 14. setup_with_token_and_three_identities
// ---------------------------------------------------------------------------

/// Three-identity TK setup — owner plus two peers (TK-014 group
/// co-sign happy path). Owner and the two peers are funded
/// independently — TK-014 has both peers sign group-action
/// transitions, so [`TK_PEER_FUNDING_ACTIVE`] is the typical peer
/// amount.
///
/// Non-async sync wrapper so `#[track_caller]` captures the test
/// file before the async state machine is created.
#[track_caller]
pub fn setup_with_token_and_three_identities(
    ctx: &E2eContext,
    owner_funding: dpp::fee::Credits,
    peer_funding: dpp::fee::Credits,
) -> impl std::future::Future<Output = FrameworkResult<TokenThreeIdentitiesSetup>> + '_ {
    let site_label = super::label_from_file(std::panic::Location::caller().file());
    async move {
        let _ = ctx; // ctx unused; kept for API compat.
        let existing = super::funding_ledger::CURRENT_TEST_LABEL
            .try_with(|o| o.clone())
            .ok()
            .flatten();
        let label = existing.or(site_label);
        super::funding_ledger::maybe_with_test_label(label, async move {
            let setup_guard = super::setup_with_per_identity_funding(
                &[owner_funding, peer_funding, peer_funding],
                TK_SETUP_WAIT_TIMEOUT,
            )
            .await?;
            let owner = setup_guard.identities[0].clone_for_token_setup();
            let peers = [
                setup_guard.identities[1].clone_for_token_setup(),
                setup_guard.identities[2].clone_for_token_setup(),
            ];

            let json = permissive_owner_token_contract_json(
                owner.id,
                DEFAULT_TOKEN_POSITION,
                DEFAULT_MAX_SUPPLY,
            );
            let contract_id =
                register_token_contract_via_sdk(setup_guard.base.ctx, &owner, json).await?;

            Ok(TokenThreeIdentitiesSetup {
                setup: TokenSetup {
                    setup_guard,
                    owner,
                    contract_id,
                    token_position: DEFAULT_TOKEN_POSITION,
                },
                peers,
            })
        })
        .await
    }
}

// ---------------------------------------------------------------------------
// 15. setup_with_token_pre_programmed_distribution
// ---------------------------------------------------------------------------

/// Single-identity TK setup with a pre-programmed distribution
/// rule (TK-013). The caller supplies the `(timestamp →
/// {recipient → amount})` schedule; the helper embeds it under
/// `tokens["0"].distributionRules.preProgrammedDistribution`.
///
/// Tests place a past timestamp here so the first claim becomes
/// eligible immediately, dodging the live-perpetual wall-clock
/// wait that gates TK-002.
///
/// Non-async sync wrapper so `#[track_caller]` captures the test
/// file before the async state machine is created.
#[track_caller]
pub fn setup_with_token_pre_programmed_distribution(
    ctx: &E2eContext,
    owner_funding: dpp::fee::Credits,
    distribution: PreProgrammedDistribution,
) -> impl std::future::Future<Output = FrameworkResult<TokenSetup>> + '_ {
    let site_label = super::label_from_file(std::panic::Location::caller().file());
    async move {
        let _ = ctx; // ctx unused; kept for API compat.
        let existing = super::funding_ledger::CURRENT_TEST_LABEL
            .try_with(|o| o.clone())
            .ok()
            .flatten();
        let label = existing.or(site_label);
        super::funding_ledger::maybe_with_test_label(label, async move {
            let setup_guard = super::setup_with_n_identities_with_step_timeout(
                1,
                owner_funding,
                TK_SETUP_WAIT_TIMEOUT,
            )
            .await?;
            let owner = setup_guard.identities[0].clone_for_token_setup();

            let mut json = permissive_owner_token_contract_json(
                owner.id,
                DEFAULT_TOKEN_POSITION,
                DEFAULT_MAX_SUPPLY,
            );
            let token_slot = json
                .get_mut(DEFAULT_TOKEN_POSITION.to_string())
                .and_then(|v| v.as_object_mut())
                .ok_or_else(|| {
                    FrameworkError::Sdk("permissive token JSON missing slot 0".into())
                })?;
            let distribution_rules = token_slot
                .get_mut("distributionRules")
                .and_then(|v| v.as_object_mut())
                .ok_or_else(|| {
                    FrameworkError::Sdk("token slot missing distributionRules".into())
                })?;

            let mut distributions_json = serde_json::Map::new();
            for (ts, recipients) in distribution.distributions {
                let mut by_recipient = serde_json::Map::new();
                for (id, amount) in recipients {
                    by_recipient.insert(bs58::encode(id.to_buffer()).into_string(), json!(amount));
                }
                distributions_json.insert(ts.to_string(), serde_json::Value::Object(by_recipient));
            }

            distribution_rules.insert(
                "preProgrammedDistribution".into(),
                json!({
                    "$formatVersion": "0",
                    "distributions": distributions_json,
                }),
            );

            let contract_id =
                register_token_contract_via_sdk(setup_guard.base.ctx, &owner, json).await?;

            Ok(TokenSetup {
                setup_guard,
                owner,
                contract_id,
                token_position: DEFAULT_TOKEN_POSITION,
            })
        })
        .await
    }
}

// ---------------------------------------------------------------------------
// 15b. setup_with_token_perpetual_distribution
// ---------------------------------------------------------------------------

/// Single-identity TK setup with a live perpetual distribution rule
/// (TK-002). The owner receives `amount_per_interval` tokens every
/// `interval_blocks` of platform block height; recipient is pinned
/// to `ContractOwner`, distribution function is
/// `FixedAmount { amount }`.
///
/// Tests must wait for at least one interval boundary to pass before
/// issuing `token_claim` with `TokenDistributionType::Perpetual` —
/// platform-block-time is ~3 s on testnet so a 5-block interval
/// implies ~15 s wall-clock plus headroom.
///
/// Only BlockBasedDistribution is wired up; TimeBased / EpochBased
/// would need their own per-network minimum interval handling and
/// aren't on the TK-002 path.
///
/// Non-async sync wrapper so `#[track_caller]` captures the test
/// file before the async state machine is created.
#[track_caller]
pub fn setup_with_token_perpetual_distribution(
    ctx: &E2eContext,
    owner_funding: dpp::fee::Credits,
    distribution: PerpetualDistribution,
) -> impl std::future::Future<Output = FrameworkResult<TokenSetup>> + '_ {
    let site_label = super::label_from_file(std::panic::Location::caller().file());
    async move {
        let _ = ctx; // ctx unused; kept for API compat.
        let existing = super::funding_ledger::CURRENT_TEST_LABEL
            .try_with(|o| o.clone())
            .ok()
            .flatten();
        let label = existing.or(site_label);
        super::funding_ledger::maybe_with_test_label(label, async move {
            let setup_guard = super::setup_with_n_identities_with_step_timeout(
                1,
                owner_funding,
                TK_SETUP_WAIT_TIMEOUT,
            )
            .await?;
            let owner = setup_guard.identities[0].clone_for_token_setup();

            let json = permissive_owner_token_contract_with_perpetual_distribution_json(
                owner.id,
                DEFAULT_TOKEN_POSITION,
                DEFAULT_MAX_SUPPLY,
                &distribution,
            );
            let contract_id =
                register_token_contract_via_sdk(setup_guard.base.ctx, &owner, json).await?;

            Ok(TokenSetup {
                setup_guard,
                owner,
                contract_id,
                token_position: DEFAULT_TOKEN_POSITION,
            })
        })
        .await
    }
}

/// Sibling of [`permissive_owner_token_contract_json`] that injects a
/// BlockBased perpetual-distribution rule under
/// `tokens["0"].distributionRules.perpetualDistribution`. The rest of
/// the contract envelope is identical to the permissive
/// owner-only baseline (8 decimals, owner-only ChangeControlRules,
/// `mintingAllowChoosingDestination = true`, no pre-programmed
/// schedule) — the perpetual node is the only deviation.
///
/// Schema mirrors the round-trip example in
/// `rs-dpp/src/data_contract/conversion/json/mod.rs`:
/// `{ "distributionType": { "BlockBasedDistribution": { "interval", "function": { "FixedAmount": { "amount" } } } }, "distributionRecipient": {"$type": "contractOwner"} }`.
pub fn permissive_owner_token_contract_with_perpetual_distribution_json(
    owner_id: Identifier,
    position: u16,
    supply: TokenAmount,
    distribution: &PerpetualDistribution,
) -> serde_json::Value {
    let mut json = permissive_owner_token_contract_json(owner_id, position, supply);
    let token_slot = json
        .get_mut(position.to_string())
        .and_then(|v| v.as_object_mut())
        .expect("permissive token JSON missing slot just inserted");
    let distribution_rules = token_slot
        .get_mut("distributionRules")
        .and_then(|v| v.as_object_mut())
        .expect("permissive token JSON missing distributionRules");

    distribution_rules.insert(
        "perpetualDistribution".into(),
        json!({
            "$formatVersion": "0",
            "distributionType": {
                "BlockBasedDistribution": {
                    "interval": distribution.interval_blocks,
                    "function": {
                        "FixedAmount": { "amount": distribution.amount_per_interval },
                    },
                },
            },
            "distributionRecipient": {"$type": "contractOwner"},
        }),
    );

    json
}

// ---------------------------------------------------------------------------
// 16. mint_to — owner-mints-to-recipient shortcut
// ---------------------------------------------------------------------------

/// Owner mints `amount` to `recipient` via
/// [`Sdk::token_mint`]. Resolves only after the proof confirms the
/// new balance.
///
/// The owner signs with [`RegisteredIdentity::critical_key`]
/// (AUTHENTICATION + CRITICAL). `TokenBaseTransition` accepts only
/// `SecurityLevel::CRITICAL`; HIGH yields
/// `InvalidSignaturePublicKeySecurityLevelError`.
pub async fn mint_to(
    ctx: &E2eContext,
    contract_id: Identifier,
    position: TokenContractPosition,
    amount: TokenAmount,
    recipient: &RegisteredIdentity,
    owner_signer: &RegisteredIdentity,
) -> FrameworkResult<()> {
    let data_contract = DataContract::fetch(ctx.sdk(), contract_id)
        .await
        .map_err(|err| FrameworkError::Sdk(format!("fetch data contract: {err}")))?
        .ok_or_else(|| FrameworkError::Sdk(format!("contract {contract_id} not found on chain")))?;

    // Snapshot recipient's pre-mint balance and contract-wide supply
    // so the post-broadcast wait gates can pin exact targets. Required
    // because sibling TK cases (TK-006/007/008) read supply or freeze
    // state immediately after `mint_to` returns and would otherwise
    // race the DAPI replication lag — the SDK's `broadcast_and_wait`
    // settles on whichever node served the broadcast, but the next
    // read may round-robin onto a lagging replica (Marvin TK-006/007/008
    // forensics, v30 run).
    let pre_balance = token_balance_raw(ctx.sdk(), recipient.id, contract_id, position).await?;
    let pre_supply = token_supply_raw(ctx.sdk(), contract_id, position).await?;

    let builder =
        TokenMintTransitionBuilder::new(Arc::new(data_contract), position, owner_signer.id, amount)
            .issued_to_identity_id(recipient.id);

    ctx.sdk()
        .token_mint(
            builder,
            &owner_signer.critical_key,
            owner_signer.signer.as_ref(),
        )
        .await
        .map_err(|err| FrameworkError::Sdk(format!("token_mint: {err}")))?;

    // Post-broadcast wait gates. Saturating-add keeps targets sane on
    // pathological mint values that would overflow.
    let balance_target = pre_balance.saturating_add(amount);
    let supply_target = pre_supply.saturating_add(amount);

    // Gate #1: recipient's chain-side balance reflects the mint.
    wait_for_token_balance(
        ctx,
        recipient.id,
        contract_id,
        position,
        balance_target,
        MINT_POST_BROADCAST_WAIT,
    )
    .await?;

    // Gate #2: contract-wide supply reflects the mint. The supply
    // query (`TotalSingleTokenBalance::fetch`) is served by a
    // different proof path than the per-identity balance and may lag
    // it across replicas; TK-006 reads supply directly after this
    // helper returns and was the failing call site without this gate.
    // Streak-gated for the same round-robin-replica reason as
    // `wait_for_token_balance`: a single supply hit only proves one
    // replica caught up, and TK-006's follow-up read can hit a laggard.
    let description =
        format!("token supply >= {supply_target} (contract={contract_id} position={position})");
    super::wait::wait_for_token_predicate(
        &description,
        || async {
            match token_supply_raw(ctx.sdk(), contract_id, position).await {
                Ok(current) if current >= supply_target => Ok(Some(current)),
                Ok(current) => {
                    tracing::debug!(
                        target: "platform_wallet::e2e::tokens",
                        ?contract_id,
                        position,
                        current,
                        expected = supply_target,
                        "token supply below post-mint target; retrying"
                    );
                    Ok(None)
                }
                Err(err) => Err(err),
            }
        },
        super::wait::CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        MINT_POST_BROADCAST_WAIT,
    )
    .await?;

    Ok(())
}

/// Post-broadcast replication-lag budget for [`mint_to`]. The SDK
/// itself awaits a proof on whichever DAPI replica served the
/// broadcast — this gate is purely for the cross-replica catch-up.
const MINT_POST_BROADCAST_WAIT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// 17. wait_for_token_balance — poll-until-target
// ---------------------------------------------------------------------------

/// Poll [`token_balance_of`] until the chain-side balance reaches
/// `expected` on [`CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES`] back-to-back
/// fetches, then return the observed value. Mirrors PA's `wait_for_balance`
/// shape.
///
/// The streak gate is load-bearing, not cosmetic: the SDK round-robins
/// across DAPI replicas, so a single `current >= expected` hit only proves
/// the value is visible on whichever node answered — the caller's next fetch
/// (or its next state transition) can land on a still-lagging sibling and
/// read a stale balance. Requiring two consecutive distinct-replica hits is
/// the same defense the address/identity/contract waiters use (see
/// `wait.rs`'s `*_chain_confirmed_n` family) and that TK-010/TK-011 needed.
pub async fn wait_for_token_balance(
    ctx: &E2eContext,
    identity_id: Identifier,
    contract_id: Identifier,
    position: TokenContractPosition,
    expected: TokenAmount,
    timeout: Duration,
) -> FrameworkResult<TokenAmount> {
    let description =
        format!("token balance >= {expected} (identity={identity_id} contract={contract_id} position={position})");
    super::wait::wait_for_token_predicate(
        &description,
        || async {
            match token_balance_raw(ctx.sdk(), identity_id, contract_id, position).await {
                Ok(current) if current >= expected => Ok(Some(current)),
                Ok(current) => {
                    tracing::debug!(
                        target: "platform_wallet::e2e::tokens",
                        ?identity_id,
                        ?contract_id,
                        position,
                        current,
                        expected,
                        "token balance below target"
                    );
                    Ok(None)
                }
                Err(err) => Err(err),
            }
        },
        super::wait::CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        timeout,
    )
    .await
}

/// Poll [`token_supply_raw`] until the chain-side total supply equals
/// `expected` on [`CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES`] back-to-back
/// fetches, then return the observed value. Supply-side companion to
/// [`wait_for_token_balance`].
///
/// Gated on exact equality, not `>=` like the balance waiter: a burn
/// (destroy / destroy-frozen) DROPS total supply, so the stale pre-burn
/// value a lagging replica still serves is *larger* than the target — a
/// `>=` gate would clear immediately on it and defeat the purpose. Exact
/// match is correct in both directions (a mint raises supply to an exact
/// figure too) and is what a deterministic mint/burn amount lets us pin.
/// The streak gate is the same round-robin defense
/// [`wait_for_token_balance`] documents.
pub async fn wait_for_token_supply(
    ctx: &E2eContext,
    contract_id: Identifier,
    position: TokenContractPosition,
    expected: TokenAmount,
    timeout: Duration,
) -> FrameworkResult<TokenAmount> {
    let description =
        format!("token supply == {expected} (contract={contract_id} position={position})");
    super::wait::wait_for_token_predicate(
        &description,
        || async {
            match token_supply_raw(ctx.sdk(), contract_id, position).await {
                Ok(current) if current == expected => Ok(Some(current)),
                Ok(current) => {
                    tracing::debug!(
                        target: "platform_wallet::e2e::tokens",
                        ?contract_id,
                        position,
                        current,
                        expected,
                        "token supply not yet at target"
                    );
                    Ok(None)
                }
                Err(err) => Err(err),
            }
        },
        super::wait::CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        timeout,
    )
    .await
}

// ---------------------------------------------------------------------------
// 19. register_extra_identity
// ---------------------------------------------------------------------------

/// Register a fresh identity on the existing test wallet attached
/// to `setup`, funded with `funding` credits from the bank. Used by
/// TK cases that need a third party past the helpers' baseline
/// (e.g. an unauthorised-mint variant).
///
/// Hot-path note: this helper calls
/// [`TestWallet::sync_balances`] after every single registration to
/// keep the funding-address `(balance, nonce)` cache consistent.
/// Calling this in a tight loop is `O(n)` full-wallet syncs — if a
/// test ever needs to register many identities post-setup, batch the
/// registrations and call `sync_balances` once at the end instead of
/// reusing this helper per iteration.
pub async fn register_extra_identity(
    ctx: &E2eContext,
    setup: &mut TokenSetup,
    funding: dpp::fee::Credits,
) -> FrameworkResult<RegisteredIdentity> {
    use super::wait::wait_for_balance;

    let test_wallet = &setup.setup_guard.base.test_wallet;

    // Allocate the next DIP-9 slot above whatever `setup_with_n_identities`
    // already consumed. Slot collisions would surface at registration.
    let next_index = setup.setup_guard.identities.len() as u32;

    let funding_addr = test_wallet.next_unused_address().await?;
    ctx.bank().fund_address(&funding_addr, funding).await?;
    wait_for_balance(test_wallet, &funding_addr, funding, Duration::from_secs(60)).await?;

    let registered = test_wallet
        .register_identity_from_addresses(funding_addr, funding, next_index)
        .await?;

    // Keep wallet caches consistent — `register_from_addresses`
    // doesn't refresh per-address balance/nonce on its own.
    test_wallet.sync_balances().await?;

    setup.setup_guard.identities.push(registered);
    Ok(setup
        .setup_guard
        .identities
        .last()
        .expect("just-pushed identity")
        .clone_for_token_setup())
}

// ---------------------------------------------------------------------------
// 2-6. Typed read-side accessors
// ---------------------------------------------------------------------------

/// Token balance for `identity_id` on `(contract_id, position)`.
pub async fn token_balance_of(
    ctx: &E2eContext,
    contract_id: Identifier,
    position: TokenContractPosition,
    identity_id: Identifier,
) -> FrameworkResult<TokenAmount> {
    token_balance_raw(ctx.sdk(), identity_id, contract_id, position).await
}

/// Total supply for `(contract_id, position)`.
pub async fn token_supply_of(
    ctx: &E2eContext,
    contract_id: Identifier,
    position: TokenContractPosition,
) -> FrameworkResult<TokenAmount> {
    token_supply_raw(ctx.sdk(), contract_id, position).await
}

/// Paused flag for `(contract_id, position)`.
pub async fn token_is_paused_of(
    ctx: &E2eContext,
    contract_id: Identifier,
    position: TokenContractPosition,
) -> FrameworkResult<bool> {
    token_is_paused_raw(ctx.sdk(), contract_id, position).await
}

/// Active pricing schedule for `(contract_id, position)`.
pub async fn token_pricing_of(
    ctx: &E2eContext,
    contract_id: Identifier,
    position: TokenContractPosition,
) -> FrameworkResult<Option<TokenPricingSchedule>> {
    token_pricing_raw(ctx.sdk(), contract_id, position).await
}

/// Frozen-balance accessor — returns the identity's full token
/// balance when `IdentityTokenInfo.frozen` is `true`, else `0`.
/// See module-level note on the bool-vs-balance framing.
pub async fn token_frozen_balance_of(
    ctx: &E2eContext,
    contract_id: Identifier,
    position: TokenContractPosition,
    identity_id: Identifier,
) -> FrameworkResult<TokenAmount> {
    token_frozen_balance_of_raw(ctx.sdk(), identity_id, contract_id, position).await
}

// ---------------------------------------------------------------------------
// 7-11. Raw-id variants (lower-level, accept (contract_id, position) as 32-byte ids)
// ---------------------------------------------------------------------------

/// Lower-level [`token_balance_of`] — accepts the `Sdk` plus raw
/// identifiers so cross-contract reads don't need a fixture.
pub async fn token_balance_raw(
    sdk: &Sdk,
    identity_id: Identifier,
    contract_id: Identifier,
    position: TokenContractPosition,
) -> FrameworkResult<TokenAmount> {
    let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), position));

    let query = IdentityTokenBalancesQuery {
        identity_id,
        token_ids: vec![token_id],
    };

    let balances: dash_sdk::platform::tokens::identity_token_balances::IdentityTokenBalances =
        TokenAmount::fetch_many(sdk, query)
            .await
            .map_err(|err| FrameworkError::Sdk(format!("fetch token balance: {err}")))?;

    Ok(balances.0.get(&token_id).copied().flatten().unwrap_or(0))
}

/// Lower-level [`token_supply_of`].
pub async fn token_supply_raw(
    sdk: &Sdk,
    contract_id: Identifier,
    position: TokenContractPosition,
) -> FrameworkResult<TokenAmount> {
    let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), position));

    let total = TotalSingleTokenBalance::fetch(sdk, token_id)
        .await
        .map_err(|err| FrameworkError::Sdk(format!("fetch token supply: {err}")))?
        .ok_or_else(|| FrameworkError::Sdk(format!("token supply not found for {token_id}")))?;

    // SignedTokenAmount is i64; supplies are non-negative on a healthy
    // chain. Clamp negatives to 0 so a corrupted state surfaces as a
    // mismatched assertion instead of a panic.
    Ok(total.token_supply.max(0) as TokenAmount)
}

/// Lower-level [`token_is_paused_of`].
pub async fn token_is_paused_raw(
    sdk: &Sdk,
    contract_id: Identifier,
    position: TokenContractPosition,
) -> FrameworkResult<bool> {
    use dpp::tokens::status::TokenStatus;

    let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), position));

    let statuses = TokenStatus::fetch_many(sdk, vec![token_id])
        .await
        .map_err(|err| FrameworkError::Sdk(format!("fetch token status: {err}")))?;

    Ok(statuses
        .get(&token_id)
        .and_then(|s| s.as_ref())
        .map(|s| s.paused())
        .unwrap_or(false))
}

/// Lower-level [`token_pricing_of`].
pub async fn token_pricing_raw(
    sdk: &Sdk,
    contract_id: Identifier,
    position: TokenContractPosition,
) -> FrameworkResult<Option<TokenPricingSchedule>> {
    let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), position));

    let ids: Vec<Identifier> = vec![token_id];
    let prices: dash_sdk::query_types::TokenDirectPurchasePrices =
        TokenPricingSchedule::fetch_many(sdk, ids.as_slice())
            .await
            .map_err(|err| FrameworkError::Sdk(format!("fetch token pricing: {err}")))?;

    Ok(prices.get(&token_id).cloned().flatten())
}

/// Lower-level [`token_frozen_balance_of`].
///
/// First reads `IdentityTokenInfo` to learn whether the identity is
/// frozen for the given token; only when frozen does it issue the
/// follow-up balance fetch. Returns `0` for an unfrozen identity to
/// keep callers' arithmetic free of `Option` plumbing.
pub async fn token_frozen_balance_of_raw(
    sdk: &Sdk,
    identity_id: Identifier,
    contract_id: Identifier,
    position: TokenContractPosition,
) -> FrameworkResult<TokenAmount> {
    use dpp::tokens::info::IdentityTokenInfo;

    let token_id = Identifier::from(calculate_token_id(contract_id.as_bytes(), position));

    let infos: dash_sdk::query_types::token_info::IdentityTokenInfos =
        IdentityTokenInfo::fetch_many(
            sdk,
            IdentityTokenInfosQuery {
                identity_id,
                token_ids: vec![token_id],
            },
        )
        .await
        .map_err(|err| FrameworkError::Sdk(format!("fetch token info: {err}")))?;

    let frozen = infos
        .0
        .get(&token_id)
        .and_then(|i: &Option<IdentityTokenInfo>| i.as_ref())
        .map(|i: &IdentityTokenInfo| i.frozen())
        .unwrap_or(false);

    if frozen {
        token_balance_raw(sdk, identity_id, contract_id, position).await
    } else {
        Ok(0)
    }
}

// ---------------------------------------------------------------------------
// Helpers internal to this module.
// ---------------------------------------------------------------------------

/// `RegisteredIdentity` is not `Clone` upstream (the
/// `SeedBackedIdentitySigner` is `Arc`-shared, so cloning the
/// owning struct is cheap if we wire it ourselves). The TK setup
/// helpers need to surface a copy of the owner / peer identity in
/// their return types while keeping the original inside
/// [`MultiIdentitySetupGuard::identities`] for teardown bookkeeping.
trait CloneForTokenSetup {
    fn clone_for_token_setup(&self) -> Self;
}

impl CloneForTokenSetup for RegisteredIdentity {
    fn clone_for_token_setup(&self) -> Self {
        RegisteredIdentity {
            id: self.id,
            master_key: self.master_key.clone(),
            high_key: self.high_key.clone(),
            transfer_key: self.transfer_key.clone(),
            critical_key: self.critical_key.clone(),
            signer: Arc::clone(&self.signer),
            identity_index: self.identity_index,
            funding: self.funding,
        }
    }
}
