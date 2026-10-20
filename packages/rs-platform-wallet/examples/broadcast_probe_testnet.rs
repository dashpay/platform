//! Manual testnet check of the broadcast probe against real evonodes.
//!
//! Hands each transaction (raw hex, one per argument) to
//! [`DapiAcceptanceProbe`] and prints the verdict. Run with node-level detail:
//!
//! ```text
//! RUST_LOG=platform_wallet::broadcast_probe=debug \
//!   cargo run -p platform-wallet --example broadcast_probe_testnet -- <hex> [<hex> ...]
//! ```
//!
//! Resubmitting a transaction is harmless — a node either already has it,
//! takes it, or says why it will not — so any transaction may be probed.

use std::sync::Arc;

use dash_sdk::SdkBuilder;
use dashcore::{consensus, Network, Transaction};
use platform_wallet::broadcast_probe::{AcceptanceProbe, DapiAcceptanceProbe};
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let transactions: Vec<Transaction> = std::env::args()
        .skip(1)
        .map(|raw| consensus::deserialize(&hex::decode(raw.trim()).expect("hex")).expect("tx"))
        .collect();
    if transactions.is_empty() {
        return Err("pass at least one raw transaction as hex".into());
    }

    let provider = TrustedHttpContextProvider::new(
        Network::Testnet,
        None,
        std::num::NonZeroUsize::new(100).expect("non-zero"),
    )?;
    let sdk = Arc::new(
        SdkBuilder::new_testnet()
            .with_context_provider(provider)
            .build()?,
    );
    let probe = DapiAcceptanceProbe::new(sdk);

    for transaction in &transactions {
        let verdict = probe.probe(transaction).await;
        println!("{}  {verdict:?}", transaction.txid());
    }
    Ok(())
}
