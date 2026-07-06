//! Found — CoinJoin funds invisible to a default wallet because a matched
//! block is applied to a wallet exactly once, so addresses derived from
//! that block's own matches are never tested against it.
//!
//! Reproduction (diagnose-only) for the report: a testnet wallet with
//! deep CoinJoin usage does not fully sync under default settings — most
//! CoinJoin funds stay invisible.
//!
//! ## Root cause (NOT the gap limit, NOT scattered history)
//!
//! The CoinJoin gap limit (`DEFAULT_COINJOIN_GAP_LIMIT` = 30) is ample.
//! On this wallet the CoinJoin External keychain is used *densely and
//! effectively contiguously* in address index — up to index 1727, with
//! the largest unused run only 12 and no unused run >= 30 anywhere (see
//! the gap table the test prints). The funding is also dense in HEIGHT,
//! not scattered: block 1415403 first-funds indices 0..=51 (52 outputs in
//! one block), block 1415404 funds 52..=139, and there is no index↔height
//! inversion below 1767. Neither gap size nor block spread is the problem.
//!
//! The defect is that a matched block is APPLIED to a given wallet exactly
//! ONCE, against only the addresses GENERATED at that instant:
//!
//! 1. When a block is applied, `check_transaction_for_match` recognises
//!    only already-generated addresses (key-wallet
//!    `account_checker.rs:651-654`). For block 1415403 the wallet has
//!    generated 0..29 (the gap window), so only outputs paying 0..29 are
//!    seen; outputs paying 30..51 in the SAME block are invisible. The
//!    matched 0..29 are marked used, then `maintain_gap_limit` derives
//!    30..59.
//! 2. On batch commit, `rescan_batch` re-matches the block's OWN filters
//!    against the newly-derived scripts (dash-spv `manager.rs:479`), and
//!    indices 30..51 genuinely match. BUT the per-`(wallet, BLOCK)` gate
//!    `BlockMatchTracker` (`manager.rs:667-668` →
//!    `block_match_tracker.rs:78-82`) returns `AlreadyProcessed` — the
//!    wallet was recorded done for this block at `sync_manager.rs:178` —
//!    so the block is skipped and NEVER re-applied. The gate is keyed by
//!    `(wallet, block)`, not `(wallet, address)`: that is the bug.
//!
//! So the watch ceiling lifts exactly one gap step per dense block. Block
//! 1415404 then adds only 52..59 (now watched), and discovery stalls at
//! `highest_used = 59` (= 29 initial watch + 30 gap) — deterministically.
//! Indices 30..51 (used only in the already-committed block 1415403) are
//! never recovered. The
//! [`found_coinjoin_gap_limit_sync_height_analysis`] test's block-atomic
//! simulation reproduces this 59 from the live `h(i)` data.
//!
//! Fix direction (not implemented here): make `BlockMatchTracker` track
//! the processed SCRIPTS per block so a new-script residual re-queues the
//! block, OR re-test the block's own outputs against newly-derived
//! addresses to a fixpoint inside `process_block` BEFORE `record_processed`.
//! The full-rescan simulation below shows either recovers the entire
//! range 0..1727.
//!
//! ## What this proves
//!
//! Two wallets are restored from the SAME testnet mnemonic and each
//! synced in its own capped SPV pass (genesis → [`SYNC_CUTOFF_HEIGHT`],
//! the last testnet block of Sunday 2026-06-07 UTC) against the same
//! chain window:
//!
//! - **Wallet A** — [`WalletAccountCreationOptions::Default`]. CoinJoin
//!   account 0 starts with a `DEFAULT_COINJOIN_GAP_LIMIT` (30) address
//!   window and relies on mid-scan discovery to extend it; that
//!   discovery is what fails.
//! - **Wallet B** — [`WalletAccountCreationOptions::AllAccounts`] with
//!   CoinJoin account 0, plus a [`WIDE_DERIVATION`] pre-derivation across
//!   every funding keychain (BIP-44 external/internal AND the testnet
//!   CoinJoin path `m/9'/1'/4'`), generated BEFORE sync so the bloom
//!   filter watches those scripts from the first batch — no mid-scan
//!   discovery is needed for the pre-derived range.
//!
//! Same seed + same network yields an IDENTICAL wallet id, so A and B
//! cannot coexist in one manager (`WalletManager` keys on wallet id).
//! Each therefore lives in its own [`PlatformWalletManager`] (sharing
//! one SDK) and runs its own capped pass. The per-wallet bloom filter is
//! built from that wallet's `monitored_addresses` (all generated
//! addresses across every account), so Wallet B sees CoinJoin funds that
//! Wallet A's once-per-block discovery never reaches — `balance_B >
//! balance_A`.
//!
//! ## Non-determinism of the delta
//!
//! Wallet A's CoinJoin stall is deterministic (highest_used = 59 every
//! run — block-atomic discovery has no race there). The reported delta
//! still varies run to run (447M / 739M / 2022M duffs seen) because of
//! the BIP-44 side and the cap-overshoot tail: chainlock promotion runs
//! a little past the filter cap during teardown, so each run captures a
//! slightly different set of recent CoinJoin/BIP-44 txs. The test
//! therefore asserts only the qualitative `balance_B > balance_A`, never
//! an exact amount.
//!
//! ## Reliable workaround (not asserted here)
//!
//! Pre-derive CoinJoin addresses BEYOND the highest used index (~1727,
//! e.g. 2500) BEFORE sync, so every script is watched from scan start
//! and no mid-scan discovery is needed. A fixed shallow pre-derivation
//! ([`WIDE_DERIVATION`] = 200) is only a *probabilistic* mitigation —
//! it widens the window but does not cover the full used range, which is
//! why Wallet B's exact result still varies.
//!
//! ## Why bypass `setup()`
//!
//! The e2e harness `setup()` requires a funded bank identity and forces
//! `WalletAccountCreationOptions::Default`. This reproduction needs
//! neither funding nor the default-only path, so it builds the SDK +
//! manager + SPV directly (mirroring `tests/spv_sync.rs`) and restores
//! the wallet itself. No bank mnemonic dependency.
//!
//! ## Run
//!
//! ```bash
//! cargo test -p platform-wallet --test e2e --features e2e -- \
//!   --exact cases::found_coinjoin_gap_limit_sync::found_coinjoin_gap_limit_sync \
//!   --nocapture
//! ```
//!
//! A cold testnet scan from genesis can exceed 10 minutes.

use std::sync::Arc;
use std::time::Duration;

