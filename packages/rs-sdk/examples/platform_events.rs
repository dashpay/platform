use dapi_grpc::platform::v0::platform_filter_v0::Kind as FilterKind;
use dapi_grpc::platform::v0::PlatformFilterV0;
use dapi_grpc::platform::v0::{
    platform_events_response::platform_events_response_v0::Response as Resp, PlatformEventsResponse,
};
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::platform::types::epoch::Epoch;
use dash_sdk::{Sdk, SdkBuilder};
use rs_dapi_client::{Address, AddressList};
use dash_event_bus::SubscriptionHandle;
use serde::Deserialize;
use std::str::FromStr;
use zeroize::Zeroizing;

#[derive(Debug, Deserialize)]
pub struct Config {
    // Aligned with rs-sdk/tests/fetch/config.rs
    #[serde(default)]
    pub platform_host: String,
    #[serde(default)]
    pub platform_port: u16,
    #[serde(default)]
    pub platform_ssl: bool,

    #[serde(default)]
    pub core_host: Option<String>,
    #[serde(default)]
    pub core_port: u16,
    #[serde(default)]
    pub core_user: String,
    #[serde(default)]
    pub core_password: Zeroizing<String>,

    #[serde(default)]
    pub platform_ca_cert_path: Option<std::path::PathBuf>,

    // Optional hex-encoded tx hash to filter STR events
    #[serde(default)]
    pub state_transition_tx_hash_hex: Option<String>,
}

impl Config {
    const CONFIG_PREFIX: &'static str = "DASH_SDK_";
    fn load() -> Self {
        let path: String = env!("CARGO_MANIFEST_DIR").to_owned() + "/tests/.env";
        let _ = dotenvy::from_path(&path);
        envy::prefixed(Self::CONFIG_PREFIX)
            .from_env()
            .expect("configuration error: missing DASH_SDK_* vars; see rs-sdk/tests/.env")
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::load();
    let sdk = setup_sdk(&config);
    // sanity check - fetch current epoch to see if connection works
    let epoch = Epoch::fetch_current(&sdk).await.expect("fetch epoch");
    tracing::info!("Current epoch: {:?}", epoch);

    // Subscribe to BlockCommitted only
    let filter_block = PlatformFilterV0 {
        kind: Some(FilterKind::BlockCommitted(true)),
    };
    let (block_id, block_handle) = sdk
        .subscribe_platform_events(filter_block)
        .await
        .expect("subscribe block_committed");

    // Subscribe to StateTransitionFinalized; optionally filter by tx hash if provided
    let tx_hash_bytes = config
        .state_transition_tx_hash_hex
        .as_deref()
        .and_then(|s| hex::decode(s).ok());
    let filter_str = PlatformFilterV0 {
        kind: Some(FilterKind::StateTransitionResult(
            dapi_grpc::platform::v0::StateTransitionResultFilter {
                tx_hash: tx_hash_bytes,
            },
        )),
    };
    let (str_id, str_handle) = sdk
        .subscribe_platform_events(filter_str)
        .await
        .expect("subscribe state_transition_result");

    // Subscribe to All events as a separate stream (demonstration)
    let filter_all = PlatformFilterV0 {
        kind: Some(FilterKind::All(true)),
    };
    let (all_id, all_handle) = sdk
        .subscribe_platform_events(filter_all)
        .await
        .expect("subscribe all");

    println!(
        "Subscribed: BlockCommitted id={}, STR id={}, All id={}",
        block_id, str_id, all_id
    );
    println!("Waiting for events... (Ctrl+C to exit)");

    let block_worker = tokio::spawn(worker(block_handle));
    let str_worker = tokio::spawn(worker(str_handle));
    let all_worker = tokio::spawn(worker(all_handle));

    // Handle Ctrl+C to remove subscriptions and exit
    let abort_block = block_worker.abort_handle();
    let abort_str = str_worker.abort_handle();
    let abort_all = all_worker.abort_handle();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("Ctrl+C received, stopping...");
        abort_block.abort();
        abort_str.abort();
        abort_all.abort();
    });

    // Wait for workers to finish
    let _ = tokio::join!(block_worker, str_worker, all_worker);
}

async fn worker<F>(handle: SubscriptionHandle<PlatformEventsResponse, F>)
where
    F: Send + Sync + 'static,
{
    while let Some(resp) = handle.recv().await {
        // Parse and print
        if let Some(dapi_grpc::platform::v0::platform_events_response::Version::V0(v0)) =
            resp.version
        {
            match v0.response {
                Some(Resp::Event(ev)) => {
                    let sub_id = ev.client_subscription_id;
                    use dapi_grpc::platform::v0::platform_event_v0::Event as E;
                    if let Some(event_v0) = ev.event {
                        if let Some(event) = event_v0.event {
                            match event {
                                E::BlockCommitted(bc) => {
                                    if let Some(meta) = bc.meta {
                                        println!(
                                            "{} BlockCommitted: height={} time_ms={} tx_count={} block_id_hash=0x{}",
                                            sub_id,
                                            meta.height,
                                            meta.time_ms,
                                            bc.tx_count,
                                            hex::encode(meta.block_id_hash)
                                        );
                                    }
                                }
                                E::StateTransitionFinalized(r) => {
                                    if let Some(meta) = r.meta {
                                        println!(
                                            "{} StateTransitionFinalized: height={} tx_hash=0x{} block_id_hash=0x{}",
                                            sub_id,
                                            meta.height,
                                            hex::encode(r.tx_hash),
                                            hex::encode(meta.block_id_hash)
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Resp::Ack(ack)) => {
                    println!("Ack: {} op={}", ack.client_subscription_id, ack.op);
                }
                Some(Resp::Error(err)) => {
                    eprintln!(
                        "Error: {} code={} msg={}",
                        err.client_subscription_id, err.code, err.message
                    );
                }
                None => {}
            }
        }
    }
}

fn setup_sdk(config: &Config) -> Sdk {
    let scheme = if config.platform_ssl { "https" } else { "http" };
    let host = &config.platform_host;
    let address = Address::from_str(&format!("{}://{}:{}", scheme, host, config.platform_port))
        .expect("parse uri");
    tracing::debug!("Using DAPI address: {}", address.uri());
    let core_host = config.core_host.as_deref().unwrap_or(host);

    #[allow(unused_mut)]
    let mut builder = SdkBuilder::new(AddressList::from_iter([address])).with_core(
        core_host,
        config.core_port,
        &config.core_user,
        &config.core_password,
    );

    #[cfg(not(target_arch = "wasm32"))]
    if let Some(cert) = &config.platform_ca_cert_path {
        builder = builder
            .with_ca_certificate_file(cert)
            .expect("load CA cert");
    }

    builder.build().expect("cannot build sdk")
}
