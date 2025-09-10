use std::str::FromStr;

use dapi_grpc::platform::v0::platform_filter_v0::Kind as FilterKind;
use dapi_grpc::platform::v0::PlatformFilterV0;
use dapi_grpc::platform::v0::{
    platform_events_response::platform_events_response_v0::Response as Resp, PlatformEventsResponse,
};
use dash_sdk::{Sdk, SdkBuilder};
use rs_dapi_client::{Address, AddressList};
use rs_dash_notify::SubscriptionHandle;
use serde::Deserialize;
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

    let worker_thread = tokio::spawn(worker(handle));

    // Handle Ctrl+C to remove subscription and exit
    let worker_abort = worker_thread.abort_handle();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!("Ctrl+C received, stopping...");
        worker_abort.abort();
    });

    // Wait for worker thread to finish
    worker_thread.await.ok();
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
                    use dapi_grpc::platform::v0::platform_event_v0::Event as E;
                    if let Some(event_v0) = ev.event {
                        if let Some(event) = event_v0.event {
                            #[allow(clippy::collapsible_match)]
                            if let E::BlockCommitted(bc) = event {
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

    let core_host = config
        .core_host
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or(host);

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
