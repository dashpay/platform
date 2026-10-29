//! TK-013 — Token claim from pre-programmed distribution.
//!
//! Owner deploys a token with a pre-programmed distribution whose
//! epoch zero is scheduled a short window ahead of wall time, waits
//! for that window to elapse, then calls `token_claim` with
//! `TokenDistributionType::PreProgrammed`. Asserts the owner's
//! balance increases by exactly the configured payout. Mirrors the
//! wallet's `token_claim_with_signer` chain path — the wallet helper
//! just forwards to `Sdk::token_claim`, which is what this test
//! drives directly to keep the framework surface flat (cf. `mint_to`
//! in `framework/tokens.rs`).
//!
//! Pre-programmed (not perpetual). Perpetual is TK-002, gated behind
//! `slow-tests` because it needs live block-time. The pre-programmed
//! variant pins a *near-future* epoch so contract registration clears
//! the `< block_info.time_ms` block-time validation gate, then sleeps
//! until the timestamp has elapsed so the claim transformer's
//! `<= block_info.time_ms` filter admits it.
//!
//! Gated behind the `e2e` cargo feature — same operator-env reasoning as the
//! transfer case (`PLATFORM_WALLET_E2E_BANK_MNEMONIC` + live testnet
//! DAPI access).

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use dpp::balances::credits::TokenAmount;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::data_contract::DataContract;
use dpp::prelude::{Identifier, TimestampMillis};

use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::platform::tokens::builders::claim::TokenClaimTransitionBuilder;
use dash_sdk::platform::tokens::transitions::ClaimResult;
use dash_sdk::platform::Fetch;

use crate::framework::prelude::*;
use crate::framework::setup_with_per_identity_funding;
use crate::framework::tokens::{
    register_token_contract_via_sdk, token_balance_of, wait_for_token_balance, DEFAULT_BASE_SUPPLY,
    DEFAULT_DECIMALS, DEFAULT_MAX_SUPPLY, DEFAULT_TOKEN_POSITION, TK_OWNER_FUNDING_DISTRIBUTION,
    TK_SETUP_WAIT_TIMEOUT,
};

/// Per-epoch payout the schedule credits to the owner. Small enough
/// that an over-shoot regression (multiple credits, double-mint)
/// surfaces as an unmistakable balance mismatch.
const PAYOUT: TokenAmount = 100;

