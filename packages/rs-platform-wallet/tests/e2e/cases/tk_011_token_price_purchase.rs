//! TK-011 — Set price + direct purchase round-trip.
//!
//! Two-identity setup (owner + buyer). Owner mints, sets a single
//! price, buyer purchases — owner+buyer credit and token balances
//! pin the cross-identity money flow.
//!
//! Wave 2 stub: gated behind the `e2e` cargo feature. Wave 4 runs against live testnet.
//!
//! Spec drift note: TEST_SPEC.md asks for a positive `actual_fee` on
//! `SetPriceResult` and `DirectPurchaseResult`, but the bare SDK enums
//! (rs-sdk/src/platform/tokens/transitions/{set_price_for_direct_
//! purchase,direct_purchase}.rs) don't surface a fee field. We assert
//! the fee via credit-balance deltas instead — buyer's decrease must
//! exceed `total_agreed_price`, owner's increase must be at most
//! `total_agreed_price` (a positive seller-side protocol fee shrinks
//! the credit landing in the owner's account).

use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::tokens::builders::purchase::TokenDirectPurchaseTransitionBuilder;
use dash_sdk::platform::tokens::builders::set_price::TokenChangeDirectPurchasePriceTransitionBuilder;
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::data_contract::DataContract;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

use crate::framework::prelude::*;
use crate::framework::tokens::{
    mint_to, setup_with_token_and_two_identities, token_balance_of, token_pricing_of,
    wait_for_token_balance, DEFAULT_TOKEN_POSITION, TK_OWNER_FUNDING_SIMPLE,
    TK_PEER_FUNDING_ACTIVE,
};
use crate::framework::wait::wait_for_token_predicate;