use dash_sdk::dapi_client::AddressList;
use dash_spv::client::config::MempoolStrategy;
use dash_spv::types::ValidationMode;
use dash_spv::ClientConfig;
use key_wallet::account::AccountType;
use key_wallet::gap_limit::DEFAULT_COINJOIN_GAP_LIMIT;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::Network;
use key_wallet::{KeySource, Mnemonic};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::{changeset::PlatformWalletPersistence, PlatformWalletManager};

/// English BIP-39 mnemonic for the testnet wallet under test.
const TEST_MNEMONIC: &str =
    "example tackle fashion marine blind focus bamboo flight gauge word duck say";

/// Last testnet block of Sunday 2026-06-07 UTC. Both wallets sync only
/// up to here (via [`platform_wallet::SpvRuntime::set_terminal_height`])
/// so the comparison is taken BEFORE any later transfers move funds.
const SYNC_CUTOFF_HEIGHT: u32 = 1_491_827;

/// Pre-derivation depth for Wallet B, generated before sync so the
/// first 200 indices on every keychain are watched from the first
/// batch — no mid-scan discovery needed for that range. This is a
/// PROBABILISTIC mitigation, not a fix: the CoinJoin keychain is used
/// up to index 1727, so 200 only widens the window. Watching the full
/// used range (~2500) before sync would be the reliable workaround.
const WIDE_DERIVATION: u32 = 200;

/// CoinJoin External pre-derivation depth for the height-analysis test.
/// Past the highest-used index (~1799 once the cap-overshoot tail is
/// counted) so EVERY used index is watched from genesis and none can be
/// missed regardless of block ordering — the ground-truth completeness
/// condition.
const COINJOIN_GROUND_TRUTH_DEPTH: u32 = 2500;

/// Effective backward re-scan depth (in blocks) the windowed simulation
/// grants a freshly-watched address against already-committed blocks.
///
/// `0` because the real dash-spv `rescan_batch` re-match (which would
/// catch the new scripts) is gated out per block: `BlockMatchTracker`
/// returns `AlreadyProcessed` for a `(wallet, block)` already recorded,
/// so the block is never re-applied even though its outputs now match.
/// The wallet's net behaviour is therefore zero effective backward
/// re-scan — empirically validated: `0` reproduces the observed stall at
/// index 59, whereas any value `>= 1` (the fixed behaviour) would recover
/// the full pre-cutoff range.
const SPV_BACKWARD_RESCAN_BLOCKS: u32 = 0;

/// Cold genesis-scan budget. The harness's own SPV cold-cache floor is
/// 600 s; this gives headroom over that for the capped historical walk.
const SYNC_TIMEOUT: Duration = Duration::from_secs(1200);

/// Poll cadence while waiting for the capped sync to land.
const POLL_INTERVAL: Duration = Duration::from_secs(3);

/// No-op persister — the reproduction reads balances from the live
/// in-memory wallet state, not from disk.
struct NoopPersister;

impl PlatformWalletPersistence for NoopPersister {
    fn store(
        &self,
        _wallet_id: WalletId,
        _changeset: platform_wallet::changeset::PlatformWalletChangeSet,
    ) -> Result<(), platform_wallet::changeset::PersistenceError> {
        Ok(())
    }

    fn flush(
        &self,
        _wallet_id: WalletId,
    ) -> Result<(), platform_wallet::changeset::PersistenceError> {
        Ok(())
    }

    fn load(
        &self,
    ) -> Result<
        platform_wallet::changeset::ClientStartState,
        platform_wallet::changeset::PersistenceError,
    > {
        Ok(platform_wallet::changeset::ClientStartState::default())
    }
}

