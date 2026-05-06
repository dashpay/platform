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
use std::time::{Duration, Instant};

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
use super::{setup_with_n_identities, FrameworkError, FrameworkResult, MultiIdentitySetupGuard};

/// Default TK-NNN token slot. The permissive owner-only contract
/// always deploys a single token at position `0`.
pub const DEFAULT_TOKEN_POSITION: TokenContractPosition = 0;

/// Default TK-NNN base supply (zero — owner mints in-test).
pub const DEFAULT_BASE_SUPPLY: TokenAmount = 0;

/// Default TK-NNN max supply (`1e15`, mirrors DET).
pub const DEFAULT_MAX_SUPPLY: TokenAmount = 1_000_000_000_000_000;

/// Default TK-NNN decimals (8, mirrors DET).
pub const DEFAULT_DECIMALS: u8 = 8;

/// Default per-identity funding for TK setup helpers — covers the
/// token contract-create fee floor (~20 B credits for permissive
/// owner-only contracts, ~30 B for the pre-programmed-distribution
/// path) plus a few follow-up state transitions with headroom. The
/// previous 1 B value undershot the chain-side floor and made every
/// TK case fail at setup with `Insufficient identity ... balance
/// 1000000000 required 20000100000`.
pub const DEFAULT_TK_FUNDING: dpp::fee::Credits = 35_000_100_000;

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
        "authorizedToMakeChange": "ContractOwner",
        "adminActionTakers": "ContractOwner",
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
        "mainControlGroupCanBeModified": "ContractOwner",
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
pub async fn setup_with_token_contract(
    ctx: &E2eContext,
    owner_funding: dpp::fee::Credits,
) -> FrameworkResult<TokenSetup> {
    let _ = ctx;
    let setup_guard = setup_with_n_identities(1, owner_funding).await?;
    let owner = setup_guard
        .identities
        .first()
        .ok_or_else(|| {
            FrameworkError::Wallet("setup_with_n_identities returned empty identities vec".into())
        })?
        .clone_for_token_setup();

    let json =
        permissive_owner_token_contract_json(owner.id, DEFAULT_TOKEN_POSITION, DEFAULT_MAX_SUPPLY);
    let contract_id = register_token_contract_via_sdk(setup_guard.base.ctx, &owner, json).await?;

    Ok(TokenSetup {
        setup_guard,
        owner,
        contract_id,
        token_position: DEFAULT_TOKEN_POSITION,
    })
}

// ---------------------------------------------------------------------------
// 13. setup_with_token_and_two_identities
// ---------------------------------------------------------------------------

/// Two-identity TK setup. Identity #0 owns the contract, identity
/// #1 is a peer for transfer / freeze / purchase scenarios.
pub async fn setup_with_token_and_two_identities(
    ctx: &E2eContext,
    funding_per: dpp::fee::Credits,
) -> FrameworkResult<TokenTwoIdentitiesSetup> {
    let _ = ctx;
    let setup_guard = setup_with_n_identities(2, funding_per).await?;
    let owner = setup_guard.identities[0].clone_for_token_setup();
    let peer = setup_guard.identities[1].clone_for_token_setup();

    let json =
        permissive_owner_token_contract_json(owner.id, DEFAULT_TOKEN_POSITION, DEFAULT_MAX_SUPPLY);
    let contract_id = register_token_contract_via_sdk(setup_guard.base.ctx, &owner, json).await?;

    Ok(TokenTwoIdentitiesSetup {
        setup: TokenSetup {
            setup_guard,
            owner,
            contract_id,
            token_position: DEFAULT_TOKEN_POSITION,
        },
        peer,
    })
}

// ---------------------------------------------------------------------------
// 14. setup_with_token_and_three_identities
// ---------------------------------------------------------------------------

/// Three-identity TK setup — owner plus two peers (TK-014 group
/// co-sign happy path).
pub async fn setup_with_token_and_three_identities(
    ctx: &E2eContext,
    funding_per: dpp::fee::Credits,
) -> FrameworkResult<TokenThreeIdentitiesSetup> {
    let _ = ctx;
    let setup_guard = setup_with_n_identities(3, funding_per).await?;
    let owner = setup_guard.identities[0].clone_for_token_setup();
    let peers = [
        setup_guard.identities[1].clone_for_token_setup(),
        setup_guard.identities[2].clone_for_token_setup(),
    ];

    let json =
        permissive_owner_token_contract_json(owner.id, DEFAULT_TOKEN_POSITION, DEFAULT_MAX_SUPPLY);
    let contract_id = register_token_contract_via_sdk(setup_guard.base.ctx, &owner, json).await?;

    Ok(TokenThreeIdentitiesSetup {
        setup: TokenSetup {
            setup_guard,
            owner,
            contract_id,
            token_position: DEFAULT_TOKEN_POSITION,
        },
        peers,
    })
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
pub async fn setup_with_token_pre_programmed_distribution(
    ctx: &E2eContext,
    owner_funding: dpp::fee::Credits,
    distribution: PreProgrammedDistribution,
) -> FrameworkResult<TokenSetup> {
    let _ = ctx;
    let setup_guard = setup_with_n_identities(1, owner_funding).await?;
    let owner = setup_guard.identities[0].clone_for_token_setup();

    let mut json =
        permissive_owner_token_contract_json(owner.id, DEFAULT_TOKEN_POSITION, DEFAULT_MAX_SUPPLY);
    let token_slot = json
        .get_mut(DEFAULT_TOKEN_POSITION.to_string())
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| FrameworkError::Sdk("permissive token JSON missing slot 0".into()))?;
    let distribution_rules = token_slot
        .get_mut("distributionRules")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| FrameworkError::Sdk("token slot missing distributionRules".into()))?;

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

    let contract_id = register_token_contract_via_sdk(setup_guard.base.ctx, &owner, json).await?;

    Ok(TokenSetup {
        setup_guard,
        owner,
        contract_id,
        token_position: DEFAULT_TOKEN_POSITION,
    })
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

    Ok(())
}

// ---------------------------------------------------------------------------
// 17. wait_for_token_balance — poll-until-target
// ---------------------------------------------------------------------------

/// Poll [`token_balance_of`] every
/// [`super::wait::DEFAULT_POLL_INTERVAL`] until the cached balance
/// reaches `expected`, then return the observed value. Mirrors PA's
/// `wait_for_balance` shape.
pub async fn wait_for_token_balance(
    ctx: &E2eContext,
    identity_id: Identifier,
    contract_id: Identifier,
    position: TokenContractPosition,
    expected: TokenAmount,
    timeout: Duration,
) -> FrameworkResult<TokenAmount> {
    let deadline = Instant::now() + timeout;
    loop {
        match token_balance_raw(ctx.sdk(), identity_id, contract_id, position).await {
            Ok(current) if current >= expected => return Ok(current),
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
            }
            Err(err) => {
                tracing::debug!(
                    target: "platform_wallet::e2e::tokens",
                    ?identity_id,
                    error = %err,
                    "token balance fetch failed; retrying"
                );
            }
        }

        if Instant::now() >= deadline {
            return Err(FrameworkError::Cleanup(format!(
                "wait_for_token_balance timed out after {timeout:?} \
                 (identity={identity_id} contract={contract_id} position={position} expected={expected})"
            )));
        }
        tokio::time::sleep(super::wait::DEFAULT_POLL_INTERVAL).await;
    }
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