const MINT_AMOUNT: u64 = 1_000;
const PRICE_PER_TOKEN: u64 = 1_000;
const PURCHASE_AMOUNT: u64 = 10;
const TOTAL_AGREED_PRICE: u64 = 10_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_011_set_price_and_direct_purchase_round_trip() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("init e2e context");
    if ctx.skip_if_bank_floor_unmet("tk_011") {
        return;
    }
    let s =
        setup_with_token_and_two_identities(ctx, TK_OWNER_FUNDING_SIMPLE, TK_PEER_FUNDING_ACTIVE)
            .await
            .expect("token + two identities setup");

    let owner = &s.setup.owner;
    let buyer = &s.peer;
    let contract_id = s.setup.contract_id;
    let position = s.setup.token_position;

    // Step 1: owner mints 1_000 tokens to self.
    mint_to(ctx, contract_id, position, MINT_AMOUNT, owner, owner)
        .await
        .expect("owner mint to self");

    // QA-V28-405 — the mint state-transition lands on whichever DAPI
    // node served the broadcast; the immediate `token_balance_of` can
    // round-robin onto a sibling that hasn't applied it yet and read
    // `0` for a freshly-deployed contract. Gate on a 3-success streak
    // of `balance == MINT_AMOUNT` before the assertion.
    let owner_token_pre = wait_for_token_predicate(
        "owner token_balance_of == MINT_AMOUNT (post-mint)",
        || async {
            match token_balance_of(ctx, contract_id, position, owner.id).await {
                Ok(b) if b == MINT_AMOUNT => Ok(Some(b)),
                Ok(_) => Ok(None),
                Err(err) => Err(err),
            }
        },
        3,
        STEP_TIMEOUT,
    )
    .await
    .expect("owner balance must equal the freshly-minted amount on a fresh contract");
    assert_eq!(
        owner_token_pre, MINT_AMOUNT,
        "wait_for_token_predicate returned a non-matching balance ({owner_token_pre})"
    );

    let buyer_token_pre = token_balance_of(ctx, contract_id, position, buyer.id)
        .await
        .expect("buyer token balance pre-purchase");
    assert_eq!(buyer_token_pre, 0, "buyer must start with zero tokens");

    // Pricing must be unset initially.
    let pricing_pre = token_pricing_of(ctx, contract_id, position)
        .await
        .expect("pricing pre-set");
    assert!(
        pricing_pre.is_none(),
        "no pricing schedule should exist before set_price (got {pricing_pre:?})"
    );

    let data_contract = Arc::new(
        DataContract::fetch(ctx.sdk(), contract_id)
            .await
            .expect("fetch contract for set_price")
            .expect("contract on chain"),
    );

    // Step 2: owner sets the pricing schedule to SinglePrice(1_000).
    let set_price_builder = TokenChangeDirectPurchasePriceTransitionBuilder::new(
        data_contract.clone(),
        position,
        owner.id,
    )
    .with_single_price(PRICE_PER_TOKEN);

    ctx.sdk()
        .token_set_price_for_direct_purchase(
            set_price_builder,
            &owner.critical_key,
            owner.signer.as_ref(),
        )
        .await
        .expect("set_price transition");

    let pricing_post = token_pricing_of(ctx, contract_id, position)
        .await
        .expect("pricing post-set");
    match pricing_post {
        Some(TokenPricingSchedule::SinglePrice(p)) => {
            assert_eq!(p, PRICE_PER_TOKEN, "on-chain price must match what we set")
        }
        other => panic!("expected SinglePrice({PRICE_PER_TOKEN}), got {other:?}"),
    }

    // Snapshot credit balances around the purchase. The bare SDK
    // result enums don't expose actual_fee, so we read the deltas
    // directly to verify the spec's two-side credit-flow assertions.
    let buyer_credits_pre = <IdentityBalance as Fetch>::fetch(ctx.sdk(), buyer.id)
        .await
        .expect("buyer credit balance pre-purchase")
        .expect("buyer balance present");
    let owner_credits_pre = <IdentityBalance as Fetch>::fetch(ctx.sdk(), owner.id)
        .await
        .expect("owner credit balance pre-purchase")
        .expect("owner balance present");

    // Step 3: buyer purchases 10 tokens at total_agreed_price=10_000.
    let purchase_builder = TokenDirectPurchaseTransitionBuilder::new(
        data_contract,
        position,
        buyer.id,
        PURCHASE_AMOUNT,
        TOTAL_AGREED_PRICE,
    );
    ctx.sdk()
        .token_purchase(purchase_builder, &buyer.critical_key, buyer.signer.as_ref())
        .await
        .expect("purchase transition");

    // Mirror the sibling token tests (TK-001/004/007/008/009): gate the
    // post-purchase reads on the buyer's token balance reaching the
    // purchased amount. The direct-purchase ST mints new tokens to the
    // buyer; reading immediately can hit a DAPI replica that hasn't
    // applied the block yet and return the pre-purchase value, racing
    // the credit-delta assertions below.
    wait_for_token_balance(
        ctx,
        buyer.id,
        contract_id,
        position,
        buyer_token_pre + PURCHASE_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("buyer balance never reached purchased amount");

    // Step 4: post-purchase balances.
    let buyer_token_post = token_balance_of(ctx, contract_id, position, buyer.id)
        .await
        .expect("buyer token balance post-purchase");
    let owner_token_post = token_balance_of(ctx, contract_id, position, owner.id)
        .await
        .expect("owner token balance post-purchase");
    // Direct purchase with keepsDirectPurchaseHistory=true mints new
    // tokens to the buyer — owner stock is not the source.
    assert_eq!(
        buyer_token_post,
        buyer_token_pre + PURCHASE_AMOUNT,
        "buyer token balance must increase by PURCHASE_AMOUNT after mint-on-purchase \
         (pre={buyer_token_pre} post={buyer_token_post})"
    );
    assert_eq!(
        owner_token_post, owner_token_pre,
        "owner stock must be unchanged — direct purchase mints new tokens, \
         does not transfer from owner (pre={owner_token_pre} post={owner_token_post})"
    );

    let buyer_credits_post = <IdentityBalance as Fetch>::fetch(ctx.sdk(), buyer.id)
        .await
        .expect("buyer credit balance post-purchase")
        .expect("buyer balance present");
    let owner_credits_post = <IdentityBalance as Fetch>::fetch(ctx.sdk(), owner.id)
        .await
        .expect("owner credit balance post-purchase")
        .expect("owner balance present");

    let buyer_credit_drop = buyer_credits_pre.saturating_sub(buyer_credits_post);
    let owner_credit_gain = owner_credits_post.saturating_sub(owner_credits_pre);
    let purchase_fee = buyer_credit_drop.saturating_sub(TOTAL_AGREED_PRICE);

    assert!(
        buyer_credit_drop >= TOTAL_AGREED_PRICE,
        "buyer credits must drop by at least the agreed price \
         (drop={buyer_credit_drop} agreed={TOTAL_AGREED_PRICE})"
    );
    assert!(
        purchase_fee > 0,
        "buyer must pay a non-zero protocol fee on top of the price \
         (drop={buyer_credit_drop} agreed={TOTAL_AGREED_PRICE})"
    );
    // Owner's net gain is bounded by the agreed price; the protocol
    // pricing-schedule spec allows a seller-side fee to shave off some
    // of the incoming credits.
    assert!(
        owner_credit_gain <= TOTAL_AGREED_PRICE,
        "owner gain must not exceed the agreed price \
         (gain={owner_credit_gain} agreed={TOTAL_AGREED_PRICE})"
    );
    assert!(
        owner_credit_gain > 0,
        "owner must receive some credits from the purchase (gain={owner_credit_gain})"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_011",
        ?contract_id,
        buyer_credit_drop,
        owner_credit_gain,
        purchase_fee,
        "TK-011 purchase round-trip complete"
    );

    // TODO(spec-drift): once SetPriceResult / DirectPurchaseResult
    // expose actual_fee, also assert SetPriceResult.actual_fee > 0 and
    // DirectPurchaseResult.actual_fee > 0 per TEST_SPEC.md TK-011.

    let _ = DEFAULT_TOKEN_POSITION; // silence unused-import in stripped builds.

    s.setup.setup_guard.teardown().await.expect("teardown");
}
