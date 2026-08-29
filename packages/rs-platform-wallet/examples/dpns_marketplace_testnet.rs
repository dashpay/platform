//! Manual TESTNET verification harness for the DPNS username-marketplace
//! wallet layer (`wallet/identity/network/dpns_marketplace.rs`).
//!
//! Exercises the real wallet-level flow end to end against testnet DAPI:
//! register (uncontested) name → list → verify sale state → re-price →
//! typed stale-price rejection → purchase by a second identity →
//! ownership/records/label reconciliation → history timeline (priceSet ×2 +
//! purchased) → re-list → delist via transfer-to-self → `$price` cleared.
//! Use the printed results to validate the active testnet contract and
//! transition behavior before shipping SDK changes.
//!
//! Environment (secrets stay in env, never printed):
//!   DPNS_MNEMONIC       required — wallet recovery phrase
//!   DPNS_PHASE          "discover" (default) or "run"
//!   DPNS_SELLER_INDEX   HD identity index of the seller (default 0)
//!   DPNS_BUYER_INDEX    HD identity index of the buyer  (default 1)
//!   DPNS_IDENTITY_ID    optional base58 id: discover also reports which HD
//!                       index (0..=9) derives this identity's keys, or that
//!                       none does (out-of-wallet key layout)
//!   DPNS_PRIVATE_KEY    optional single signing key (hex or WIF) fallback
//!                       when the identity's keys are not HD-derived; used
//!                       with DPNS_IDENTITY_ID
//!   DPNS_DAPI_ADDRESSES optional comma-separated https://host:port list
//!
//! Run:
//!   DPNS_MNEMONIC="…" cargo run -p platform-wallet --example dpns_marketplace_testnet
//!   DPNS_PHASE=run DPNS_MNEMONIC="…" cargo run -p platform-wallet --example dpns_marketplace_testnet

use std::sync::Arc;