/// No-op event handler — SPV updates the wallet balance atomics
/// directly via the manager's internal `BalanceUpdateHandler`.
struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 4)]
async fn found_coinjoin_gap_limit_sync() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let network = Network::Testnet;

    // --- SDK (testnet bootstrap seeds + trusted context provider) ---
    let sdk = build_testnet_sdk(network);

    // --- Manager wired to a no-op persister ---
    let persister = Arc::new(NoopPersister);
    let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    let manager = Arc::new(PlatformWalletManager::new(
        Arc::clone(&sdk),
        persister,
        vec![event_handler],
    ));

    // --- Restore both wallets BEFORE sync so the bloom filter watches
    // every address. `birth_height_override = Some(0)` forces a full
    // historical scan from genesis (the funds predate the cutoff and
    // their exact birth block is not known here). The scan is capped at
    // SYNC_CUTOFF_HEIGHT, so the cost is bounded.
    let mnemonic: Mnemonic = TEST_MNEMONIC.parse().expect("valid BIP-39 mnemonic");
    let seed = mnemonic.to_seed("");

    // Wallet A — default configuration.
    let wallet_a = manager
        .create_wallet_from_seed_bytes(
            network,
            &seed,
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("create Wallet A (Default)");

    // Wallet B — AllAccounts incl. CoinJoin account 0. Same seed + same
    // network yields an IDENTICAL wallet id, which would collide inside
    // one manager (`WalletAlreadyExists`). So B lives in a SECOND
    // manager that shares the same SDK; each manager runs its own capped
    // SPV pass against the same testnet chain window, giving an
    // apples-to-apples comparison of the two gap windows.
    let manager_b = Arc::new(PlatformWalletManager::new(
        Arc::clone(&sdk),
        Arc::new(NoopPersister),
        vec![Arc::new(NoopEventHandler) as Arc<dyn PlatformEventHandler>],
    ));
    let mut coinjoin = std::collections::BTreeSet::new();
    coinjoin.insert(0u32);
    let mut bip44 = std::collections::BTreeSet::new();
    bip44.insert(0u32);
    let wallet_b = manager_b
        .create_wallet_from_seed_bytes(
            network,
            &seed,
            WalletAccountCreationOptions::AllAccounts(
                bip44,
                Default::default(),
                coinjoin,
                Default::default(),
                Default::default(),
            ),
            Some(0),
        )
        .await
        .expect("create Wallet B (AllAccounts + CoinJoin)");

    // Pre-derive on EVERY funding keychain of Wallet B BEFORE sync, so
    // those scripts (incl. the CoinJoin path m/9'/1'/4') are in the
    // bloom filter from the first batch and need no mid-scan discovery.
    pre_derive_wide(&wallet_b, WIDE_DERIVATION).await;

    let watched_a = monitored_count(&wallet_a).await;
    let watched_b = monitored_count(&wallet_b).await;
    tracing::info!(
        target: "platform_wallet::e2e::cases::found_coinjoin_gap_limit_sync",
        watched_a,
        watched_b,
        coinjoin_gap_limit_default = DEFAULT_COINJOIN_GAP_LIMIT,
        wide_derivation = WIDE_DERIVATION,
        "pre-sync watched-address counts"
    );

    // --- Capped sync, Wallet A first, then Wallet B, against the same
    // chain window. Each pass halts once filters commit to the cutoff.
    sync_capped(&manager, network, &sdk, "A").await;
    sync_capped(&manager_b, network, &sdk, "B").await;

    // --- Read balances + diagnostics ---
    let balance_a = wallet_a.balance().confirmed();
    let balance_b = wallet_b.balance().confirmed();
    let synced_a = wallet_a.state().await.core_wallet.synced_height();
    let synced_b = wallet_b.state().await.core_wallet.synced_height();

    let per_account_a = per_account_report(&wallet_a).await;
    let per_account_b = per_account_report(&wallet_b).await;

    tracing::info!(
        target: "platform_wallet::e2e::cases::found_coinjoin_gap_limit_sync",
        balance_a,
        balance_b,
        delta = balance_b.saturating_sub(balance_a),
        synced_a,
        synced_b,
        "REPRODUCTION RESULT"
    );
    for line in &per_account_a {
        tracing::info!(target: "platform_wallet::e2e::cases::found_coinjoin_gap_limit_sync", wallet = "A", "{line}");
    }
    for line in &per_account_b {
        tracing::info!(target: "platform_wallet::e2e::cases::found_coinjoin_gap_limit_sync", wallet = "B", "{line}");
    }

    println!("=== CoinJoin gap-limit reproduction ===");
    println!("balance_A (Default, gap={DEFAULT_COINJOIN_GAP_LIMIT}): {balance_a} duffs");
    println!("balance_B (AllAccounts + {WIDE_DERIVATION} wide):       {balance_b} duffs");
    println!(
        "delta (hidden by default config):                      {} duffs",
        balance_b.saturating_sub(balance_a)
    );
    println!("synced height: A={synced_a}, B={synced_b} (cutoff target={SYNC_CUTOFF_HEIGHT})");
    println!("--- Wallet A per-account ---");
    for line in &per_account_a {
        println!("  {line}");
    }
    println!("--- Wallet B per-account ---");
    for line in &per_account_b {
        println!("  {line}");
    }

    // --- Gap analysis on Wallet B's synced state: the unused-index runs
    // between consecutive USED addresses, contrasted against the index
    // Wallet A's default scan actually reached on the CoinJoin keychain.
    let default_ceiling = coinjoin_external_highest_used(&wallet_a).await;
    let gap_report = gap_analysis_report(&wallet_b, default_ceiling).await;
    for line in &gap_report {
        tracing::info!(target: "platform_wallet::e2e::cases::found_coinjoin_gap_limit_sync", "{line}");
    }
    println!("\n=== Wallet B gap analysis (used-index runs per keychain) ===");
    for line in &gap_report {
        println!("{line}");
    }

    // The reproduction assertion: pre-watching the scripts (Wallet B)
    // reveals CoinJoin funds that the default wallet's forward-only
    // mid-scan discovery (Wallet A) never reaches. Qualitative only —
    // the delta is non-deterministic (see module docs).
    assert!(
        balance_b > balance_a,
        "BUG NOT REPRODUCED: expected balance_B ({balance_b}) > balance_A ({balance_a}). \
         Either the CoinJoin funds did not require mid-scan discovery at this cutoff \
         height, or the pre-derivation did not widen the watched set. Check the \
         per-account reports above for where the funds sit."
    );
}

/// Build a testnet SDK with bootstrap seeds and a trusted HTTP context
/// provider. The reproduction only needs Core (Layer-1) SPV balance, so
/// the provider is wired with the network-builtin testnet quorums URL.
fn build_testnet_sdk(network: Network) -> Arc<dash_sdk::Sdk> {
    use std::num::NonZeroUsize;

    use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

    let cache_size = NonZeroUsize::new(256).expect("non-zero");
    let provider = TrustedHttpContextProvider::new(network, None, cache_size)
        .expect("build testnet trusted context provider");
    let sdk = dash_sdk::SdkBuilder::new_testnet()
        .with_context_provider(provider)
        .build()
        .expect("build testnet SDK");
    Arc::new(sdk)
}

/// Pre-derive `count` addresses on every funding keychain of the wallet
/// (BIP-44 external/internal, CoinJoin) so their scripts are in the
/// bloom filter from scan start, sidestepping the forward-only mid-scan
/// discovery for the pre-derived range. Generation happens against the
/// account's public xpub (`KeySource::Public`).
async fn pre_derive_wide(wallet: &Arc<platform_wallet::PlatformWallet>, count: u32) {
    let wallet_id = wallet.wallet_id();
    let mut wm = wallet.wallet_manager().write().await;
    let (managed_wallet, info) = wm
        .get_wallet_mut_and_info_mut(&wallet_id)
        .expect("wallet present in manager");

    // Map each funding account type → its account xpub (the key source
    // for address derivation), snapshotted from the signing wallet.
    let key_sources: std::collections::BTreeMap<AccountType, KeySource> = managed_wallet
        .accounts
        .all_accounts()
        .iter()
        .map(|a| (a.account_type, KeySource::Public(a.account_xpub)))
        .collect();

    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    for funds in info.core_wallet.accounts.all_funding_accounts_mut() {
        let account_type = funds.managed_account_type().to_account_type();
        let Some(key_source) = key_sources.get(&account_type) else {
            continue;
        };
        for pool in funds.managed_account_type_mut().address_pools_mut() {
            let already = pool.highest_generated.map(|h| h + 1).unwrap_or(0);
            if already >= count {
                continue;
            }
            let to_generate = count - already;
            if let Err(e) = pool.generate_addresses(to_generate, key_source, true) {
                tracing::warn!(
                    target: "platform_wallet::e2e::cases::found_coinjoin_gap_limit_sync",
                    ?account_type,
                    error = %e,
                    "pre-derivation failed for a pool; continuing"
                );
            }
        }
    }
}

/// Count the addresses the manager would put in the bloom filter for
/// this wallet (`monitored_addresses` = all generated addresses across
/// every account).
async fn monitored_count(wallet: &Arc<platform_wallet::PlatformWallet>) -> usize {
    let wallet_id = wallet.wallet_id();
    let wm = wallet.wallet_manager().read().await;
    wm.get_wallet_info(&wallet_id)
        .map(|info| info.monitored_addresses().len())
        .unwrap_or(0)
}

