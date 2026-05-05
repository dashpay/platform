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
//! Gated behind `#[ignore]` so a stock `cargo test -p platform-wallet`
//! stays green for contributors and CI jobs that lack a funded
//! testnet bank wallet, live DAPI access, and the operator `.env`.
//! See `cases/transfer.rs` for the operator-setup template.

use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::DataContract;

use crate::framework::prelude::*;
use crate::framework::tokens::{setup_with_token_contract, DEFAULT_TK_FUNDING};

/// Per-step deadline for the post-broadcast contract fetch. The
/// register helper already awaits the broadcast proof, so the fetch
/// should resolve on the first attempt; we keep a small budget for
/// trusted-context-provider warmup.
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio_shared_rt::test(shared)]
#[ignore = "TK-003: requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access; run with `cargo test -- --ignored`"]
async fn tk_003_register_token_contract() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let setup = match setup_with_token_contract_with_master_signing_diagnostic().await {
        Ok(s) => s,
        Err(err) => {
            // Wave 1 editorial note: the framework signs with MASTER.
            // If chain-side rejection on signing-key class trips, the
            // helper surfaces it as a `FrameworkError::Sdk` carrying
            // `InvalidSignatureError`. Promote that to a sharp panic
            // so Wave 4 (Marvin) sees the trigger in CI logs without
            // any spelunking.
            let msg = err.to_string();
            if msg.contains("InvalidSignatureError") || msg.contains("InvalidIdentityPublicKey") {
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

    let ctx = setup.setup_guard.base.ctx;
    let contract_id = setup.contract_id;
    let owner_id = setup.owner.id;

    // Round-trip: the chain-derived id returned by the helper must
    // resolve to a real contract whose ownerId matches the registering
    // identity. `DataContract::fetch` returns `Option<_>`; `None`
    // means the broadcast claimed success but the proof never landed.
    let fetched = tokio::time::timeout(FETCH_TIMEOUT, DataContract::fetch(ctx.sdk(), contract_id))
        .await
        .expect("fetch contract: timed out")
        .expect("fetch contract: SDK error")
        .expect("fetch contract: not found on chain after registration");

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
    assert!(
        fetched.tokens().contains_key(&setup.token_position),
        "contract must declare a token at the helper's default position {}",
        setup.token_position,
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::tk_003",
        ?contract_id,
        ?owner_id,
        token_position = setup.token_position,
        "TK-003: token contract registered and fetched successfully"
    );

    setup.setup_guard.teardown().await.expect("teardown");
}

/// Thin shim around [`setup_with_token_contract`] so the test body
/// can map the `FrameworkResult` into a structured panic for the
/// MASTER-vs-CRITICAL signing diagnostic above. Splitting the call
/// keeps the diagnostic prose and the happy path readable.
async fn setup_with_token_contract_with_master_signing_diagnostic(
) -> FrameworkResult<crate::framework::tokens::TokenSetup> {
    // Late `init` so the diagnostic owns the very first SDK error
    // (the helper does not retry on `InvalidSignatureError`).
    let ctx = E2eContext::init().await?;
    setup_with_token_contract(ctx, DEFAULT_TK_FUNDING).await
}
