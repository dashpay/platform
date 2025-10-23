fn main() {
    #[cfg(feature = "subscriptions")]
    subscribe::main();
    #[cfg(not(feature = "subscriptions"))]
    {
        println!("Enable the 'subscriptions' feature to run this example.");
    }
}

#[cfg(feature = "subscriptions")]
mod subscribe {

    use dapi_grpc::platform::v0::platform_event_v0::Event as PlatformEvent;
    use dapi_grpc::platform::v0::platform_filter_v0::Kind as FilterKind;
    use dapi_grpc::platform::v0::platform_subscription_response::Version as ResponseVersion;
    use dapi_grpc::platform::v0::{
        PlatformFilterV0, PlatformSubscriptionResponse, StateTransitionResultFilter,
    };
    use dapi_grpc::tonic::Streaming;
    use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
    use dash_sdk::platform::types::epoch::Epoch;
    use dash_sdk::{Sdk, SdkBuilder};
    use futures::StreamExt;
    use rs_dapi_client::{Address, AddressList};
    use serde::Deserialize;
    use std::str::FromStr;
    use tracing::info;
    use tracing_subscriber::EnvFilter;
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

    #[tokio::main(flavor = "multi_thread", worker_threads = 2)]
    pub(super) async fn main() {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info")))
            .init();

        let config = Config::load();
        let sdk = setup_sdk(&config);
        // sanity check - fetch current epoch to see if connection works
        let epoch = Epoch::fetch_current(&sdk).await.expect("fetch epoch");
        tracing::info!("Current epoch: {:?}", epoch);

        // Subscribe to BlockCommitted only
        let filter_block = PlatformFilterV0 {
            kind: Some(FilterKind::BlockCommitted(true)),
        };
        let block_stream = sdk
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
                StateTransitionResultFilter {
                    tx_hash: tx_hash_bytes,
                },
            )),
        };
        let str_stream = sdk
            .subscribe_platform_events(filter_str)
            .await
            .expect("subscribe state_transition_result");

        // Subscribe to All events as a separate stream (demonstration)
        let filter_all = PlatformFilterV0 {
            kind: Some(FilterKind::All(true)),
        };
        let all_stream = sdk
            .subscribe_platform_events(filter_all)
            .await
            .expect("subscribe all");

        info!("Subscriptions created. Waiting for events... (Ctrl+C to exit)");

        let block_worker = tokio::spawn(worker(block_stream, "BlockCommitted"));
        let str_worker = tokio::spawn(worker(str_stream, "StateTransitionResult"));
        let all_worker = tokio::spawn(worker(all_stream, "AllEvents"));

        // Handle Ctrl+C to remove subscriptions and exit
        let abort_block = block_worker.abort_handle();
        let abort_str = str_worker.abort_handle();
        let abort_all = all_worker.abort_handle();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            info!("Ctrl+C received, stopping...");
            abort_block.abort();
            abort_str.abort();
            abort_all.abort();
        });

        // Wait for workers to finish
        let _ = tokio::join!(block_worker, str_worker, all_worker);
    }

    async fn worker(mut stream: Streaming<PlatformSubscriptionResponse>, label: &str) {
        while let Some(message) = stream.next().await {
            match message {
                Ok(response) => {
                    if let Some(ResponseVersion::V0(v0)) = response.version {
                        let sub_id = v0.client_subscription_id;
                        if let Some(event_v0) = v0.event {
                            if let Some(event) = event_v0.event {
                                match event {
                                    PlatformEvent::BlockCommitted(bc) => {
                                        if let Some(meta) = bc.meta {
                                            info!(
                                                "{label}: sub_id={sub_id} height={} time_ms={} tx_count={} block_id_hash=0x{}",
                                                meta.height,
                                                meta.time_ms,
                                                bc.tx_count,
                                                hex::encode(meta.block_id_hash)
                                            );
                                        }
                                    }
                                    PlatformEvent::StateTransitionFinalized(r) => {
                                        if let Some(meta) = r.meta {
                                            println!(
                                                "{label}: id={sub_id} height={} tx_hash=0x{} block_id_hash=0x{}",
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
                }
                Err(status) => {
                    eprintln!("{label}: stream error {status}");
                    break;
                }
            }
        }
        println!("{label}: stream closed");
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
}
