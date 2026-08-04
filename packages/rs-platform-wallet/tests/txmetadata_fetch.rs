//! Testnet integration test for the encrypted `txMetadata` FETCH path
//! (dashpay/platform#4087). Runs the EXACT production query
//! ([`platform_wallet::query_owned_encrypted_documents`], the network half of
//! `IdentityWallet::fetch_encrypted_documents`) against a real testnet identity
//! that has two legacy-written encrypted `txMetadata` documents, and asserts the
//! query returns both with the expected `keyIndex` / `encryptionKeyIndex` /
//! `encryptedMetadata` fields.
//!
//! This pins the wire query so a regression in the where-clause / order-by /
//! encoding is caught by this check rather than only on-device. NOTE: the test
//! is `#[ignore]`d because it hits live testnet, so it is a MANUAL, testnet-
//! gated check — run it explicitly with `--ignored` (see below). It is NOT part
//! of the default `cargo test` / CI run, and no scheduled job runs `--ignored`
//! today; treat it as a local / pre-release regression gate. The
//! DECRYPT half is not exercised here — it needs the owner's mnemonic — but the
//! per-document field extraction that feeds decrypt IS asserted, proving the
//! pipeline reaches the decrypt step for both documents.
//!
//! # Running
//! ```bash
//! cargo test -p platform-wallet --test txmetadata_fetch -- --ignored --nocapture
//! ```
//! Requires outbound HTTPS to testnet DAPI nodes + the testnet quorum service
//! (`https://quorums.testnet.networks.dash.org`).

use std::num::NonZeroUsize;
use std::sync::Arc;

use dash_sdk::platform::Fetch;
use dash_sdk::SdkBuilder;
use dpp::document::DocumentV0Getters;
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::Value;
use dpp::prelude::{DataContract, Identifier};
use key_wallet::Network;
use platform_wallet::query_owned_encrypted_documents;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

/// Testnet identity that owns the two legacy-written encrypted `txMetadata`
/// documents (base58).
const OWNER_B58: &str = "532rVHxLD6Z3MNiu5LZyNqn55Ybz4bydZozXU4cqqp1L";
/// The wallet-utils system data contract (base58) — its `txMetadata` type.
const CONTRACT_B58: &str = "7CSFGeF4WNzgDmx94zwvHkYaG3Dx4XEe5LFsFgJswLbm";
const DOC_TYPE: &str = "txMetadata";

async fn testnet_sdk() -> Arc<dash_sdk::Sdk> {
    let provider =
        TrustedHttpContextProvider::new(Network::Testnet, None, NonZeroUsize::new(100).unwrap())
            .expect("trusted context provider");
    let sdk = SdkBuilder::new_testnet()
        .with_context_provider(provider)
        .build()
        .expect("build testnet sdk");
    Arc::new(sdk)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "hits testnet"]
async fn fetch_returns_both_legacy_txmetadata_documents() {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    let sdk = testnet_sdk().await;
    let owner = Identifier::from_string(OWNER_B58, Encoding::Base58).expect("owner id");
    let contract_id = Identifier::from_string(CONTRACT_B58, Encoding::Base58).expect("contract id");

    let contract = DataContract::fetch(&sdk, contract_id)
        .await
        .expect("fetch contract")
        .expect("wallet-utils contract present on testnet");
    // Production parity (`IdentityWallet::fetch_encrypted_documents`):
    // register the fetched contract with the trusted context provider before
    // the query, exactly as the on-device path does. With this line the repro
    // is config-identical to the device call: `SdkBuilder::new_testnet()` +
    // `TrustedHttpContextProvider::new(Testnet, None, 100)`, proofs on
    // (builder default), platform version auto (0), since_ms = 0.
    {
        use dash_sdk::platform::ContextProvider;
        if let Some(provider) = sdk.context_provider() {
            provider.register_data_contract(Arc::new(contract.clone()));
        }
    }
    let contract = Arc::new(contract);

    // The exact production query (since_ms = 0 => fetch everything, as the
    // decrypt-proof probe does).
    let docs = query_owned_encrypted_documents(&sdk, Arc::clone(&contract), &owner, DOC_TYPE, 0)
        .await
        .expect("query owned encrypted documents");

    let materialized: Vec<_> = docs.iter().filter_map(|(_, d)| d.as_ref()).collect();
    assert_eq!(
        materialized.len(),
        2,
        "expected 2 legacy-written txMetadata documents for {OWNER_B58}, got {} (raw entries: {})",
        materialized.len(),
        docs.len()
    );

    // Every document must expose the fields the decrypt step consumes:
    // integer keyIndex/encryptionKeyIndex and a byte-array encryptedMetadata.
    for doc in materialized {
        let key_index = doc
            .properties()
            .get("keyIndex")
            .and_then(|v: &Value| v.to_integer::<u32>().ok())
            .expect("keyIndex is a u32");
        let encryption_key_index = doc
            .properties()
            .get("encryptionKeyIndex")
            .and_then(|v: &Value| v.to_integer::<u32>().ok())
            .expect("encryptionKeyIndex is a u32");
        let encrypted_len = doc
            .properties()
            .get("encryptedMetadata")
            .and_then(|v: &Value| v.to_binary_bytes().ok())
            .map(|b| b.len())
            .expect("encryptedMetadata is a byte array");

        // These identities' documents were written by the Android wallet with
        // the ENCRYPTION/MEDIUM key (id 2); the blob is version(1)+IV(16)+CBC.
        assert_eq!(key_index, 2, "keyIndex should be the ENCRYPTION key id");
        assert!(encryption_key_index >= 1, "encryptionKeyIndex is 1-based");
        assert!(
            encrypted_len > 1 + 16,
            "encryptedMetadata must exceed the version+IV header ({encrypted_len} bytes)"
        );
    }
}

