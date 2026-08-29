//! Operational utility — derive and print the bank wallet's primary
//! Platform (L2) and Core (L1) receive addresses **offline**.
//!
//! Unlike [`super::print_bank_address`], this helper does NOT call
//! `setup()` and therefore does NOT construct the SDK / trusted-context
//! provider, sync the bank over the network, or run SPV. BIP-32/DIP-17
//! key derivation is fully deterministic and offline, so this works even
//! when the configured DAPI / trusted-context endpoints are unreachable
//! (e.g. the porter devnet `quorums.devnet.porter.networks.dash.org`
//! host does not resolve — see `tests/.env.example` `TODO(porter)`).
//!
//! It mirrors the exact derivation logic of
//! `framework::bank::derive_platform_address_at_index` (DIP-17
//! `PlatformPayment { account: 0, key_class: 0 }`, leaf 0) and
//! `framework::bank::derive_core_receive_address_at_index` (BIP-44
//! `Standard` account 0, external chain 0, leaf 0).
//!
//! ```text
//! cargo test --test e2e -- --ignored --nocapture print_bank_address_offline
//! ```

use crate::framework::config::Config;
use dpp::address_funds::PlatformAddress;
use dpp::util::hash::ripemd160_sha256;
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::mnemonic::Language;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::{AccountType, ChildNumber, Mnemonic, Wallet};

/// BIP-44 external (receive) chain selector — internal/change is `1`.
const BIP44_EXTERNAL_CHAIN: u32 = 0;
/// DIP-17 default PlatformPayment account / key-class (matches
/// `framework::wallet_factory::DEFAULT_PLATFORM_PAYMENT_ACCOUNT_SPEC`).
const DEFAULT_ACCOUNT_INDEX_PUB: u32 = 0;
const DEFAULT_KEY_CLASS_PUB: u32 = 0;

#[tokio_shared_rt::test(shared)]
#[ignore = "operational utility — offline bank-address derivation; run on demand"]
async fn print_bank_address_offline() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_test_writer()
        .try_init();

    // Reads `tests/.env` (mnemonic + network) but touches no network.
    let config = Config::from_env().expect("config load failed (need tests/.env)");
    let network = config.network;

    let mnemonic = Mnemonic::from_phrase(config.bank_mnemonic.trim(), Language::English)
        .expect("invalid BIP-39 bank mnemonic");

    let wallet = Wallet::from_mnemonic(mnemonic, network, WalletAccountCreationOptions::Default)
        .expect("offline wallet construction failed");

    // --- Platform (L2) primary receive address: DIP-17 index 0 ---
    let platform_path = AccountType::PlatformPayment {
        account: DEFAULT_ACCOUNT_INDEX_PUB,
        key_class: DEFAULT_KEY_CLASS_PUB,
    }
    .derivation_path(network)
    .expect("DIP-17 PlatformPayment account path");
    let platform_leaf =
        platform_path.extend([ChildNumber::from_normal_idx(0).expect("leaf index 0")]);
    let platform_pubkey = wallet
        .derive_public_key(&platform_leaf)
        .expect("derive platform pubkey");
    let pkh = ripemd160_sha256(&platform_pubkey.serialize());
    let platform_addr = PlatformAddress::P2pkh(pkh);
    let platform_bech32m = platform_addr.to_bech32m_string(network);

    // --- Core (L1) fallback receive address: BIP-44 ext chain 0, index 0 ---
    let core_account_path = AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    }
    .derivation_path(network)
    .expect("BIP-44 Standard account path");
    let core_leaf = core_account_path.extend([
        ChildNumber::from_normal_idx(BIP44_EXTERNAL_CHAIN).expect("external chain"),
        ChildNumber::from_normal_idx(0).expect("leaf index 0"),
    ]);
    let core_pubkey = wallet
        .derive_public_key(&core_leaf)
        .expect("derive core pubkey");
    let core_dash_pubkey = dashcore::PublicKey::new(core_pubkey);
    let core_addr = dashcore::Address::p2pkh(&core_dash_pubkey, network);

    eprintln!(
        "\n=== BANK PLATFORM ADDRESS (bech32m, L2) ===\n{platform_bech32m}\n===========================================\n"
    );
    eprintln!("\n=== BANK CORE ADDRESS (L1, asset-lock) ===\n{core_addr}\n==========================================\n");
    eprintln!("NETWORK={network}");
    println!("BANK_PLATFORM_ADDRESS={platform_bech32m}");
    println!("BANK_CORE_ADDRESS={core_addr}");
    println!("NETWORK={network}");
}
