//! TK-012 — Update token config (single ChangeItem mutation).
//!
//! Single-identity (owner) setup. Owner mutates `max_supply` via a
//! `TokenConfigurationChangeItem::MaxSupply(...)` and we re-fetch the
//! contract to confirm the change is observable on chain.
//!
//! Wave 2 stub: gated behind the `e2e` cargo feature. Wave 4 runs against live testnet.
//!
//! Spec drift note: TEST_SPEC.md asks for a positive `actual_fee` on
//! `ConfigUpdateResult`, but the bare SDK `ConfigUpdateResult` enum
//! (rs-sdk/src/platform/tokens/transitions/config_update.rs) does not
//! surface a fee field. Wave 4 will read the fee from credit-balance
//! deltas or wait on an SDK fee accessor; for now the `actual_fee`
//! assertion is a TODO.
//!
//! Each call to `setup_with_token_contract` deploys a brand-new
//! contract under a fresh owner — the spec's "fresh deploy" requirement
//! falls out for free.

use std::sync::Arc;

use dash_sdk::platform::tokens::builders::config_update::TokenConfigUpdateTransitionBuilder;
use dash_sdk::platform::Fetch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::data_contract::DataContract;

use crate::framework::prelude::*;
use crate::framework::tokens::{
    setup_with_token_contract, DEFAULT_MAX_SUPPLY, DEFAULT_TOKEN_POSITION,
    TK_OWNER_FUNDING_CONFIG_UPDATE,
};

/// Doubled max_supply target — `TEST_SPEC.md` TK-012 step 2.
const NEW_MAX_SUPPLY: u64 = 2_000_000_000_000_000;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_012_update_token_config_max_supply() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let ctx = E2eContext::init().await.expect("init e2e context");
    if ctx.skip_if_bank_floor_unmet("tk_012") {
        return;
    }
    let s = setup_with_token_contract(ctx, TK_OWNER_FUNDING_CONFIG_UPDATE)
        .await
        .expect("token + owner setup");

    let owner = &s.owner;
    let contract_id = s.contract_id;
    let position = s.token_position;

    // Pre-state: confirm the freshly-deployed contract has the default
    // max_supply we expect to mutate from.
    let pre_contract = DataContract::fetch(ctx.sdk(), contract_id)
        .await
        .expect("fetch pre-update contract")
        .expect("contract on chain");
    let pre_version = pre_contract.version();
    let pre_token_config = pre_contract
        .tokens()
        .get(&position)
        .expect("token slot present at default position");
    assert_eq!(
        pre_token_config.max_supply(),
        Some(DEFAULT_MAX_SUPPLY),
        "freshly-deployed permissive contract must have max_supply=DEFAULT_MAX_SUPPLY"
    );

    let pre_contract_arc = Arc::new(pre_contract);

    // Step 2: owner submits a single-ChangeItem mutation.
    let change_item = TokenConfigurationChangeItem::MaxSupply(Some(NEW_MAX_SUPPLY));
    let builder =
        TokenConfigUpdateTransitionBuilder::new(pre_contract_arc, position, owner.id, change_item);

    ctx.sdk()
        .token_update_contract_token_configuration(
            builder,
            &owner.critical_key,
            owner.signer.as_ref(),
        )
        .await
        .expect("config update transition");

    // Step 3: re-fetch the contract; assert max_supply moved and the
    // contract version (or token-config version, whichever DPP bumps)
    // advanced.
    let post_contract = DataContract::fetch(ctx.sdk(), contract_id)
        .await
        .expect("fetch post-update contract")
        .expect("contract still on chain");
    let post_version = post_contract.version();
    let post_token_config = post_contract
        .tokens()
        .get(&position)
        .expect("token slot still at default position");
    assert_eq!(
        post_token_config.max_supply(),
        Some(NEW_MAX_SUPPLY),
        "max_supply must reflect the change-item value (got {:?})",
        post_token_config.max_supply()
    );
    assert!(
        post_version >= pre_version,
        "contract version must not regress (pre={pre_version} post={post_version})"
    );
    // DPP bumps either the contract version or the token-config version
    // on a config mutation — at least one of the two must advance.
    let contract_version_bumped = post_version > pre_version;
    assert!(
        contract_version_bumped,
        "contract version must advance on a TokenConfigurationChangeItem mutation \
         (pre={pre_version} post={post_version})"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_012",
        ?contract_id,
        pre_version,
        post_version,
        new_max_supply = NEW_MAX_SUPPLY,
        "TK-012 max_supply update settled"
    );

    // TODO(spec-drift): once ConfigUpdateResult exposes actual_fee,
    // assert config_update_fee > 0 per TEST_SPEC.md TK-012.

    let _ = DEFAULT_TOKEN_POSITION; // pin import even when unused.

    s.setup_guard.teardown().await.expect("teardown");
}