/// Independent legacy-install capture SCAFFOLDING (dashpay/platform#4186,
/// reviewer shumkov's "decrypt a blob produced by a real legacy dash-wallet
/// install" ask). This is a MANUAL, testnet-gated helper — it resolves the
/// throwaway wallet's DPNS name `yabba2` to its identity id, fetches every
/// `txMetadata` document that identity owns, derives the tx-metadata key from
/// the wallet's recovery phrase with THIS branch's own derivation
/// (`derive_tx_metadata_key`, identity_index 0), opens each blob, and prints the
/// blob hex + key indices + decrypted plaintext so the captured values can be
/// hard-coded into the network-free fixture
/// `legacy_install_yabba2_wire_compat_vector` in
/// `src/wallet/identity/crypto/tx_metadata.rs`.
///
/// The wallet is a DESIGNATED THROWAWAY provided for this fixture; its recovery
/// phrase is intended to become public in the repo.
///
/// # Running
/// ```bash
/// cargo test -p platform-wallet --test txmetadata_fetch \
///     capture_legacy_yabba2 -- --ignored --nocapture
/// ```
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "hits testnet"]
async fn capture_legacy_yabba2_txmetadata_blobs() {
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    use platform_wallet::wallet::identity::crypto::tx_metadata::{
        derive_tx_metadata_key, open_tx_metadata,
    };

    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    // The DESIGNATED THROWAWAY testnet wallet the legacy dash-wallet 11.9 install
    // ran under (public by design for this fixture).
    const YABBA2_PHRASE: &str =
        "across jungle only rocket promote mule behave siren crush pole awful deposit";
    const DPNS_NAME: &str = "yabba2";

    let sdk = testnet_sdk().await;

    // 1. Resolve the DPNS name -> identity id (the SDK's own resolver).
    let owner = sdk
        .resolve_dpns_name(DPNS_NAME)
        .await
        .expect("resolve_dpns_name call")
        .expect("DPNS name yabba2 resolves to an identity");
    println!(
        "RESOLVED yabba2 -> identity {}",
        owner.to_string(Encoding::Base58)
    );

    // 2. Fetch + register the wallet-utils contract (production parity).
    let contract_id = Identifier::from_string(CONTRACT_B58, Encoding::Base58).expect("contract id");
    let contract = DataContract::fetch(&sdk, contract_id)
        .await
        .expect("fetch contract")
        .expect("wallet-utils contract present on testnet");
    {
        use dash_sdk::platform::ContextProvider;
        if let Some(provider) = sdk.context_provider() {
            provider.register_data_contract(Arc::new(contract.clone()));
        }
    }
    let contract = Arc::new(contract);

    // 3. The exact production query (since_ms = 0 => fetch everything).
    let docs = query_owned_encrypted_documents(&sdk, Arc::clone(&contract), &owner, DOC_TYPE, 0)
        .await
        .expect("query owned encrypted documents");
    let materialized: Vec<_> = docs.iter().filter_map(|(_, d)| d.as_ref()).collect();
    println!(
        "FETCHED {} txMetadata document(s) (raw entries: {}) for identity {}",
        materialized.len(),
        docs.len(),
        owner.to_string(Encoding::Base58)
    );
    if materialized.is_empty() {
        println!(
            "ZERO documents. Queried contract={CONTRACT_B58} type={DOC_TYPE} owner={} since_ms=0",
            owner.to_string(Encoding::Base58)
        );
        return;
    }

    // 4. Derive keys from the wallet's recovery phrase with the branch's own
    //    derivation (identity_index 0 — the only slot a legacy wallet writes).
    let wallet = Wallet::from_mnemonic(
        Mnemonic::from_phrase(YABBA2_PHRASE, Language::English).expect("valid recovery phrase"),
        Network::Testnet,
        WalletAccountCreationOptions::None,
    )
    .expect("wallet from recovery phrase");

    // 5. Decrypt each document with the new Rust open path and print the capture.
    for (i, doc) in materialized.iter().enumerate() {
        let props = doc.properties();
        let key_index = props
            .get("keyIndex")
            .and_then(|v: &Value| v.to_integer::<u32>().ok())
            .expect("keyIndex is a u32");
        let encryption_key_index = props
            .get("encryptionKeyIndex")
            .and_then(|v: &Value| v.to_integer::<u32>().ok())
            .expect("encryptionKeyIndex is a u32");
        let blob = props
            .get("encryptedMetadata")
            .and_then(|v: &Value| v.to_binary_bytes().ok())
            .expect("encryptedMetadata is a byte array");
        let created_at = doc.created_at();
        let updated_at = doc.updated_at();

        let aes_key = derive_tx_metadata_key(
            &wallet,
            Network::Testnet,
            0,
            key_index,
            encryption_key_index,
        )
        .expect("derive txMetadata key at identity_index 0");
        let opened = open_tx_metadata(&aes_key, &blob).expect("open legacy blob");

        println!("---- DOCUMENT {i} ----");
        println!("keyIndex            = {key_index}");
        println!("encryptionKeyIndex  = {encryption_key_index}");
        println!("createdAt           = {created_at:?}");
        println!("updatedAt           = {updated_at:?}");
        println!("blob_len            = {}", blob.len());
        println!("BLOB_HEX            = {}", hex::encode(&blob));
        println!("version             = {}", opened.version);
        println!("plaintext_len       = {}", opened.payload.len());
        println!("PLAINTEXT_HEX       = {}", hex::encode(&opened.payload));
        println!(
            "PLAINTEXT_UTF8_LOSSY= {}",
            String::from_utf8_lossy(&opened.payload)
        );
    }
}