/// Per-funding-account confirmed-balance + watched-index report, so the
/// run output names WHICH keychain (and index range) holds the funds
/// the default configuration hides.
async fn per_account_report(wallet: &Arc<platform_wallet::PlatformWallet>) -> Vec<String> {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

    let state = wallet.state().await;
    let mut out = Vec::new();
    for funds in state.core_wallet.accounts.all_funding_accounts() {
        let account_type = funds.managed_account_type().to_account_type();
        let confirmed = funds.balance.confirmed();
        let pools: Vec<String> = funds
            .managed_account_type()
            .address_pools()
            .iter()
            .map(|p| {
                format!(
                    "{:?}[gap={} highest_used={:?} highest_generated={:?}]",
                    p.pool_type, p.gap_limit, p.highest_used, p.highest_generated
                )
            })
            .collect();
        out.push(format!(
            "{account_type:?}: confirmed={confirmed} duffs; pools={}",
            pools.join(", ")
        ));
    }
    out
}

/// Read the `highest_used` index of the wallet's CoinJoin account 0
/// External pool — the depth its scan actually reached. `None` when the
/// wallet has no CoinJoin account or the pool saw no usage.
async fn coinjoin_external_highest_used(
    wallet: &Arc<platform_wallet::PlatformWallet>,
) -> Option<u32> {
    use key_wallet::account::AccountType as AT;
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

    let state = wallet.state().await;
    for funds in state.core_wallet.accounts.all_funding_accounts() {
        if matches!(
            funds.managed_account_type().to_account_type(),
            AT::CoinJoin { .. }
        ) {
            for pool in funds.managed_account_type().address_pools() {
                if pool.pool_type == AddressPoolType::External {
                    return pool.highest_used;
                }
            }
        }
    }
    None
}

/// Per-keychain gap analysis: for every funding pool, the sorted USED
/// indices, the leading unused run (index 0 → first used), the unused
/// run between each consecutive used pair, and a summary tying the
/// pattern to what the default-configured Wallet A actually observed.
///
/// `default_observed_ceiling` is the `highest_used` Wallet A reached on
/// the matching keychain (e.g. CoinJoin External = 59), so the report
/// can quantify the indices the default scan never touched even though
/// the wide wallet (B) used them.
async fn gap_analysis_report(
    wallet: &Arc<platform_wallet::PlatformWallet>,
    coinjoin_external_default_ceiling: Option<u32>,
) -> Vec<String> {
    use key_wallet::account::AccountType as AT;
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

    let state = wallet.state().await;
    let mut out = Vec::new();
    for funds in state.core_wallet.accounts.all_funding_accounts() {
        let account_type = funds.managed_account_type().to_account_type();
        for pool in funds.managed_account_type().address_pools() {
            let mut used: Vec<u32> = pool.used_indices.iter().copied().collect();
            used.sort_unstable();

            out.push(format!(
                "── {account_type:?} / {:?} (gap_limit={}) ──",
                pool.pool_type, pool.gap_limit
            ));
            if used.is_empty() {
                out.push("   used indices: (none)".to_string());
                continue;
            }
            out.push(format!("   used indices ({}): {used:?}", used.len()));

            // Leading run: index 0 → first used. A first-used index >= the
            // gap limit would itself defeat a from-zero scan.
            let first = used[0];
            if first > 0 {
                let lead_flag = if first >= DEFAULT_COINJOIN_GAP_LIMIT {
                    "  <<< LEADING GAP >= 30"
                } else {
                    ""
                };
                out.push(format!(
                    "   leading run 0 → {first}: {first} unused address(es){lead_flag}"
                ));
            }

            // Inter-used unused runs; report only the non-zero ones to
            // keep the table readable, and the max so the reader sees the
            // largest canyon at a glance.
            let mut max_gap = 0u32;
            let mut gaps_ge_30 = 0u32;
            for win in used.windows(2) {
                let (prev, next) = (win[0], win[1]);
                let gap = next - prev - 1;
                max_gap = max_gap.max(gap);
                if gap > 0 {
                    let flag = if gap >= DEFAULT_COINJOIN_GAP_LIMIT {
                        gaps_ge_30 += 1;
                        "  <<< GAP >= 30 (a from-zero gap-30 follow cannot bridge this)"
                    } else {
                        ""
                    };
                    out.push(format!(
                        "   used {prev} → {next}: {gap} unused address(es){flag}"
                    ));
                }
            }

            let highest = *used.last().expect("non-empty");
            out.push(format!(
                "   summary: {} used indices, span [{first}..{highest}], \
                 max inter-used unused run = {max_gap}, runs >= 30 = {gaps_ge_30}",
                used.len()
            ));

            // For CoinJoin External, contrast against Wallet A's observed
            // ceiling — the headline finding of this reproduction.
            let is_coinjoin_external = matches!(account_type, AT::CoinJoin { .. })
                && pool.pool_type == AddressPoolType::External;
            if is_coinjoin_external {
                if let Some(ceiling) = coinjoin_external_default_ceiling {
                    let hidden = used.iter().filter(|&&i| i > ceiling).count();
                    out.push(format!(
                        "   ⇒ DEFAULT WALLET (Wallet A) stalled at highest_used={ceiling}; \
                         {hidden} of these {} used indices sit ABOVE that ceiling and were \
                         INVISIBLE to the default scan — even though the usage here is \
                         {} (max inter-used run = {max_gap}). The defeat is the shallow \
                         initial pre-derivation window (gap_limit={}) plus the SPV \
                         historical scan failing to advance the watched window across a \
                         deep CoinJoin run, NOT an inter-address gap >= 30.",
                        used.len(),
                        if gaps_ge_30 == 0 {
                            "effectively contiguous"
                        } else {
                            "punctuated by some large runs"
                        },
                        pool.gap_limit,
                    ));
                }
            }
        }
    }
    out
}

