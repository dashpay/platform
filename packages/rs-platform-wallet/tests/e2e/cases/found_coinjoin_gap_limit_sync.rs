//! Found — CoinJoin funds invisible to a default wallet because SPV
//! compact-filter discovery is forward-only within the active batch
//! window.
//!
//! Reproduction (diagnose-only) for the report: a testnet wallet with
//! deep CoinJoin usage does not fully sync under default settings — most
//! CoinJoin funds stay invisible.
//!
//! ## Root cause (NOT the gap limit)
//!
//! The CoinJoin gap limit (`DEFAULT_COINJOIN_GAP_LIMIT` = 30) is ample.
//! On this wallet the CoinJoin External keychain is used *densely and
//! effectively contiguously* in address index — up to index 1727, with
//! the largest unused run only 12 and no unused run >= 30 anywhere (see
//! the gap table the test prints). Gap size is not the problem.
//!
//! The defect is in dash-spv historical discovery: the compact-filter
//! rescan is **forward-only within the active batch window**
//! (~`MAX_LOOKAHEAD_BATCHES` (3) × `BATCH_PROCESSING_SIZE` (5000) ≈
//! 15 000 blocks). Addresses derived mid-scan to maintain the gap window
//! (`maintain_gap_limit`) are only re-matched against batches still in
//! the live window; already-committed batches are evicted and never
//! revisited. So discovery has a dependency on the *index* axis — to
//! learn index `N + gap` you must first match the block that used index
//! `N` — while the rescan progresses on the *height/batch* axis. Those
//! axes are orthogonal. CoinJoin first-use blocks are scattered across
//! years of history, so as soon as the block that first uses the next
//! index falls outside the live window, discovery snaps shut. Wallet A
//! advances exactly one gap step (highest_used=59, highest_generated=89)
//! and then stalls, missing 1578 of 1638 used CoinJoin indices.
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
//! Wallet A's forward-only discovery never reaches — `balance_B >
//! balance_A`.
//!
//! ## Non-determinism of the delta
//!
//! The observed delta varies run to run (447M / 739M / 2022M duffs seen)
//! because Wallet A's outcome depends on a race between gap-limit
//! derivation latency and batch commit/eviction — a derived address can
//! win or lose against the eviction of its using-block's batch. The test
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
            seed,
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
            seed,
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