/// Replica-lag budget for the post-claim balance gate. Same 60 s the
/// sibling token tests (TK-001/004/007/008/009/011) use.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_013_token_claim_from_pre_programmed_distribution() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    {
        let floor_ctx = E2eContext::init().await.expect("init e2e context");
        if floor_ctx.skip_if_bank_floor_unmet("tk_013") {
            return;
        }
    }

    // Register the owner first so its identifier is known before we
    // bake the distribution schedule into the contract JSON. The
    // helper `setup_with_token_pre_programmed_distribution` takes the
    // schedule by value and registers + deploys in a single call — it
    // can't see the owner id ahead of time, so for the
    // owner-claims-its-own-payout shape (TK-013) we drive the lower
    // primitives directly.
    //
    // QA-V40-001 — under `worker_threads = 12` parallel churn on the
    // shared bank wallet, the 35 B-credit funding wait routinely needs
    // more than the 60 s `DEFAULT_SETUP_STEP_TIMEOUT`. Route through the
    // explicit-budget entry point with `TK_SETUP_WAIT_TIMEOUT` (120 s)
    // for headroom on cross-replica replication lag — same pattern the
    // five `setup_with_token_*` helpers and TK-003 already use.
    let setup_guard =
        setup_with_per_identity_funding(&[TK_OWNER_FUNDING_DISTRIBUTION], TK_SETUP_WAIT_TIMEOUT)
            .await
            .expect("register owner identity");
    let ctx = setup_guard.base.ctx;
    let owner = &setup_guard.identities[0];
    let owner_id = owner.id;

    // Two competing chain-side rules force a narrow window for
    // `epoch_zero_at`:
    //   * `data_contract_create` rejects a pre-programmed distribution
    //     whose first timestamp is *strictly less than* the current
    //     block time at broadcast — `PreProgrammedDistributionTimestampInPast`.
    //   * The claim transformer only credits distributions whose
    //     timestamp is `<= block_info.time_ms` at claim time —
    //     anything still in the future yields
    //     `InvalidTokenClaimNoCurrentRewards`.
    // So we park epoch zero a small window ahead of `now_ms` (enough
    // to clear the broadcast + block-inclusion lag for the contract
    // create), then wait wall-clock until the timestamp has elapsed
    // before issuing the claim. 60 s is comfortably above observed
    // testnet inclusion latency without turning the test into a
    // 5-minute hang.
    // QA-V19-001: Wall-clock waiting alone is not sufficient — the
    // platform's `block_info.time_ms` (against which the claim
    // transformer's `<= block_info.time_ms` filter runs) lags
    // wall-clock on testnet by tens of seconds. v18 captured a run
    // where wall_clock had crossed `epoch_zero_at + 15s` yet the
    // chain reported `current_moment` ~75 s behind, still tripping
    // `InvalidTokenClaimNoCurrentRewards`. The fix:
    //   1. Bump `FUTURE_OFFSET` to 240 s so the contract-create
    //      broadcast clears the `>= block_info.time_ms` validator
    //      with comfortable headroom (chain-time can lag wall-clock
    //      by 60–90 s under load and we still need the schedule
    //      timestamp to be strictly in the platform-future).
    //   2. After contract registration, *poll* the platform's latest
    //      `ResponseMetadata.time_ms` (via `ExtendedEpochInfo::
    //      fetch_current_with_metadata`) until that observed value
    //      crosses `epoch_zero_at + POST_EPOCH_CUSHION` — this is
    //      the same `block_info.time_ms` the claim transformer
    //      consults, so once we've seen it advance past the schedule
    //      we know the next claim will admit the distribution.
    // Raised from 240 s → 300 s (QA-V19-002): testnet block-time lag
    // under load can reach 90–120 s; the wider offset keeps the contract
    // creation strictly in the platform-future even on slow nodes.
    const FUTURE_OFFSET: Duration = Duration::from_secs(300);
    /// Cushion past `epoch_zero_at` enforced against the OBSERVED
    /// platform block time (not wall-clock). Once the chain reports
    /// `time_ms >= epoch_zero_at + POST_EPOCH_CUSHION` the next
    /// block's `block_info.time_ms` will satisfy the `<=` filter.
    const POST_EPOCH_CUSHION: Duration = Duration::from_secs(15);
    /// Poll cadence for `ExtendedEpochInfo::fetch_current_with_metadata`.
    const POLL_INTERVAL: Duration = Duration::from_secs(3);
    /// Hard ceiling on the wait so a stuck testnet fails the test fast
    /// rather than hanging the suite. Raised from 420 s → 600 s
    /// (QA-V19-002): testnet block-time timestamps lag wall-clock by
    /// 60–120 s under load; the previous 420 s budget was marginally
    /// too tight (observed overshoot: ~11 s). 600 s accommodates
    /// testnet block-time variability without masking genuine stalls.
    const MAX_WAIT: Duration = Duration::from_secs(600);

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is past UNIX_EPOCH")
        .as_millis() as TimestampMillis;
    let epoch_zero_at = now_ms + FUTURE_OFFSET.as_millis() as u64;

    let contract_json = build_pre_programmed_token_json(owner_id, epoch_zero_at, PAYOUT);
    let contract_id = register_token_contract_via_sdk(ctx, owner, contract_json)
        .await
        .expect("register pre-programmed token contract");

    // Poll platform-side block time until it crosses
    // `epoch_zero_at + cushion`. Querying `ExtendedEpochInfo::
    // fetch_current_with_metadata` returns the platform's latest
    // `ResponseMetadata.time_ms` — the same value the claim
    // transformer evaluates `<= block_info.time_ms` against. Without
    // this poll the test races the chain and rejects with
    // `InvalidTokenClaimNoCurrentRewards`.
    let target_ms = epoch_zero_at + POST_EPOCH_CUSHION.as_millis() as u64;
    let deadline = Instant::now() + MAX_WAIT;
    loop {
        let (_, metadata) = ExtendedEpochInfo::fetch_current_with_metadata(ctx.sdk())
            .await
            .expect("fetch current epoch metadata");
        let observed_ms = metadata.time_ms;
        if observed_ms >= target_ms {
            tracing::info!(
                target: "platform_wallet::e2e::cases::tk_013",
                ?contract_id,
                epoch_zero_at,
                observed_ms,
                target_ms,
                "TK-013 platform block time crossed target — proceeding to claim"
            );
            break;
        }
        if Instant::now() >= deadline {
            panic!(
                "TK-013: platform block time did not catch up to \
                 epoch_zero_at + cushion within {:?} (observed_ms={observed_ms}, \
                 target_ms={target_ms}, delta_ms={})",
                MAX_WAIT,
                target_ms - observed_ms,
            );
        }
        tracing::info!(
            target: "platform_wallet::e2e::cases::tk_013",
            ?contract_id,
            observed_ms,
            target_ms,
            delta_ms = target_ms - observed_ms,
            "TK-013 waiting for platform block time to advance"
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }

    // Snapshot pre-claim balance so the assertion is robust against
    // any historical seed in the contract (there shouldn't be one,
    // but a strict diff is the right shape).
    let balance_before = token_balance_of(ctx, contract_id, DEFAULT_TOKEN_POSITION, owner_id)
        .await
        .expect("pre-claim balance");

    // Build + broadcast the claim. The wallet's
    // `token_claim_with_signer` is a thin forward to
    // `Sdk::token_claim`, so we drive the SDK builder directly here
    // — same chain path, fewer indirections, mirrors the existing
    // `mint_to` framework helper.
    let data_contract = Arc::new(
        DataContract::fetch(ctx.sdk(), contract_id)
            .await
            .expect("fetch token data contract")
            .expect("token data contract present on chain"),
    );
    let builder = TokenClaimTransitionBuilder::new(
        Arc::clone(&data_contract),
        DEFAULT_TOKEN_POSITION,
        owner_id,
        TokenDistributionType::PreProgrammed,
    );
    let claim_result = ctx
        .sdk()
        .token_claim(builder, &owner.critical_key, owner.signer.as_ref())
        .await
        .expect("token_claim broadcast");

    // The proof envelope returns either a Document (history-tracked)
    // or a GroupActionWithDocument (group-gated). For TK-013 the
    // contract is owner-only and the claim is non-group, so we expect
    // a Document — guarding both arms keeps the test sensitive to
    // a result-shape change without depending on it.
    match &claim_result {
        ClaimResult::Document(_) | ClaimResult::GroupActionWithDocument(_, _) => {}
    }

    // Mirror the sibling token tests (TK-001/004/007/008/009/011): gate
    // the post-claim read on the owner's token balance reaching the
    // expected post-claim amount. Reading immediately after the
    // `token_claim` broadcast can hit a DAPI replica that hasn't yet
    // applied the block carrying the payout and return the pre-claim
    // value, racing the credit-delta assertion below.
    wait_for_token_balance(
        ctx,
        owner_id,
        contract_id,
        DEFAULT_TOKEN_POSITION,
        balance_before + PAYOUT,
        STEP_TIMEOUT,
    )
    .await
    .expect("owner balance never reached pre-claim + payout");

    let balance_after = token_balance_of(ctx, contract_id, DEFAULT_TOKEN_POSITION, owner_id)
        .await
        .expect("post-claim balance");

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_013",
        ?contract_id,
        ?owner_id,
        epoch_zero_at,
        balance_before,
        balance_after,
        payout = PAYOUT,
        "TK-013 post-claim balance snapshot"
    );

    assert_eq!(
        balance_after,
        balance_before + PAYOUT,
        "post-claim balance must equal pre-claim + payout (claim from pre-programmed distribution silently fails — balance just doesn't move). \
         observed before={balance_before} after={balance_after} expected_delta={PAYOUT}"
    );

    // Spec § TK-013: a second claim against the same epoch must fail
    // with a typed "already claimed" / "no claimable amount" error.
    // A regression that silently lets the same epoch be claimed
    // multiple times — exactly the silent-on-failure class of bug
    // the spec rationale calls out — would otherwise pass undetected.
    let retry_builder = TokenClaimTransitionBuilder::new(
        data_contract,
        DEFAULT_TOKEN_POSITION,
        owner_id,
        TokenDistributionType::PreProgrammed,
    );
    let retry_result = ctx
        .sdk()
        .token_claim(retry_builder, &owner.critical_key, owner.signer.as_ref())
        .await;
    let retry_err = match retry_result {
        Ok(_) => panic!(
            "second claim against the same pre-programmed epoch must fail \
             — regression: payout was credited twice"
        ),
        Err(err) => err,
    };

    // Typed-variant match: Drive raises
    // `StateError::InvalidTokenClaimNoCurrentRewards` when the same
    // pre-programmed epoch is claimed twice. We unwrap the SDK error
    // to its consensus payload via the same shape `is_instant_lock_proof_invalid`
    // uses (`StateTransitionBroadcastError.cause` /
    // `Protocol(ConsensusError(...))`) so we don't depend on Display.
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    let consensus_error: Option<&ConsensusError> = match &retry_err {
        dash_sdk::Error::StateTransitionBroadcastError(broadcast_err) => {
            broadcast_err.cause.as_ref()
        }
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(ce)) => Some(ce.as_ref()),
        _ => None,
    };
    assert!(
        matches!(
            consensus_error,
            Some(ConsensusError::StateError(
                StateError::InvalidTokenClaimNoCurrentRewards(_),
            )),
        ),
        "second-claim error must be `StateError::InvalidTokenClaimNoCurrentRewards` \
         (observed: {retry_err:?})"
    );

    // Sanity: the failed retry must NOT have credited the owner a
    // second payout.
    let balance_after_retry = token_balance_of(ctx, contract_id, DEFAULT_TOKEN_POSITION, owner_id)
        .await
        .expect("post-retry balance");
    assert_eq!(
        balance_after_retry, balance_after,
        "rejected second claim must not alter the owner balance \
         (pre={balance_after} post={balance_after_retry})"
    );

    setup_guard.teardown().await.expect("teardown");
}