/// Run ONE capped SPV pass for `manager` against `network`, halting once
/// filters commit to [`SYNC_CUTOFF_HEIGHT`]. Seeds P2P peers from the
/// SDK's live testnet address list (port 19999). Storage is anchored in
/// a fresh per-label temp dir so the two passes don't share state.
async fn sync_capped(
    manager: &Arc<PlatformWalletManager<NoopPersister>>,
    network: Network,
    sdk: &Arc<dash_sdk::Sdk>,
    label: &str,
) {
    let storage_path = std::env::temp_dir().join(format!(
        "platform-wallet-coinjoin-gaplimit-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&storage_path).expect("create SPV storage dir");

    let mut config = ClientConfig::new(network)
        .with_storage_path(storage_path)
        .with_validation_mode(ValidationMode::Full)
        .with_start_height(0)
        .with_mempool_tracking(MempoolStrategy::BloomFilter);
    seed_p2p_peers(&mut config, sdk.address_list(), 19999);

    let spv = manager.spv_arc();
    spv.set_terminal_height(Some(SYNC_CUTOFF_HEIGHT));
    spv.spawn_in_background(config);

    let start = std::time::Instant::now();
    let mut last_committed = 0u32;
    loop {
        if start.elapsed() > SYNC_TIMEOUT {
            let _ = spv.stop().await;
            panic!(
                "wallet {label}: capped sync did not reach cutoff {SYNC_CUTOFF_HEIGHT} \
                 within {SYNC_TIMEOUT:?} (last committed filter height {last_committed})"
            );
        }

        if let Some(progress) = spv.sync_progress().await {
            if let Ok(filters) = progress.filters() {
                let committed = filters.committed_height();
                if committed != last_committed {
                    tracing::info!(
                        target: "platform_wallet::e2e::cases::found_coinjoin_gap_limit_sync",
                        wallet = label,
                        committed,
                        cutoff = SYNC_CUTOFF_HEIGHT,
                        "capped filter sync progress"
                    );
                    last_committed = committed;
                }
                if committed >= SYNC_CUTOFF_HEIGHT {
                    break;
                }
            }
        }

        // The runtime self-stops at the cap; once it has, `is_started`
        // flips false and we're done regardless of the last poll value.
        if !spv.is_started() && start.elapsed() > Duration::from_secs(10) {
            break;
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }

    // Ensure the runtime is fully stopped before the next pass reuses
    // the shared SDK / spawns its own runtime.
    let _ = spv.stop().await;
    // Clear the cap so a hypothetical re-run of this manager wouldn't
    // inherit it (defence-in-depth; the manager is dropped after).
    spv.set_terminal_height(None);
}

/// Seed `config` with P2P peers extracted from the SDK's live testnet
/// address list. Non-IP hosts fall through to SPV's own DNS discovery.
fn seed_p2p_peers(config: &mut ClientConfig, address_list: &AddressList, port: u16) {
    use std::net::{IpAddr, SocketAddr};

    for address in address_list.get_live_addresses() {
        if let Some(host) = address.uri().host() {
            if let Ok(ip) = host.parse::<IpAddr>() {
                config.add_peer(SocketAddr::new(ip, port));
            }
        }
    }
}

/// F1 falsification — gap-limit sweep. Rebuilds the default wallet's
/// CoinJoin External pool at a chosen gap limit `g` (initial watch window
/// exactly `0..g-1`), syncs the real testnet chain to the cutoff, and
/// reports the ACTUAL CoinJoin External `highest_used`. The block-atomic
/// single-apply diagnosis predicts a SPECIFIC stall per `g` (computed
/// offline from the `h(i)` per-block funding); the naive "any gap > the
/// max unused run (12) finds everything" predicts the full range for any
/// `g >= 13`. The two diverge sharply (e.g. `g=13`: model ~12 vs naive
/// ~1799), so this run discriminates them.
///
/// Gap is read from `F1_COINJOIN_GAP` (default 30 — the anchor that
/// empirically stalls at 59). `#[ignore]`: heavyweight real sync, run with
/// `--ignored` and `F1_COINJOIN_GAP=<g>` set.
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 4)]
#[ignore = "F1 falsification: heavyweight real testnet sync; run with --ignored and F1_COINJOIN_GAP set"]
async fn found_coinjoin_gap_limit_sweep_f1() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=warn".into()),
        )
        .with_test_writer()
        .try_init();

    let gap: u32 = std::env::var("F1_COINJOIN_GAP")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(DEFAULT_COINJOIN_GAP_LIMIT);

    let network = Network::Testnet;
    let sdk = build_testnet_sdk(network);
    let manager = Arc::new(PlatformWalletManager::new(
        Arc::clone(&sdk),
        Arc::new(NoopPersister),
        vec![Arc::new(NoopEventHandler) as Arc<dyn PlatformEventHandler>],
    ));

    let mnemonic: Mnemonic = TEST_MNEMONIC.parse().expect("valid BIP-39 mnemonic");
    let seed = mnemonic.to_seed("");
    let mut coinjoin = std::collections::BTreeSet::new();
    coinjoin.insert(0u32);
    let mut bip44 = std::collections::BTreeSet::new();
    bip44.insert(0u32);
    let wallet = manager
        .create_wallet_from_seed_bytes(
            network,
            &seed,
            WalletAccountCreationOptions::AllAccounts(
                bip44,
                Default::default(),
                coinjoin,
                Default::default(),
                Default::default(),
            ),
            Some(0),
        )
        .await
        .expect("create sweep wallet");

    set_coinjoin_gap_limit(&wallet, gap).await;

    sync_capped(&manager, network, &sdk, &format!("F1g{gap}")).await;

    let state = wallet.state().await;
    let (highest_used, highest_generated, confirmed) =
        coinjoin_external_pool_state(&state).expect("CoinJoin External pool present");

    println!("\n=== F1 gap-limit sweep ===");
    println!(
        "gap={gap}: CoinJoin External actual highest_used={highest_used:?} \
         highest_generated={highest_generated:?} confirmed={confirmed} duffs"
    );
    tracing::info!(
        target: "platform_wallet::e2e::cases::found_coinjoin_gap_limit_sync",
        gap,
        ?highest_used,
        ?highest_generated,
        confirmed,
        "F1 gap-limit sweep result"
    );
}

/// Rebuild the wallet's CoinJoin account-0 External pool at gap limit
/// `gap`, regenerating exactly indices `0..gap-1` so the initial watch
/// window matches a wallet created with that gap. Production hardcodes
/// `DEFAULT_COINJOIN_GAP_LIMIT` at account construction (key-wallet
/// `managed_account_collection.rs:595`), so this reaches the otherwise
/// fixed knob directly via the pool's public fields.
async fn set_coinjoin_gap_limit(wallet: &Arc<platform_wallet::PlatformWallet>, gap: u32) {
    use key_wallet::account::AccountType as AT;
    use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType};
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

    let wallet_id = wallet.wallet_id();
    let mut wm = wallet.wallet_manager().write().await;
    let (managed_wallet, info) = wm
        .get_wallet_mut_and_info_mut(&wallet_id)
        .expect("wallet present in manager");

    let key_source = managed_wallet
        .accounts
        .coinjoin_accounts
        .get(&0)
        .map(|a| KeySource::Public(a.account_xpub))
        .expect("coinjoin account 0 xpub");
    let network = managed_wallet.network;

    for funds in info.core_wallet.accounts.all_funding_accounts_mut() {
        if !matches!(
            funds.managed_account_type().to_account_type(),
            AT::CoinJoin { .. }
        ) {
            continue;
        }
        for pool in funds.managed_account_type_mut().address_pools_mut() {
            if pool.pool_type != AddressPoolType::External {
                continue;
            }
            // Rebuild from the existing base path so the rebuilt pool
            // derives the identical scripts, with exactly `gap` generated.
            *pool = AddressPool::new(
                pool.base_path.clone(),
                AddressPoolType::External,
                gap,
                network,
                &key_source,
            )
            .expect("rebuild CoinJoin External pool at chosen gap");
        }
    }
}

