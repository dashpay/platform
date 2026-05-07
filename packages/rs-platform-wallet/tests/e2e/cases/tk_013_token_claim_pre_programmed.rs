//! TK-013 — Token claim from pre-programmed distribution.
//!
//! Owner deploys a token with a pre-programmed distribution whose
//! epoch zero is scheduled 5 minutes ahead of wall time, then calls
//! `token_claim` with `TokenDistributionType::PreProgrammed`. Asserts
//! the owner's balance increases by exactly the configured payout.
//! Mirrors the wallet's `token_claim_with_signer` chain path — the
//! wallet helper just forwards to `Sdk::token_claim`, which is what
//! this test drives directly to keep the framework surface flat (cf.
//! `mint_to` in `framework/tokens.rs`).
//!
//! Pre-programmed (not perpetual). Perpetual is TK-002, gated behind
//! `slow-tests` because it needs live block-time. The pre-programmed
//! variant uses a near-future epoch so contract registration clears
//! block-time validation; the claim is issued after the epoch elapses.
//!
//! Gated behind `#[ignore]` — same operator-env reasoning as the
//! transfer case (`PLATFORM_WALLET_E2E_BANK_MNEMONIC` + live testnet
//! DAPI access).

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::data_contract::DataContract;
use dpp::prelude::{Identifier, TimestampMillis};

use dash_sdk::platform::tokens::builders::claim::TokenClaimTransitionBuilder;
use dash_sdk::platform::tokens::transitions::ClaimResult;
use dash_sdk::platform::Fetch;

use crate::framework::prelude::*;
use crate::framework::setup_with_n_identities;
use crate::framework::tokens::{
    register_token_contract_via_sdk, token_balance_of, DEFAULT_BASE_SUPPLY, DEFAULT_DECIMALS,
    DEFAULT_MAX_SUPPLY, DEFAULT_TOKEN_POSITION,
};

/// Per-epoch payout the schedule credits to the owner. Small enough
/// that an over-shoot regression (multiple credits, double-mint)
/// surfaces as an unmistakable balance mismatch.
const PAYOUT: TokenAmount = 100;

/// Per-identity bank funding for the setup helper. Mirrors `DEFAULT_TK_FUNDING`
/// — sized to cover the contract-deploy fee floor (~30 B credits).
const FUNDING: dpp::fee::Credits = 35_000_100_000;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
#[ignore = "requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access; run with `cargo test -- --ignored`"]
async fn tk_013_token_claim_from_pre_programmed_distribution() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Register the owner first so its identifier is known before we
    // bake the distribution schedule into the contract JSON. The
    // helper `setup_with_token_pre_programmed_distribution` takes the
    // schedule by value and registers + deploys in a single call — it
    // can't see the owner id ahead of time, so for the
    // owner-claims-its-own-payout shape (TK-013) we drive the lower
    // primitives directly.
    let setup_guard = setup_with_n_identities(1, FUNDING)
        .await
        .expect("register owner identity");
    let ctx = setup_guard.base.ctx;
    let owner = &setup_guard.identities[0];
    let owner_id = owner.id;

    // Park epoch zero 5 minutes in the future so the contract
    // registration passes block-time validation (the platform rejects
    // any pre-programmed distribution whose epoch is already in the
    // past at broadcast time). 300 s gives enough runway to clear
    // the broadcast-plus-block-inclusion window on testnet without
    // turning the test into a 10-minute wait.
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is past UNIX_EPOCH")
        .as_millis() as TimestampMillis;
    let epoch_zero_at = now_ms + Duration::from_secs(300).as_millis() as u64;

    let contract_json = build_pre_programmed_token_json(owner_id, epoch_zero_at, PAYOUT);
    let contract_id = register_token_contract_via_sdk(ctx, owner, contract_json)
        .await
        .expect("register pre-programmed token contract");

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
        .token_claim(builder, &owner.high_key, owner.signer.as_ref())
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
        .token_claim(retry_builder, &owner.high_key, owner.signer.as_ref())
        .await;
    let err_text = match retry_result {
        Ok(_) => panic!(
            "second claim against the same pre-programmed epoch must fail \
             — regression: payout was credited twice"
        ),
        Err(err) => format!("{err}").to_lowercase(),
    };
    assert!(
        err_text.contains("already claimed")
            || err_text.contains("no claimable amount")
            || err_text.contains("nothing to claim")
            || err_text.contains("already paid")
            || err_text.contains("alreadypaid"),
        "second-claim error must reference the 'already claimed' / 'no claimable amount' \
         class (observed: {err_text})"
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
        "authorizedToMakeChange": "ContractOwner",
        "adminActionTakers": "ContractOwner",
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
        "mainControlGroupCanBeModified": "ContractOwner",
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
