use std::str::FromStr;

use clap::Parser;
use dapi_grpc::platform::v0::platform_events_response::platform_events_response_v0::Response as Resp;
use dapi_grpc::platform::v0::platform_filter_v0::Kind as FilterKind;
use dapi_grpc::platform::v0::PlatformFilterV0;
use dash_sdk::{Sdk, SdkBuilder};
use rs_dapi_client::{Address, AddressList};

#[derive(clap::Parser, Debug)]
#[command(version)]
pub struct Config {
    /// Dash Platform server hostname or IPv4 address
    #[arg(short = 'i', long = "address")]
    pub server_address: String,

    /// Dash Platform DAPI port
    #[arg(short = 'd', long)]
    pub platform_port: u16,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::parse();
    let sdk = setup_sdk(&config);

    // Subscribe using raw EventBus handle via SDK
    let filter = PlatformFilterV0 {
        kind: Some(FilterKind::All(true)),
    };
    let (id, handle) = sdk
        .subscribe_platform_events(filter)
        .await
        .expect("subscribe");

    println!("Subscribed with client_subscription_id={}", id);
    println!("Waiting for BlockCommitted events... (Ctrl+C to exit)");

    // Handle Ctrl+C to remove subscription and exit
    let shutdown = tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
    });

    tokio::select! {
        _ = shutdown => {
            println!("Shutting down...");
        }
        _ = async {
            loop {
                match handle.recv().await {
                    Some(resp) => {
                        // Parse and print
                        if let Some(dapi_grpc::platform::v0::platform_events_response::Version::V0(v0)) = resp.version {
                            match v0.response {
                                Some(Resp::Event(ev)) => {
                                    use dapi_grpc::platform::v0::platform_event_v0::Event as E;
                                    if let Some(event_v0) = ev.event {
                                        if let Some(event) = event_v0.event {
                                            match event {
                                                E::BlockCommitted(bc) => {
                                                    if let Some(meta) = bc.meta {
                                                        println!(
                                                            "BlockCommitted: height={} time_ms={} tx_count={} block_id_hash=0x{}",
                                                            meta.height,
                                                            meta.time_ms,
                                                            bc.tx_count,
                                                            hex::encode(meta.block_id_hash)
                                                        );
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                Some(Resp::Ack(ack)) => {
                                    println!("Ack: {} op={}", ack.client_subscription_id, ack.op);
                                }
                                Some(Resp::Error(err)) => {
                                    eprintln!("Error: {} code={} msg={}", err.client_subscription_id, err.code, err.message);
                                }
                                None => {}
                            }
                        }
                    }
                    None => break,
                }
            }
        } => {}
    }
}

fn setup_sdk(config: &Config) -> Sdk {
    let address = Address::from_str(&format!(
        "https://{}:{}",
        config.server_address, config.platform_port
    ))
    .expect("parse uri");

    SdkBuilder::new(AddressList::from_iter([address]))
        .build()
        .expect("cannot build sdk")
}
