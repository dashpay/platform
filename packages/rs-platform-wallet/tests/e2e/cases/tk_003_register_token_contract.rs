//! TK-003 — Register a permissive owner-only token contract.
//!
//! P0 foundation case. Exercises Wave G's
//! [`crate::framework::tokens::register_token_contract_via_sdk`] end
//! to end and asserts that the chain-derived contract id is
//! immediately fetchable via `DataContract::fetch` after the
//! broadcast resolves. Composes with [`setup_with_token_contract`]
//! which already drives the helper internally — TK-003 just pins the
//! observable post-conditions.
//!
//! Editorial note (Wave 1 Bilby): the helper signs with
//! [`RegisteredIdentity::master_key`] (MASTER, KeyID 0) because the
//! `RegisteredIdentity` snapshot only carries MASTER + HIGH on the
//! Wave A PR (#3578). The chain-side contract-create transition
//! validates the signing key against the contract's CRITICAL
//! requirement; if testnet ever rejects MASTER with
//! `InvalidSignatureError`, that is the trigger for Wave 4 (Marvin)
//! to pick up the signing-key-class upgrade and is asserted here as
//! a hard `panic!` so it surfaces unambiguously in CI logs.
//!
//! Gated behind the `e2e` cargo feature so a stock `cargo test -p platform-wallet`
//! stays green for contributors and CI jobs that lack a funded
//! testnet bank wallet, live DAPI access, and the operator `.env`.
//! See `cases/transfer.rs` for the operator-setup template.

use std::time::Duration;

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
use dpp::data_contract::DataContract;

use crate::framework::prelude::*;
use crate::framework::tokens::{DEFAULT_DECIMALS, DEFAULT_MAX_SUPPLY, TK_OWNER_FUNDING_SIMPLE};

/// Per-step deadline for the post-broadcast contract fetch. The
/// register helper already awaits the broadcast proof, so the fetch
/// should resolve on the first attempt; we keep a small budget for
/// trusted-context-provider warmup.
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn tk_003_register_token_contract() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Register the owner identity first so we can read its credit
    // balance pre-deploy and assert the contract-create fee delta
    // against it. We unfold the work that `setup_with_token_contract`
    // does internally (register identity + register contract) into
    // two phases so the credit-balance snapshot lands between them.
    let ctx = E2eContext::init().await.expect("init e2e context");
    if ctx.skip_if_bank_floor_unmet("tk_003") {
        return;
    }
    // QA-V39-002 — funding 35 B credits on a freshly-funded test wallet
    // address under `worker_threads = 12` parallel churn on the shared
    // bank wallet routinely needs more than the 60 s
    // `DEFAULT_SETUP_STEP_TIMEOUT`. Route through the explicit-budget
    // entry point with [`crate::framework::tokens::TK_SETUP_WAIT_TIMEOUT`]
    // (120 s) so the funding wait_for_balance has headroom for the
    // cross-replica replication lag.
    let setup_guard = crate::framework::setup_with_per_identity_funding(
        &[TK_OWNER_FUNDING_SIMPLE],
        crate::framework::tokens::TK_SETUP_WAIT_TIMEOUT,
    )
    .await
    .expect("register owner identity");
    let owner = setup_guard
        .identities
        .first()
        .expect("setup_with_n_identities returned empty identities");
    let owner_id = owner.id;

    let owner_credits_pre_deploy = IdentityBalance::fetch(ctx.sdk(), owner_id)
        .await
        .expect("fetch owner credits pre-deploy")
        .expect("owner identity present");

    let contract_json = crate::framework::tokens::permissive_owner_token_contract_json(
        owner_id,
        crate::framework::tokens::DEFAULT_TOKEN_POSITION,
        DEFAULT_MAX_SUPPLY,
    );
    let contract_id =
        match crate::framework::tokens::register_token_contract_via_sdk(ctx, owner, contract_json)
            .await
        {
            Ok(id) => id,
            Err(err) => {
                // Wave 1 editorial note: the framework signs with MASTER.
                // If chain-side rejection on signing-key class trips, the
                // helper surfaces it as a `FrameworkError::Sdk` carrying
                // `InvalidSignatureError`. Promote that to a sharp panic
                // so Wave 4 (Marvin) sees the trigger in CI logs without
                // any spelunking.
                let msg = err.to_string();
                if msg.contains("InvalidSignatureError") || msg.contains("InvalidIdentityPublicKey")
                {
                    tracing::error!(
                        target: "platform_wallet::e2e::cases::tk_003",
                        %msg,
                        "TK-003: chain rejected MASTER-signed DataContractCreate"
                    );
                    panic!(
                        "TK-003: signing key class needs CRITICAL upgrade — see Wave 1 \
                         editorial note in tokens.rs (master_key vs critical_key on \
                         RegisteredIdentity, PR #3578). underlying error: {msg}"
                    );
                }
                panic!("TK-003 setup failed: {msg}");
            }
        };

    let owner_credits_post_deploy = IdentityBalance::fetch(ctx.sdk(), owner_id)
        .await
        .expect("fetch owner credits post-deploy")
        .expect("owner identity present");

    // Round-trip: the chain-derived id returned by the helper must
    // resolve to a real contract whose ownerId matches the registering
    // identity. `DataContract::fetch` returns `Option<_>`; `None`
    // means the broadcast claimed success but the proof never landed.
    let fetched = tokio::time::timeout(FETCH_TIMEOUT, DataContract::fetch(ctx.sdk(), contract_id))
        .await
        .expect("fetch contract: timed out")
        .expect("fetch contract: SDK error")
        .expect("fetch contract: not found on chain after registration");

    let token_position = crate::framework::tokens::DEFAULT_TOKEN_POSITION;

    assert_eq!(
        fetched.id(),
        contract_id,
        "fetched contract id must match the helper's chain-derived id"
    );
    assert_eq!(
        fetched.owner_id(),
        owner_id,
        "contract ownerId must match the registering identity"
    );
    assert!(
        !fetched.tokens().is_empty(),
        "permissive owner-only contract must declare at least one token slot"
    );
    let token_config = fetched
        .tokens()
        .get(&token_position)
        .expect("contract must declare a token at the helper's default position");

    // Token shape — assert decimals + max_supply match what the
    // permissive helper baked into the JSON. A schema-drift in
    // `permissive_owner_token_contract_json` would otherwise deploy
    // successfully here without surfacing.
    assert_eq!(
        token_config.conventions().decimals(),
        DEFAULT_DECIMALS,
        "token decimals must match the helper's default"
    );
    assert_eq!(
        token_config.max_supply(),
        Some(DEFAULT_MAX_SUPPLY),
        "token max_supply must match the helper's default"
    );

    // Credit-fee assertion: the deploy must have decreased the
    // identity's credit balance by a non-zero amount (contract-create
    // fee). A regression that quietly stops charging contract-create
    // fees would surface here.
    assert!(
        owner_credits_post_deploy < owner_credits_pre_deploy,
        "owner credit balance must decrease after the contract-create transition \
         (pre={owner_credits_pre_deploy} post={owner_credits_post_deploy})"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_003",
        ?contract_id,
        ?owner_id,
        token_position,
        decimals = token_config.conventions().decimals(),
        max_supply = ?token_config.max_supply(),
        contract_create_fee = owner_credits_pre_deploy - owner_credits_post_deploy,
        "TK-003: token contract registered and fetched successfully"
    );

    setup_guard.teardown().await.expect("teardown");
}