use dash_sdk::sdk::{Address, AddressList};
use dash_sdk::SdkBuilder;
use dashcore::hashes::{hash160, Hash};
use dashcore::Network;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, KeyType};
use dpp::prelude::Identifier;
use key_wallet::bip32::ExtendedPrivKey;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use platform_wallet::changeset::{
    ClientStartState, DpnsNameSaleStatus, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::error::PlatformWalletError;
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::identity::network::{
    derive_ecdsa_identity_auth_keypair_from_master, DpnsNameHistoryEventKind, IdentityWallet,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use simple_signer::signer::SimpleSigner;

/// Testnet DAPI evonodes (same set as `tests/spv_sync.rs`); override
/// with `DPNS_DAPI_ADDRESSES`.
const TESTNET_DAPI_ADDRESSES: &[&str] = &[
    "https://68.67.122.1:1443",
    "https://68.67.122.2:1443",
    "https://68.67.122.3:1443",
];

/// Listing prices for the flow (credits). Small on purpose — the point
/// is the protocol semantics, not the amounts.
const PRICE_INITIAL: Credits = 1_000_000;
const PRICE_FINAL: Credits = 2_000_000;
const PRICE_RELIST: Credits = 3_000_000;
/// Buyer top-up floor: purchase price + the wallet's fee reserve with
/// headroom for the buyer's own later transitions (re-list + delist).
const BUYER_MIN_CREDITS: Credits = 500_000_000;
const BUYER_TOP_UP: Credits = 1_000_000_000;

struct NoopPersister;
impl PlatformWalletPersistence for NoopPersister {
    fn store(
        &self,
        _wallet_id: WalletId,
        _changeset: PlatformWalletChangeSet,
    ) -> Result<(), platform_wallet::changeset::PersistenceError> {
        Ok(())
    }
    fn load(&self) -> Result<ClientStartState, platform_wallet::changeset::PersistenceError> {
        Ok(ClientStartState::default())
    }
    fn flush(
        &self,
        _wallet_id: WalletId,
    ) -> Result<(), platform_wallet::changeset::PersistenceError> {
        Ok(())
    }
}

struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

fn dapi_addresses() -> AddressList {
    let raw = std::env::var("DPNS_DAPI_ADDRESSES").unwrap_or_default();
    let addrs: Vec<Address> = if raw.trim().is_empty() {
        TESTNET_DAPI_ADDRESSES
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect()
    } else {
        raw.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect()
    };
    assert!(!addrs.is_empty(), "no DAPI addresses configured");
    AddressList::from_iter(addrs)
}

/// Whether `sk_bytes` is the private key for `key` (33-byte pubkey for
/// ECDSA_SECP256K1, hash160 for ECDSA_HASH160).
fn private_key_matches(key: &dpp::identity::IdentityPublicKey, sk_bytes: &[u8; 32]) -> bool {
    let secp = dashcore::secp256k1::Secp256k1::new();
    let Ok(sk) = dashcore::secp256k1::SecretKey::from_byte_array(sk_bytes) else {
        return false;
    };
    let pubkey = dashcore::secp256k1::PublicKey::from_secret_key(&secp, &sk).serialize();
    match key.key_type() {
        KeyType::ECDSA_SECP256K1 => key.data().as_slice() == pubkey.as_slice(),
        KeyType::ECDSA_HASH160 => {
            key.data().as_slice() == hash160::Hash::hash(&pubkey).as_byte_array().as_slice()
        }
        _ => false,
    }
}

/// Load every ECDSA key of `identity` (HD index `identity_index`,
/// derivation convention `key_index == key_id`) into `signer`, verifying
/// each derived pubkey against the on-chain key before insertion.
/// Returns how many keys matched.
fn load_hd_keys_into_signer(
    signer: &mut SimpleSigner,
    identity: &Identity,
    identity_index: u32,
    master: &ExtendedPrivKey,
) -> u32 {
    let mut matched = 0;
    for (key_id, ipk) in identity.public_keys() {
        if !matches!(
            ipk.key_type(),
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160
        ) {
            continue;
        }
        let Ok(kp) = derive_ecdsa_identity_auth_keypair_from_master(
            master,
            key_wallet::Network::Testnet,
            identity_index,
            *key_id,
        ) else {
            continue;
        };
        if private_key_matches(ipk, &kp.private_key) {
            signer.add_identity_public_key(ipk.clone(), *kp.private_key);
            matched += 1;
        }
    }
    matched
}

fn parse_private_key(raw: &str) -> Option<[u8; 32]> {
    let trimmed = raw.trim();
    if let Ok(bytes) = hex::decode(trimmed) {
        if bytes.len() == 32 {
            let mut out = [0u8; 32];
            out.copy_from_slice(&bytes);
            return Some(out);
        }
    }
    dashcore::PrivateKey::from_wif(trimmed)
        .ok()
        .map(|pk| pk.inner.secret_bytes())
}

async fn discover(
    idw: &IdentityWallet,
    master: &ExtendedPrivKey,
    sdk: &Arc<dash_sdk::Sdk>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("== discover: HD identities (index 0..=9) ==");
    for index in 0..10u32 {
        match idw.load_identity_by_index_from_master(index, master).await {
            Ok(Some(identity)) => {
                let key_summary: Vec<String> = identity
                    .public_keys()
                    .iter()
                    .map(|(id, k)| {
                        format!(
                            "#{id}:{:?}/{:?}/{:?}",
                            k.purpose(),
                            k.security_level(),
                            k.key_type()
                        )
                    })
                    .collect();
                println!(
                    "index {index}: {} balance={} keys=[{}]",
                    identity.id(),
                    identity.balance(),
                    key_summary.join(", ")
                );
            }
            Ok(None) => println!("index {index}: (none)"),
            Err(e) => println!("index {index}: lookup error: {e}"),
        }
    }

    if let Ok(raw_id) = std::env::var("DPNS_IDENTITY_ID") {
        use dash_sdk::platform::Fetch;
        let id = Identifier::from_string(
            raw_id.trim(),
            dpp::platform_value::string_encoding::Encoding::Base58,
        )?;
        println!("== discover: key layout of {id} ==");
        let Some(identity) = Identity::fetch(sdk.as_ref(), id).await? else {
            println!("identity not found on testnet");
            return Ok(());
        };
        println!(
            "balance={} keys={}",
            identity.balance(),
            identity.public_keys().len()
        );
        let mut any = false;
        for index in 0..10u32 {
            let mut probe = SimpleSigner::default();
            let matched = load_hd_keys_into_signer(&mut probe, &identity, index, master);
            if matched > 0 {
                println!("HD index {index}: {matched} key(s) derive from this mnemonic");
                any = true;
            }
        }
        if !any {
            println!("no key on this identity derives from the mnemonic (indexes 0..=9)");
        }
        if let Ok(raw_sk) = std::env::var("DPNS_PRIVATE_KEY") {
            match parse_private_key(&raw_sk) {
                Some(sk) => {
                    let matches: Vec<String> = identity
                        .public_keys()
                        .iter()
                        .filter(|(_, k)| private_key_matches(k, &sk))
                        .map(|(kid, k)| {
                            format!("#{kid} ({:?}/{:?})", k.purpose(), k.security_level())
                        })
                        .collect();
                    println!(
                        "DPNS_PRIVATE_KEY matches keys: [{}]",
                        if matches.is_empty() {
                            "none".to_string()
                        } else {
                            matches.join(", ")
                        }
                    );
                }
                None => println!("DPNS_PRIVATE_KEY did not parse as hex or WIF"),
            }
        }
    }
    Ok(())
}

/// Re-read `label`'s on-chain state until `predicate` holds or attempts
/// run out. Fresh reads race lagging replicas — a query right after a
/// broadcast can land on a node one block behind, briefly serving the
/// pre-transition document. The proof-verified CONFIRMED document from
/// the transition is the authoritative check; these visibility re-reads
/// are the "and other clients can see it" bonus, so they tolerate
/// replica lag with a bounded retry.
async fn wait_for_visible_state(
    idw: &IdentityWallet,
    label: &str,
    predicate: impl Fn(&platform_wallet::wallet::identity::network::DpnsDomainState) -> bool,
) -> Result<platform_wallet::wallet::identity::network::DpnsDomainState, Box<dyn std::error::Error>>
{
    let mut last = None;
    for _ in 0..6 {
        if let Some(state) = idw.dpns_name_state(label).await? {
            if predicate(&state) {
                return Ok(state);
            }
            last = Some(state);
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    Ok(last.ok_or("name never became visible")?)
}

/// Assert helper that prints a PASS line (the run transcript is the
/// verification artifact).
fn check(name: &str, ok: bool, detail: impl std::fmt::Display) {
    if ok {
        println!("PASS  {name}: {detail}");
    } else {
        println!("FAIL  {name}: {detail}");
        panic!("verification step failed: {name}");
    }
}

#[allow(clippy::too_many_lines)]
async fn run_flow(
    idw: &IdentityWallet,
    master: &ExtendedPrivKey,
) -> Result<(), Box<dyn std::error::Error>> {
    let seller_index: u32 = std::env::var("DPNS_SELLER_INDEX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let buyer_index: u32 = std::env::var("DPNS_BUYER_INDEX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    // `_from_master`: the manager-created wallet is external-signable
    // (no seed retained Rust-side), so the plain by-index loader cannot
    // derive the lookup key hash; the master-xpriv variant exists for
    // exactly this shape.
    let seller = idw
        .load_identity_by_index_from_master(seller_index, master)
        .await?
        .ok_or_else(|| format!("no identity at seller index {seller_index}"))?;
    let buyer = idw
        .load_identity_by_index_from_master(buyer_index, master)
        .await?
        .ok_or_else(|| format!("no identity at buyer index {buyer_index}"))?;
    let seller_id = seller.id();
    let buyer_id = buyer.id();
    println!(
        "seller (index {seller_index}): {seller_id} balance={}",
        seller.balance()
    );
    println!(
        "buyer  (index {buyer_index}): {buyer_id} balance={}",
        buyer.balance()
    );

    let mut signer = SimpleSigner::default();
    let seller_keys = load_hd_keys_into_signer(&mut signer, &seller, seller_index, master);
    let buyer_keys = load_hd_keys_into_signer(&mut signer, &buyer, buyer_index, master);
    check(
        "signer-keys",
        seller_keys > 0 && buyer_keys > 0,
        format!("seller {seller_keys} key(s), buyer {buyer_keys} key(s) HD-derived"),
    );

    // Buyer must afford price + fee reserve (plus its own later
    // transitions); top up from the seller when short.
    if buyer.balance() < BUYER_MIN_CREDITS {
        println!(
            "buyer balance {} < {BUYER_MIN_CREDITS}, transferring {BUYER_TOP_UP} credits from seller",
            buyer.balance()
        );
        idw.transfer_credits_with_external_signer(
            &seller_id,
            &buyer_id,
            BUYER_TOP_UP,
            &signer,
            None,
        )
        .await?;
        idw.refresh_identity(&buyer_id).await?;
    }

    // Fresh uncontested label per run: contains digits (timestamp) so it
    // never enters a masternode vote.
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let label = format!("mktp{unix}test");
    println!("== registering test name {label:?} on seller ==");
    let full_name = idw
        .register_name_with_external_signer(&seller_id, &label, &signer)
        .await?;
    check("register", full_name.ends_with(".dash"), &full_name);

    // 1. List.
    let listed = idw
        .set_dpns_name_price(&seller_id, &label, PRICE_INITIAL, &signer)
        .await?;
    check(
        "list",
        listed.price == Some(PRICE_INITIAL),
        format!("confirmed $price={:?}", listed.price),
    );
    let fresh = wait_for_visible_state(idw, &label, |s| {
        s.price == Some(PRICE_INITIAL) && s.owner_id == seller_id
    })
    .await?;
    check(
        "list-visible",
        fresh.price == Some(PRICE_INITIAL) && fresh.owner_id == seller_id,
        format!("on-chain $price={:?} owner={}", fresh.price, fresh.owner_id),
    );

    // 2. Re-price.
    let repriced = idw
        .set_dpns_name_price(&seller_id, &label, PRICE_FINAL, &signer)
        .await?;
    check(
        "re-price",
        repriced.price == Some(PRICE_FINAL),
        format!("confirmed $price={:?}", repriced.price),
    );

    // 3. Typed stale-price rejection (pre-flight, before any broadcast).
    let stale = idw
        .purchase_dpns_name(&buyer_id, &label, PRICE_INITIAL, &signer)
        .await;
    check(
        "stale-price-typed",
        matches!(
            stale,
            Err(PlatformWalletError::DocumentPriceChanged { expected, actual, .. })
                if expected == PRICE_INITIAL && actual == PRICE_FINAL
        ),
        format!("{stale:?}"),
    );

    // 4. Purchase at the confirmed price.
    idw.refresh_identity(&buyer_id).await?;
    let bought = idw
        .purchase_dpns_name(&buyer_id, &label, PRICE_FINAL, &signer)
        .await?;
    check(
        "purchase-owner",
        bought.owner_id == buyer_id,
        format!("owner={}", bought.owner_id),
    );
    check(
        "purchase-clears-price",
        bought.price.is_none(),
        format!("$price={:?}", bought.price),
    );
    check(
        "purchase-rewrites-records",
        bought.records_identity_id == Some(buyer_id),
        format!("records.identity={:?}", bought.records_identity_id),
    );

    // 5. Local reconciliation: label moved seller → buyer; marketplace row
    //    tracks the buyer as Owned.
    let rows = idw.local_dpns_name_states(None).await?;
    let row = rows
        .iter()
        .find(|r| r.normalized_label == bought.normalized_label)
        .expect("marketplace row for purchased name");
    check(
        "local-row",
        row.wallet_identity_id == buyer_id && row.status == DpnsNameSaleStatus::Owned,
        format!(
            "row identity={} status={:?}",
            row.wallet_identity_id, row.status
        ),
    );

    // 6. History: Registered + PriceSet(1M) + PriceSet(2M) + Purchased(2M).
    let history = idw.dpns_name_history(&label).await?;
    println!("history ({} events):", history.len());
    for event in &history {
        println!("  {:?}", event);
    }
    let price_sets: Vec<Credits> = history
        .iter()
        .filter_map(|e| match e.kind {
            DpnsNameHistoryEventKind::PriceSet { price } => Some(price),
            _ => None,
        })
        .collect();
    let purchased = history.iter().any(|e| {
        matches!(
            e.kind,
            DpnsNameHistoryEventKind::Purchased { price, seller, buyer }
                if price == PRICE_FINAL && seller == seller_id && buyer == buyer_id
        )
    });
    check(
        "history-price-sets",
        price_sets == vec![PRICE_INITIAL, PRICE_FINAL],
        format!("{price_sets:?}"),
    );
    check(
        "history-purchase",
        purchased,
        "purchase event with price+parties",
    );
    check(
        "history-registered",
        matches!(
            history.first().map(|e| &e.kind),
            Some(DpnsNameHistoryEventKind::Registered)
        ),
        "timeline starts at registration",
    );

    // 7. Typed not-for-sale rejection now that the purchase cleared $price.
    let not_for_sale = idw
        .purchase_dpns_name(&seller_id, &label, PRICE_FINAL, &signer)
        .await;
    check(
        "not-for-sale-typed",
        matches!(
            not_for_sale,
            Err(PlatformWalletError::DocumentNotForSale { .. })
        ),
        format!("{not_for_sale:?}"),
    );

    // 8. Delist: buyer re-lists, then delists via transfer-to-self; the
    //    method itself verifies the confirmed document cleared $price.
    idw.set_dpns_name_price(&buyer_id, &label, PRICE_RELIST, &signer)
        .await?;
    let delisted = idw.delist_dpns_name(&buyer_id, &label, &signer).await?;
    check(
        "delist-clears-price",
        delisted.price.is_none() && delisted.owner_id == buyer_id,
        format!("$price={:?} owner={}", delisted.price, delisted.owner_id),
    );
    let fresh =
        wait_for_visible_state(idw, &label, |s| s.price.is_none() && s.owner_id == buyer_id)
            .await?;
    check(
        "delist-visible",
        fresh.price.is_none() && fresh.owner_id == buyer_id,
        format!("on-chain $price={:?} owner={}", fresh.price, fresh.owner_id),
    );

    // 9. Search + sync passes for completeness.
    let results = idw
        .search_dpns_names_with_state("mktp", Some(50), None)
        .await?;
    check(
        "search",
        results
            .iter()
            .any(|s| s.normalized_label == fresh.normalized_label),
        format!("{} result(s) for prefix", results.len()),
    );
    let summary = idw.sync_dpns_marketplace().await?;
    println!(
        "sync summary: tracked={} added={:?} departed={} prices_changed={}",
        summary.names_tracked,
        summary.names_added.len(),
        summary.names_departed.len(),
        summary.prices_changed.len()
    );

    println!("== ALL CHECKS PASSED ==");
    Ok(())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let phrase = std::env::var("DPNS_MNEMONIC")
        .map_err(|_| "DPNS_MNEMONIC env var is required (never printed)")?;

    let addresses = dapi_addresses();
    let provider = TrustedHttpContextProvider::new(
        Network::Testnet,
        None,
        std::num::NonZeroUsize::new(100).unwrap(),
    )?;
    let sdk = Arc::new(
        SdkBuilder::new(addresses)
            .with_network(Network::Testnet)
            .with_context_provider(provider)
            .build()?,
    );

    let manager = Arc::new(PlatformWalletManager::new(
        Arc::clone(&sdk),
        Arc::new(NoopPersister),
        vec![Arc::new(NoopEventHandler)],
    ));
    let wallet = manager
        .create_wallet_from_mnemonic(
            &phrase,
            Network::Testnet,
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await?;
    let idw = wallet.identity();

    let mnemonic: key_wallet::Mnemonic = phrase.parse()?;
    let master = ExtendedPrivKey::new_master(key_wallet::Network::Testnet, &mnemonic.to_seed(""))?;

    match std::env::var("DPNS_PHASE").as_deref() {
        Ok("run") => run_flow(idw, &master).await,
        _ => discover(idw, &master, &sdk).await,
    }
}

fn main() {
    let _ = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .try_init();

    // 8 MiB worker stacks: every marketplace op verifies GroveDB
    // document-query proofs, whose recursion overflows the 2 MiB tokio
    // default (same rationale as DASHPAY_SYNC_STACK_BYTES).
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()
        .expect("build runtime")
        .block_on(run())
        .expect("verification run failed");
}