/// `(highest_used, highest_generated, confirmed)` of the CoinJoin
/// account-0 External pool from a read guard.
fn coinjoin_external_pool_state(
    state: &platform_wallet::wallet::platform_wallet::WalletStateReadGuard<'_>,
) -> Option<(Option<u32>, Option<u32>, u64)> {
    use key_wallet::account::AccountType as AT;
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

    for funds in state.core_wallet.accounts.all_funding_accounts() {
        if matches!(
            funds.managed_account_type().to_account_type(),
            AT::CoinJoin { .. }
        ) {
            let confirmed = funds.balance.confirmed();
            for pool in funds.managed_account_type().address_pools() {
                if pool.pool_type == AddressPoolType::External {
                    return Some((pool.highest_used, pool.highest_generated, confirmed));
                }
            }
        }
    }
    None
}

/// Ground-truth index↔height analysis for the CoinJoin External
/// keychain. Syncs a wallet that watches every used index from genesis,
/// extracts `h(i)` = first funding block height per used index, then runs
/// the inversion analysis and two discovery simulations over that data.
///
/// `#[ignore]` because it is a heavyweight (~8 min) diagnostic, not a
/// pass/fail gate; run it explicitly with `--ignored`. The pure
/// simulation logic is covered by unit tests below without any sync.
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 4)]
#[ignore = "heavyweight diagnostic: cold testnet sync + full index/height analysis; run with --ignored"]
async fn found_coinjoin_gap_limit_sync_height_analysis() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=info".into()),
        )
        .with_test_writer()
        .try_init();

    let network = Network::Testnet;
    let sdk = build_testnet_sdk(network);
    let manager = Arc::new(PlatformWalletManager::new(
        Arc::clone(&sdk),
        Arc::new(NoopPersister),
        vec![Arc::new(NoopEventHandler) as Arc<dyn PlatformEventHandler>],
    ));

    let mnemonic: Mnemonic = TEST_MNEMONIC.parse().expect("valid BIP-39 mnemonic");
    let seed = mnemonic.to_seed("");
    let mut coinjoin = std::collections::BTreeSet::new();
    coinjoin.insert(0u32);
    let mut bip44 = std::collections::BTreeSet::new();
    bip44.insert(0u32);
    let wallet = manager
        .create_wallet_from_seed_bytes(
            network,
            &seed,
            WalletAccountCreationOptions::AllAccounts(
                bip44,
                Default::default(),
                coinjoin,
                Default::default(),
                Default::default(),
            ),
            Some(0),
        )
        .await
        .expect("create ground-truth wallet");

    // Deep CoinJoin pre-derivation past highest-used so nothing is missed.
    pre_derive_wide(&wallet, COINJOIN_GROUND_TRUTH_DEPTH).await;
    sync_capped(&manager, network, &sdk, "GT").await;

    // h(i): first funding height per used CoinJoin External index.
    let mut by_index = coinjoin_external_first_funding_heights(&wallet).await;
    by_index.sort_unstable_by_key(|(i, _)| *i);

    let highest_used = by_index.last().map(|(i, _)| *i).unwrap_or(0);
    println!("\n=== CoinJoin External index↔height ground truth ===");
    println!(
        "used indices discovered: {}, highest_used index: {highest_used}",
        by_index.len()
    );
    assert!(
        highest_used >= 1727,
        "ground-truth wallet only reached index {highest_used} (< 1727); \
         increase COINJOIN_GROUND_TRUTH_DEPTH and re-run"
    );

    println!("\n--- (index, first_funding_height) sorted by INDEX ---");
    for (i, h) in &by_index {
        println!("  i={i:>5}  h={h}");
    }

    let mut by_height = by_index.clone();
    by_height.sort_by_key(|(i, h)| (*h, *i));
    println!("\n--- (index, first_funding_height) sorted by HEIGHT ---");
    for (i, h) in &by_height {
        println!("  h={h}  i={i}");
    }

    // Inversion analysis: walk ascending height; flag every tx whose
    // index exceeds the running discovery ceiling (gap-30 follow).
    println!("\n=== INVERSION analysis (ascending height, ceiling = highest_used+1+30) ===");
    let inversions = inversions(&by_index, DEFAULT_COINJOIN_GAP_LIMIT);
    if inversions.is_empty() {
        println!("  none — no index ever outran the gap-30 ceiling.");
    } else {
        for inv in &inversions {
            println!(
                "  height {} funded index {}, but ceiling was only {} (gap {})",
                inv.height, inv.index, inv.ceiling, inv.gap
            );
        }
        let first = &inversions[0];
        println!(
            "  ⇒ FIRST inversion (predicted stall): index {} at height {} vs ceiling {}",
            first.index, first.height, first.ceiling
        );
    }

    // Sim WINDOWED: block-atomic once-per-block apply with no effective
    // re-test (the AlreadyProcessed gate). Reproduces the stall at 59.
    let windowed = sim_windowed(
        &by_index,
        DEFAULT_COINJOIN_GAP_LIMIT,
        SPV_BACKWARD_RESCAN_BLOCKS,
    );
    println!("\n=== SIM WINDOWED (block-atomic once-per-block, models the real system) ===");
    println!(
        "  discovered {} of {} used indices; stall (highest discovered) = {}",
        windowed.discovered_count,
        by_index.len(),
        windowed.highest_discovered
    );

    // Sim FULL-RESCAN: on every ceiling extension, re-match the extended
    // watch set against the ENTIRE scanned range; iterate to fixpoint.
    let full = sim_full_rescan(&by_index, DEFAULT_COINJOIN_GAP_LIMIT);
    println!("\n=== SIM FULL-RESCAN (models the proposed fix) ===");
    println!(
        "  discovered {} of {} used indices; highest discovered = {}",
        full.discovered_count,
        by_index.len(),
        full.highest_discovered
    );

    // Minimum initial pre-derivation depth that lets the block-atomic
    // gap-follow reach the highest used index under the real model.
    let min_depth = min_initial_depth_windowed(
        &by_index,
        DEFAULT_COINJOIN_GAP_LIMIT,
        SPV_BACKWARD_RESCAN_BLOCKS,
    );
    println!("\n=== RECOVERY DEPTH ===");
    println!(
        "  minimum initial pre-derivation depth for the windowed model to reach \
         index {highest_used}: {min_depth}"
    );
}

