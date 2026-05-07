//! TK-002 — Token claim against a live perpetual distribution.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` § TK-002 (long-runtime, nightly only).
//! Demoted from CI tier because perpetual intervals run on testnet
//! block time (~3 s) and a meaningful claim window is 30–60 s of wall
//! clock; TK-013 covers the synchronous pre-programmed analogue.
//!
//! Editorial note (Wave 2-α): the spec entry calls for `TK-003`'s
//! helper to be **extended to take a `distribution_rules` override
//! (live perpetual)** — that extension is not on the Wave 1 baseline.
//! `setup_with_token_contract` only deploys the permissive owner-only
//! template (`perpetualDistribution: null`); the existing
//! `setup_with_token_pre_programmed_distribution` only handles the
//! pre-programmed shape. Wiring perpetual rules requires either a new
//! helper in `framework/tokens.rs` (out of scope for sub-team α — see
//! task constraints) or assembling the V0 `TokenPerpetualDistribution`
//! JSON inline, which is brittle without a tested round-trip.
//!
//! Following the panic-with-todo pattern authorised for
//! helper-blocked cases, the test sets up a baseline two-identity
//! token fixture and panics at the perpetual-rules step. Once the
//! helper lands, replace the panic with:
//!   1. deploy contract with `BlockBasedDistribution { interval: 1,
//!      function: FixedAmount(N), recipient: ContractOwner }`,
//!   2. wait for `interval` blocks (~30–60 s on testnet),
//!   3. `token_claim_with_signer(..., TokenDistributionType::Perpetual, ...)`,
//!   4. assert balance grew by ≥ N,
//!   5. (sub-case) second claim within same interval → "already claimed"
//!      / "no claimable amount" typed error.

use std::time::Duration;

use crate::framework::harness::E2eContext;
use crate::framework::tokens::{setup_with_token_and_two_identities, DEFAULT_TK_FUNDING};

/// Per-step deadline for token-balance observations.
#[allow(dead_code)]
const STEP_TIMEOUT: Duration = Duration::from_secs(120);

/// Minimum claim window in wall-clock seconds for the perpetual rule
/// once the helper lands. Sized to cover several testnet blocks
/// (~3 s/block) plus headroom.
#[allow(dead_code)]
const PERPETUAL_WAIT: Duration = Duration::from_secs(45);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
#[ignore = "blocked on Wave G perpetual-distribution helper (setup_with_token_contract `distribution_rules` override); also requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access"]
async fn tk_002_token_claim_perpetual_distribution() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Panic FIRST — running with `--ignored` against testnet would
    // otherwise burn a contract-create + 2× identity-register pair on
    // a contract that doesn't even carry the perpetual rules this
    // test is meant to exercise. Setup scaffolding is left below
    // (under `#[allow(unreachable_code)]`) so the eventual
    // implementor sees the shape the spec asks for.
    //
    // Wave 1's `framework/tokens.rs` does not expose a helper that
    // overrides `distributionRules.perpetualDistribution` on the
    // permissive template. Sub-team α is constrained from editing
    // `tokens.rs`; the helper extension is the work item that unblocks
    // this case.
    // INTENTIONAL(QA-V16-004): keep hard panic — Wave-G perpetual-distribution helper
    // genuinely missing. Needs `setup_with_token_perpetual_distribution` (or distribution_rules
    // override on `permissive_owner_token_contract_json`) in `framework/tokens.rs`. Failing
    // test documents the gap until the helper lands.
    panic!(
        "TK-002: requires Wave G perpetual-distribution helper \
         (setup_with_token_contract extended with `distribution_rules` override) — \
         see TEST_SPEC.md § TK-002"
    );

    #[allow(unreachable_code)]
    {
        let ctx = E2eContext::init().await.expect("init e2e context");

        // Baseline two-identity fixture so the funding + signer plumbing
        // is identical to TK-001 once the perpetual helper lands. The
        // contract deployed here uses the permissive owner-only template
        // with `perpetualDistribution: null` — i.e. NOT yet what TK-002
        // wants.
        let two = setup_with_token_and_two_identities(ctx, DEFAULT_TK_FUNDING)
            .await
            .expect("setup token + 2 identities");
        let _contract_id = two.setup.contract_id;
        let _position = two.setup.token_position;
        let _owner = &two.setup.owner;

        two.setup.setup_guard.teardown().await.expect("teardown");
    }
}
