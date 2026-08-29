//! Operational utility — print the bank wallet's primary receive address.
//!
//! Run on demand when you need to top up the bank:
//! ```
//! cargo test --test e2e -- --ignored --nocapture print_bank_primary_address
//! ```

use crate::framework::prelude::*;

#[tokio_shared_rt::test(shared)]
async fn print_bank_primary_address() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_test_writer()
        .try_init();
    let s = setup().await.expect("e2e setup failed");
    let bank = s.ctx.bank();
    let network = bank.network();
    let addr_bech32m = bank.primary_receive_address().to_bech32m_string(network);
    let core_addr = bank
        .primary_core_receive_address()
        .await
        .expect("failed to derive Core receive address");
    let total_credits = bank.total_credits().await;
    eprintln!(
        "\n=== BANK PLATFORM ADDRESS (bech32m) ===\n{addr_bech32m}\n=======================================\n"
    );
    eprintln!(
        "\n=== BANK CORE FALLBACK ADDRESS ===\n{core_addr}\n==================================\n"
    );
    eprintln!("BANK_TOTAL_CREDITS={total_credits}");
    println!("BANK_PRIMARY_ADDRESS={addr_bech32m}");
    println!("BANK_CORE_ADDRESS={core_addr}");
    println!("BANK_TOTAL_CREDITS={total_credits}");
    s.teardown().await.expect("teardown failed");
}