/// Extract `(index, first_funding_height)` for every USED CoinJoin
/// External (account 0) address. Walks the account's full transaction
/// history (retained under `keep-finalized-transactions`), resolving each
/// `Received` output's address to its derivation index via the pool, and
/// keeps the minimum block height per index.
async fn coinjoin_external_first_funding_heights(
    wallet: &Arc<platform_wallet::PlatformWallet>,
) -> Vec<(u32, u32)> {
    use key_wallet::account::AccountType as AT;
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::managed_account::transaction_record::OutputRole;

    let state = wallet.state().await;
    let mut first_height: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();

    for funds in state.core_wallet.accounts.all_funding_accounts() {
        if !matches!(
            funds.managed_account_type().to_account_type(),
            AT::CoinJoin { .. }
        ) {
            continue;
        }
        // CoinJoin is single-pool (External). Snapshot the address→index
        // map once, then scan transactions.
        let Some(pool) = funds
            .managed_account_type()
            .address_pools()
            .into_iter()
            .find(|p| p.pool_type == AddressPoolType::External)
        else {
            continue;
        };

        for record in funds.transactions().values() {
            let Some(height) = record.height() else {
                continue;
            };
            for out in &record.output_details {
                if out.role != OutputRole::Received {
                    continue;
                }
                let Some(addr) = out.address.as_ref() else {
                    continue;
                };
                let Some(index) = pool.address_index(addr) else {
                    continue;
                };
                first_height
                    .entry(index)
                    .and_modify(|h| *h = (*h).min(height))
                    .or_insert(height);
            }
        }
    }

    first_height.into_iter().collect()
}

/// One detected inversion: a funding tx whose address index outran the
/// running discovery ceiling at the height it appeared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Inversion {
    height: u32,
    index: u32,
    ceiling: u32,
    gap: u32,
}

/// Walk `by_index` in ascending HEIGHT order with a gap-limit ceiling
/// (ceiling starts at `gap` = watching indices `0..gap`; each in-window
/// hit advances it to `highest_used_so_far + 1 + gap`). Returns every tx
/// whose index exceeds the ceiling at its height. Pure; `by_index` is
/// `(index, first_funding_height)` and need not be pre-sorted.
fn inversions(by_index: &[(u32, u32)], gap: u32) -> Vec<Inversion> {
    let mut by_height: Vec<(u32, u32)> = by_index.to_vec();
    by_height.sort_by_key(|(i, h)| (*h, *i));

    let mut ceiling = gap; // watching 0..gap (indices 0..=gap-1)
    let mut highest_used: Option<u32> = None;
    let mut out = Vec::new();
    for (index, height) in by_height {
        if index < ceiling {
            highest_used = Some(highest_used.map_or(index, |h| h.max(index)));
            if let Some(hu) = highest_used {
                ceiling = hu + 1 + gap;
            }
        } else {
            out.push(Inversion {
                height,
                index,
                ceiling,
                gap: index - ceiling + 1,
            });
        }
    }
    out
}

/// Outcome of a discovery simulation over `(index, height)` data.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SimResult {
    discovered_count: usize,
    highest_discovered: u32,
}

/// WINDOWED simulation — models the real block-atomic forward-only system
/// with the default initial watch depth (`gap`). See
/// [`sim_windowed_with_initial`].
fn sim_windowed(by_index: &[(u32, u32)], gap: u32, backward_blocks: u32) -> SimResult {
    sim_windowed_with_initial(by_index, gap, backward_blocks, gap)
}

/// FULL-RESCAN simulation — models the proposed fix. Every time the
/// ceiling extends, re-match newly-watched indices against the ENTIRE
/// scanned range (no eviction); iterate to fixpoint. With unlimited
/// backward reach the height ordering no longer matters: any used index
/// within the running ceiling is found, so the only thing that can stop
/// it is a true index gap `>= gap` with no used index in between.
fn sim_full_rescan(by_index: &[(u32, u32)], gap: u32) -> SimResult {
    let used: std::collections::BTreeSet<u32> = by_index.iter().map(|(i, _)| *i).collect();
    let mut ceiling = gap;
    loop {
        // Highest used index reachable under the current ceiling.
        let highest = used.range(..ceiling).next_back().copied();
        let next_ceiling = highest.map_or(ceiling, |h| h + 1 + gap);
        if next_ceiling <= ceiling {
            break;
        }
        ceiling = next_ceiling;
    }
    let highest_discovered = used.range(..ceiling).next_back().copied().unwrap_or(0);
    SimResult {
        discovered_count: used.range(..ceiling).count(),
        highest_discovered,
    }
}

/// Minimum initial pre-derivation depth `d` (watch indices `0..d` from
/// genesis) such that the block-atomic gap-follow reaches the highest
/// used index. Found by scanning candidate depths and returning the
/// smallest that discovers the full used set.
fn min_initial_depth_windowed(by_index: &[(u32, u32)], gap: u32, backward_blocks: u32) -> u32 {
    let target = by_index.iter().map(|(i, _)| *i).max().unwrap_or(0);
    let total = by_index.len();
    // Candidate depths: each used index + 1 is a meaningful boundary; the
    // answer is one of those (or the trivial `gap`). Scan ascending.
    let mut candidates: Vec<u32> = by_index.iter().map(|(i, _)| *i + 1).collect();
    candidates.push(gap);
    candidates.sort_unstable();
    candidates.dedup();
    for &d in &candidates {
        let r = sim_windowed_with_initial(by_index, gap, backward_blocks, d);
        if r.highest_discovered >= target && r.discovered_count == total {
            return d;
        }
    }
    target + 1
}

