//! Configuration helpers for mocking of rs-sdk.
//!
//! This module contains [Config] struct that can be used to configure rs-sdk.
//! It's mainly used for testing.
use rs_dapi_client::AddressList;
use serde::Deserialize;
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Deserialize)]
/// Configuration for rs-sdk.
///
/// Content of this configuration is loaded from environment variables or `${CARGO_MANIFEST_DIR}/.env` file
/// when the [Config::new()] is called.
/// Variable names in the enviroment and `.env` file must be prefixed with [RS_SDK_](Config::CONFIG_PREFIX)
/// and written as SCREAMING_SNAKE_CASE (e.g. `RS_SDK_PLATFORM_HOST`).
pub struct Config<D> {
    /// Hostname of the Dash Platform node to connect to
    pub platform_host: String,
    /// Port of the Dash Platform node grpc interface
    pub platform_port: u16,
    /// Port of the Dash Core RPC interface running on the Dash Platform node
    pub core_port: u16,
    /// Username for Dash Core RPC interface
    pub core_user: String,
    /// Password for Dash Core RPC interface
    pub core_password: String,

    /// Directory where all generated test vectors will be saved.
    ///
    /// See [SdkBuilder::with_dump_dir()](rs_sdk::SdkBuilder::with_dump_dir()) for more details.
    #[serde(default = "default_dump_dir")]
    pub dump_dir: PathBuf,

    /// Custom settings, used as needed by the test
    #[serde(flatten)]
    pub settings: D,
}

impl<D: for<'de1> Deserialize<'de1>> Config<D> {
    const CONFIG_PREFIX: &str = "RS_SDK_";
    /// Load configuration from operating system enviroment variables and `.env` file.
    ///
    /// Create new [Config] with data from enviroment variables and `${CARGO_MANIFEST_DIR}/.env` file.
    /// Variable names in the enviroment and `.env` file must be converted to SCREAMING_SNAKE_CASE and
    /// prefixed with [RS_SDK_](Config::CONFIG_PREFIX).
    pub fn new() -> Self {
        // load config from .env file, ignore errors
        let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/.env";

        dotenvy::from_path(path).expect("failed to load config file");

        envy::prefixed(Self::CONFIG_PREFIX)
            .from_env()
            .expect("configuration error")
    }
    #[allow(unused)]
    /// Create list of Platform addresses from the configuration
    pub fn address_list(&self) -> AddressList {
        let address: String = format!("http://{}:{}", self.platform_host, self.platform_port);

        AddressList::from_iter(vec![http::Uri::from_str(&address).expect("valid uri")])
    }

    /// Create new SDK instance
    ///
    /// Depending on the feature flags, it will connect to the configured platform node or mock API.
    ///
    /// ## Feature flags
    ///
    /// * `online-testing` is set - connect to the platform and generate
    /// new test vectors during execution
    /// * `online-testing` is not set - use mock implementation and
    /// load existing test vectors from disk
    pub async fn setup_api(&self) -> crate::Sdk {
        #[cfg(feature = "online-testing")]
        // Dump all traffic to disk
        let sdk = crate::SdkBuilder::new(self.address_list())
            .with_core(
                &self.platform_host,
                self.core_port,
                &self.core_user,
                &self.core_password,
            )
            .with_dump_dir(&self.dump_dir)
            .build()
            .expect("cannot initialize api");

        #[cfg(not(feature = "online-testing"))]
        let sdk = {
            let mut mock_sdk = crate::SdkBuilder::new_mock()
                .build()
                .expect("initialize api");

            mock_sdk
                .mock()
                .quorum_info_dir(&self.dump_dir)
                .load_expectations(&self.dump_dir)
                .await
                .expect("load expectations");

            mock_sdk
        };

        sdk
    }
}

impl<D: for<'de1> Deserialize<'de1>> Default for Config<D> {
    fn default() -> Self {
        Self::new()
    }
}

fn default_dump_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vectors")
}
