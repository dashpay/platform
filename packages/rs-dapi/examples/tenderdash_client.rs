use std::{env, error::Error, time::Duration};

use base64::engine::{Engine as _, general_purpose::STANDARD};
use rs_dapi::{DAPIResult, clients::TenderdashClient};
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Enable a basic tracing subscriber if the caller did not configure one already.
    let _ = tracing_subscriber::fmt::try_init();

    println!("Tenderdash Client example that tests all implemented Tenderdash methods.");
    println!(
        "You can use TENDERDASH_RPC_URL and TENDERDASH_WS_URL env vars to override the default connection URLs."
    );

    let rpc_url =
        env::var("TENDERDASH_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:26657".to_string());
    let ws_url = env::var("TENDERDASH_WS_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:26657/websocket".to_string());

    println!("Connecting to Tenderdash HTTP at {rpc_url} and WS at {ws_url}");

    let client = match TenderdashClient::new(&rpc_url, &ws_url).await {
        Ok(client) => client,
        Err(err) => {
            eprintln!("Failed to initialize Tenderdash client: {err}");
            return Ok(());
        }
    };

    // Fetch high-level node status information.
    print_result("status", client.status().await);

    // Fetch network information about peers and listeners.
    print_result("net_info", client.net_info().await);

    // Prepare simple demo payloads (base64 encoded strings are expected by RPC).
    let demo_tx = STANDARD.encode(b"demo-state-transition");
    let demo_hash = STANDARD.encode("demo-transaction-hash");

    // Validate a transaction with CheckTx (tenderdash will likely reject our dummy payload).
    print_result("check_tx", client.check_tx(demo_tx.clone()).await);

    // Try broadcasting the same transaction.
    print_result("broadcast_tx", client.broadcast_tx(demo_tx.clone()).await);

    // Search for the transaction in the mempool and committed blocks.
    print_result("unconfirmed_tx", client.unconfirmed_tx(&demo_hash).await);
    print_result("tx", client.tx(demo_hash.clone()).await);

    // Subscribe to streaming transaction and block events.
    let mut tx_events = client.subscribe_to_transactions();
    let mut block_events = client.subscribe_to_blocks();

    let tx_listener = tokio::spawn(async move {
        match timeout(Duration::from_secs(5), tx_events.recv()).await {
            Ok(Ok(event)) => println!("Received transaction event: {:?}", event),
            Ok(Err(err)) => println!("Transaction subscription closed with error: {err}"),
            Err(_) => println!("No transaction events received within 5 seconds"),
        }
    });

    let block_listener = tokio::spawn(async move {
        match timeout(Duration::from_secs(5), block_events.recv()).await {
            Ok(Ok(event)) => println!("Received block event: {:?}", event),
            Ok(Err(err)) => println!("Block subscription closed with error: {err}"),
            Err(_) => println!("No block events received within 5 seconds"),
        }
    });

    let (tx_result, block_result) = tokio::join!(tx_listener, block_listener);
    if let Err(err) = tx_result {
        println!("Transaction listener task failed: {err}");
    }
    if let Err(err) = block_result {
        println!("Block listener task failed: {err}");
    }

    println!("Tenderdash client example finished.");
    Ok(())
}

fn print_result<T: std::fmt::Debug>(label: &str, result: DAPIResult<T>) {
    match result {
        Ok(value) => println!("{label} succeeded: {value:#?}"),
        Err(err) => println!("{label} failed: {err}"),
    }
}