/// WINDOWED simulation with a configurable initial watch depth — the
/// faithful model of dash-spv once-per-block discovery.
///
/// Sweeps funding events forward in height order (the scan frontier =
/// current event height; it only increases). The watch ceiling starts at
/// `max(gap, initial_depth)` (indices `0..ceiling` watched from genesis).
///
/// **Block-atomic** — this is the key fidelity point. A block is applied
/// to the wallet once, against only the addresses generated at that
/// instant. Within a block, only indices below the ceiling *as it stood
/// before the block* are discovered; the gap window extends AFTER the
/// whole block is processed, and the newly-watched addresses apply only
/// to LATER blocks. In the real system the block's own `rescan_batch`
/// re-match would catch them, but the per-`(wallet, block)`
/// `BlockMatchTracker` returns `AlreadyProcessed` and skips re-applying
/// the block — so the net effect is no re-test of the committed block.
/// This reproduces the empirical stall at 59: CoinJoin packs dozens of
/// indices per block, so one gap-30 extension reaches at most ~30 new
/// indices into the next block and silently misses every index used only
/// in the just-applied block above the prior ceiling.
///
/// `backward_blocks` models the FIXED behaviour as a tunable: `0` = the
/// current gated system (no effective re-test); `N` = re-test the last
/// `N` processed blocks against the extended watch set, to fixpoint.
fn sim_windowed_with_initial(
    by_index: &[(u32, u32)],
    gap: u32,
    backward_blocks: u32,
    initial_depth: u32,
) -> SimResult {
    use std::collections::BTreeMap;

    // Group used indices by block height (ascending).
    let mut blocks: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &(index, height) in by_index {
        blocks.entry(height).or_default().push(index);
    }
    let block_list: Vec<Vec<u32>> = blocks.into_values().collect();

    let mut ceiling = gap.max(initial_depth);
    let mut highest_discovered: Option<u32> = None;
    let mut discovered: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();

    let extend =
        |disc: &std::collections::BTreeSet<u32>, ceiling: &mut u32, hd: &mut Option<u32>| {
            if let Some(&max) = disc.iter().next_back() {
                *hd = Some(max);
                let nc = max + 1 + gap;
                if nc > *ceiling {
                    *ceiling = nc;
                }
            }
        };

    for (bi, idxs) in block_list.iter().enumerate() {
        // Atomic match against the ceiling as it stood before this block.
        let watch_before = ceiling;
        for &idx in idxs {
            if idx < watch_before {
                discovered.insert(idx);
            }
        }
        extend(&discovered, &mut ceiling, &mut highest_discovered);

        // Optional backward re-scan of the last `backward_blocks` blocks
        // (including this one) against the extended watch set, to fixpoint.
        if backward_blocks > 0 {
            loop {
                let mut changed = false;
                let lo = (bi + 1).saturating_sub(backward_blocks as usize);
                for prev in &block_list[lo..=bi] {
                    for &idx in prev {
                        if idx < ceiling && discovered.insert(idx) {
                            changed = true;
                        }
                    }
                }
                if changed {
                    extend(&discovered, &mut ceiling, &mut highest_discovered);
                } else {
                    break;
                }
            }
        }
    }

    SimResult {
        discovered_count: discovered.len(),
        highest_discovered: highest_discovered.unwrap_or(0),
    }
}

#[cfg(test)]
mod sim_tests {
    use super::*;

    /// DENSE-BLOCK defeat — the real mechanism. Many contiguous indices
    /// packed into ONE block defeat once-per-block discovery: the block is
    /// applied once, matching only the pre-watched 0..gap; the ceiling
    /// extends to 2*gap-1 AFTER the block, but the indices used only in
    /// that just-applied block (gap..) are never re-applied (the
    /// `AlreadyProcessed` gate skips the block). Modelling a single
    /// backward block (the fixed behaviour — re-test the just-finished
    /// block once) recovers everything.
    #[test]
    fn windowed_stalls_on_dense_block_backward_one_recovers() {
        let gap = 30;
        // indices 0..=200 all funded in ONE block (height 1000).
        let by_index: Vec<(u32, u32)> = (0..=200u32).map(|i| (i, 1000)).collect();

        let w0 = sim_windowed(&by_index, gap, 0);
        // Pre-watched 0..30 match; ceiling → 59; nothing left to scan
        // (single block already committed) → highest_discovered = 29.
        assert_eq!(
            w0.highest_discovered, 29,
            "dense block stalls at gap-1: {w0:?}"
        );
        assert_eq!(w0.discovered_count, 30);

        let w1 = sim_windowed(&by_index, gap, 1);
        assert_eq!(
            w1.highest_discovered, 200,
            "one backward block recovers all: {w1:?}"
        );
        assert_eq!(w1.discovered_count, 201);
    }

    /// Two dense blocks reproduce the shape behind the empirical stall:
    /// block A uses 0..=51, block B uses 52..=139. With zero backward
    /// re-scan, A matches 0..30 (ceiling→59), B then matches 52..59
    /// (ceiling→89) — final highest_used = 59, exactly the observed value.
    #[test]
    fn windowed_reproduces_empirical_fifty_nine_stall_shape() {
        let gap = 30;
        let mut by_index: Vec<(u32, u32)> = (0..=51u32).map(|i| (i, 1000)).collect();
        by_index.extend((52..=139u32).map(|i| (i, 1001)));
        let w = sim_windowed(&by_index, gap, 0);
        assert_eq!(
            w.highest_discovered, 59,
            "two dense blocks stall at 59: {w:?}"
        );
    }

    /// FULL-RESCAN recovers all contiguous indices regardless of block
    /// packing/ordering; its only residual limit is a true index gap
    /// `>= gap`.
    #[test]
    fn full_rescan_recovers_dense_blocks_but_stalls_on_true_gap() {
        let gap = 30;
        // Dense, contiguous 0..=200 in one block → full rescan gets all.
        let dense: Vec<(u32, u32)> = (0..=200u32).map(|i| (i, 1000)).collect();
        let f = sim_full_rescan(&dense, gap);
        assert_eq!(
            f.discovered_count, 201,
            "full rescan recovers dense block: {f:?}"
        );
        assert_eq!(f.highest_discovered, 200);

        // 0..=20 then a 39-index gap to 60 → even full rescan stalls at 20.
        let mut gapped: Vec<(u32, u32)> = (0..=20u32).map(|i| (i, 100 + i)).collect();
        gapped.push((60, 50));
        let g = sim_full_rescan(&gapped, gap);
        assert_eq!(
            g.highest_discovered, 20,
            "true index gap stalls rescan: {g:?}"
        );
        assert_eq!(g.discovered_count, 21);
    }

    /// One index per block in forward height order is fully discovered —
    /// confirms the block-atomic model doesn't spuriously stall when each
    /// block introduces at most one new index within the gap window.
    #[test]
    fn windowed_recovers_one_index_per_block_forward() {
        let by_index: Vec<(u32, u32)> = (0..=100u32).map(|i| (i, 100 + i)).collect();
        let w = sim_windowed(&by_index, 30, 0);
        assert_eq!(w.discovered_count, 101);
        assert_eq!(w.highest_discovered, 100);
    }

    /// Inversion detector flags the first index that outruns the ceiling.
    #[test]
    fn inversions_flags_first_outrunner() {
        // index 50 appears (in height order) before the ceiling can cover
        // it — ceiling starts at 30, only 0..29 watched.
        let by_index = vec![(0u32, 10u32), (50u32, 11u32)];
        let inv = inversions(&by_index, 30);
        assert_eq!(inv.len(), 1);
        assert_eq!(inv[0].index, 50);
        assert_eq!(inv[0].ceiling, 31); // after discovering index 0: 0+1+30
    }
}
