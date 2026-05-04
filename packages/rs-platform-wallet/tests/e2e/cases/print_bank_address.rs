//! Operational utility — print the bank wallet's primary receive address.
//!
//! Run on demand when you need to top up the bank:
//! ```
//! cargo test --test e2e -- --ignored --nocapture print_bank_primary_address
//! ```

use crate::framework::prelude::*;

#[tokio_shared_rt::test(shared)]
#[ignore = "operational utility — prints bank primary address; run on demand"]
async fn print_bank_primary_address() {
    let s = setup().await.expect("e2e setup failed");
    let bank = s.ctx.bank();
    let network = bank.network();
    let addr_bech32m = bank.primary_receive_address().to_bech32m_string(network);
    let total_credits = bank.total_credits().await;
    eprintln!("\n=== BANK PRIMARY ADDRESS ===\n{addr_bech32m}\n============================\n");
    eprintln!("BANK_TOTAL_CREDITS={total_credits}");
    println!("BANK_PRIMARY_ADDRESS={addr_bech32m}");
    println!("BANK_TOTAL_CREDITS={total_credits}");
    s.teardown().await.expect("teardown failed");
}