/// Build a permissive owner-only V1 token-contract JSON with a
/// pre-programmed distribution baked in at `epoch_zero_at_ms`
/// granting `payout` to `owner_id`. Self-contained rather than
/// mutating `permissive_owner_token_contract_json` so this case file
/// owns the exact shape it tests against.
fn build_pre_programmed_token_json(
    owner_id: Identifier,
    epoch_zero_at_ms: TimestampMillis,
    payout: TokenAmount,
) -> serde_json::Value {
    use serde_json::json;

    let owner_b58 = bs58::encode(owner_id.to_buffer()).into_string();
    let owner_only = json!({
        "$formatVersion": "0",
        "authorizedToMakeChange": {"$type": "contractOwner"},
        "adminActionTakers": {"$type": "contractOwner"},
        "changingAuthorizedActionTakersToNoOneAllowed": false,
        "changingAdminActionTakersToNoOneAllowed": false,
        "selfChangingAdminActionTakersAllowed": false,
    });

    // `serde_json::json!` requires literal map keys, so build the
    // schedule map manually.
    let mut by_recipient = serde_json::Map::new();
    by_recipient.insert(owner_b58.clone(), json!(payout));
    let mut schedule = serde_json::Map::new();
    schedule.insert(
        epoch_zero_at_ms.to_string(),
        serde_json::Value::Object(by_recipient),
    );

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
        "maxSupply": DEFAULT_MAX_SUPPLY,
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
            "preProgrammedDistribution": {
                "$formatVersion": "0",
                "distributions": serde_json::Value::Object(schedule),
            },
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
        "description": "TK-013 pre-programmed distribution token (rs-platform-wallet e2e).",
        "marketplaceRules": {
            "$formatVersion": "0",
            "tradeMode": "NotTradeable",
            "tradeModeChangeRules": owner_only,
        },
    });

    let mut tokens = serde_json::Map::new();
    tokens.insert(DEFAULT_TOKEN_POSITION.to_string(), token_slot);
    serde_json::Value::Object(tokens)
}
