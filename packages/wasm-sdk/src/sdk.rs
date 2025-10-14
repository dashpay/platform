use crate::context_provider::WasmContext;
use crate::error::WasmSdkError;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::{Sdk, SdkBuilder};
use rs_dapi_client::RequestSettings;
use std::ops::{Deref, DerefMut};
use std::time::Duration;
use wasm_bindgen::prelude::wasm_bindgen;

// Store shared trusted contexts
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub(crate) static MAINNET_TRUSTED_CONTEXT: Lazy<
    Mutex<Option<crate::context_provider::WasmTrustedContext>>,
> = Lazy::new(|| Mutex::new(None));
pub(crate) static TESTNET_TRUSTED_CONTEXT: Lazy<
    Mutex<Option<crate::context_provider::WasmTrustedContext>>,
> = Lazy::new(|| Mutex::new(None));

#[wasm_bindgen]
pub struct WasmSdk(Sdk);
// Dereference JsSdk to Sdk so that we can use &JsSdk everywhere where &sdk is needed
impl std::ops::Deref for WasmSdk {
    type Target = Sdk;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Sdk> for WasmSdk {
    fn as_ref(&self) -> &Sdk {
        &self.0
    }
}

impl From<Sdk> for WasmSdk {
    fn from(sdk: Sdk) -> Self {
        WasmSdk(sdk)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    pub fn version(&self) -> u32 {
        self.0.version().protocol_version
    }

    /// Get reference to the inner SDK for direct gRPC calls
    pub(crate) fn inner_sdk(&self) -> &Sdk {
        &self.0
    }

    /// Get the network this SDK is configured for
    pub(crate) fn network(&self) -> dash_sdk::dpp::dashcore::Network {
        self.0.network
    }
}

impl WasmSdk {
    /// Clone the inner Sdk (not exposed to WASM)
    pub(crate) fn inner_clone(&self) -> Sdk {
        self.0.clone()
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "prefetchTrustedQuorumsMainnet")]
    pub async fn prefetch_trusted_quorums_mainnet() -> Result<(), WasmSdkError> {
        use crate::context_provider::WasmTrustedContext;

        let trusted_context = WasmTrustedContext::new_mainnet()
            .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))?;

        trusted_context
            .prefetch_quorums()
            .await
            .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))?;

        // Store the context for later use
        *MAINNET_TRUSTED_CONTEXT.lock().unwrap() = Some(trusted_context);

        Ok(())
    }

    #[wasm_bindgen(js_name = "prefetchTrustedQuorumsTestnet")]
    pub async fn prefetch_trusted_quorums_testnet() -> Result<(), WasmSdkError> {
        use crate::context_provider::WasmTrustedContext;

        let trusted_context = WasmTrustedContext::new_testnet()
            .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))?;

        trusted_context
            .prefetch_quorums()
            .await
            .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))?;

        // Store the context for later use
        *TESTNET_TRUSTED_CONTEXT.lock().unwrap() = Some(trusted_context);

        Ok(())
    }
}

#[wasm_bindgen]
pub struct WasmSdkBuilder(SdkBuilder);

impl Deref for WasmSdkBuilder {
    type Target = SdkBuilder;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WasmSdkBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[wasm_bindgen]
impl WasmSdkBuilder {
    /// Get the latest platform version number
    #[wasm_bindgen(js_name = "getLatestVersionNumber")]
    pub fn get_latest_version_number() -> u32 {
        PlatformVersion::latest().protocol_version
    }

    /// Create a new SdkBuilder with specific addresses and network.
    ///
    /// # Arguments
    /// * `addresses` - Array of HTTPS URLs (e.g., ["https://127.0.0.1:1443"])
    /// * `network` - Network identifier: "mainnet" or "testnet"
    ///
    /// # Example
    /// ```javascript
    /// const builder = WasmSdkBuilder.withAddresses(['https://127.0.0.1:1443'], 'testnet');
    /// const sdk = builder.build();
    /// ```
    #[wasm_bindgen(js_name = "withAddresses")]
    pub fn new_with_addresses(
        addresses: Vec<String>,
        network: String,
    ) -> Result<Self, WasmSdkError> {
        use crate::context_provider::WasmTrustedContext;
        use dash_sdk::dpp::dashcore::Network;
        use dash_sdk::sdk::Uri;
        use rs_dapi_client::Address;

        // Parse and validate addresses
        if addresses.is_empty() {
            return Err(WasmSdkError::invalid_argument(
                "Addresses must be a non-empty array",
            ));
        }
        let parsed_addresses: Result<Vec<Address>, _> = addresses
            .into_iter()
            .map(|addr| {
                addr.parse::<Uri>()
                    .map_err(|e| format!("Invalid URI '{}': {}", addr, e))
                    .and_then(|uri| {
                        Address::try_from(uri).map_err(|e| format!("Invalid address: {}", e))
                    })
            })
            .collect();

        let parsed_addresses = parsed_addresses.map_err(|e| WasmSdkError::invalid_argument(e))?;

        // Parse network - only mainnet and testnet are supported
        let network = match network.to_lowercase().as_str() {
            "mainnet" => Network::Dash,
            "testnet" => Network::Testnet,
            _ => {
                return Err(WasmSdkError::invalid_argument(format!(
                    "Invalid network '{}'. Expected: mainnet or testnet",
                    network
                )))
            }
        };

        // Use the cached trusted context if available for the network, otherwise create a new one
        let trusted_context = match network {
            Network::Dash => {
                let guard = MAINNET_TRUSTED_CONTEXT.lock().unwrap();
                guard.clone()
            }
            .map(Ok)
            .unwrap_or_else(|| {
                WasmTrustedContext::new_mainnet()
                    .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))
            })?,
            Network::Testnet => {
                let guard = TESTNET_TRUSTED_CONTEXT.lock().unwrap();
                guard.clone()
            }
            .map(Ok)
            .unwrap_or_else(|| {
                WasmTrustedContext::new_testnet()
                    .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))
            })?,
            // Network was already validated above
            _ => unreachable!("Network already validated to mainnet or testnet"),
        };

        let address_list = dash_sdk::sdk::AddressList::from_iter(parsed_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(network)
            .with_context_provider(trusted_context);

        Ok(Self(sdk_builder))
    }

    #[wasm_bindgen(js_name = "mainnet")]
    pub fn new_mainnet() -> Self {
        // Mainnet addresses from mnowatch.org
        let mainnet_addresses = vec![
            "https://149.28.241.190:443".parse().unwrap(),
            "https://198.7.115.48:443".parse().unwrap(),
            "https://134.255.182.186:443".parse().unwrap(),
            "https://93.115.172.39:443".parse().unwrap(),
            "https://5.189.164.253:443".parse().unwrap(),
            "https://178.215.237.134:443".parse().unwrap(),
            "https://157.66.81.162:443".parse().unwrap(),
            "https://173.212.232.90:443".parse().unwrap(),
            "https://178.215.237.135:443".parse().unwrap(),
            "https://5.182.33.231:443".parse().unwrap(),
            "https://109.199.104.243:443".parse().unwrap(),
            "https://37.60.236.212:443".parse().unwrap(),
            "https://23.88.63.58:443".parse().unwrap(),
            "https://207.244.247.40:443".parse().unwrap(),
            "https://45.32.70.131:443".parse().unwrap(),
            "https://158.220.122.76:443".parse().unwrap(),
            "https://52.33.9.172:443".parse().unwrap(),
            "https://194.163.166.185:443".parse().unwrap(),
            "https://185.158.107.124:443".parse().unwrap(),
            "https://185.198.234.17:443".parse().unwrap(),
            "https://93.190.140.101:443".parse().unwrap(),
            "https://194.163.153.225:443".parse().unwrap(),
            "https://194.146.13.7:443".parse().unwrap(),
            "https://158.247.208.247:443".parse().unwrap(),
            "https://93.190.140.112:443".parse().unwrap(),
            "https://75.119.132.2:443".parse().unwrap(),
            "https://173.212.239.247:443".parse().unwrap(),
            "https://51.38.142.61:443".parse().unwrap(),
            "https://44.240.99.214:443".parse().unwrap(),
            "https://5.75.133.148:443".parse().unwrap(),
            "https://62.84.182.155:443".parse().unwrap(),
            "https://89.35.131.149:443".parse().unwrap(),
            "https://192.248.178.237:443".parse().unwrap(),
            "https://45.77.11.194:443".parse().unwrap(),
            "https://37.60.243.119:443".parse().unwrap(),
            "https://46.254.241.7:443".parse().unwrap(),
            "https://194.195.87.34:443".parse().unwrap(),
            "https://43.163.251.51:443".parse().unwrap(),
            "https://184.174.36.201:443".parse().unwrap(),
            "https://85.239.244.103:443".parse().unwrap(),
            "https://65.108.246.145:443".parse().unwrap(),
            "https://194.163.170.55:443".parse().unwrap(),
            "https://2.58.82.231:443".parse().unwrap(),
            "https://5.252.55.187:443".parse().unwrap(),
            "https://198.7.119.139:443".parse().unwrap(),
            "https://213.199.44.112:443".parse().unwrap(),
            "https://155.138.220.69:443".parse().unwrap(),
            "https://209.145.48.154:443".parse().unwrap(),
            "https://162.212.35.100:443".parse().unwrap(),
            "https://185.239.209.6:443".parse().unwrap(),
            "https://157.173.113.158:443".parse().unwrap(),
            "https://134.255.182.185:443".parse().unwrap(),
            "https://173.212.239.124:443".parse().unwrap(),
            "https://144.126.141.62:443".parse().unwrap(),
            "https://51.38.142.62:443".parse().unwrap(),
            "https://157.10.199.77:443".parse().unwrap(),
            "https://5.189.186.78:443".parse().unwrap(),
            "https://164.68.118.37:443".parse().unwrap(),
            "https://158.220.92.144:443".parse().unwrap(),
            "https://192.248.175.198:443".parse().unwrap(),
            "https://43.167.244.109:443".parse().unwrap(),
            "https://146.59.45.235:443".parse().unwrap(),
            "https://104.200.24.196:443".parse().unwrap(),
            "https://146.59.153.204:443".parse().unwrap(),
            "https://37.60.236.225:443".parse().unwrap(),
            "https://172.233.66.70:443".parse().unwrap(),
            "https://57.128.212.163:443".parse().unwrap(),
            "https://82.208.20.153:443".parse().unwrap(),
            "https://51.195.235.166:443".parse().unwrap(),
            "https://158.220.122.74:443".parse().unwrap(),
            "https://82.211.21.38:443".parse().unwrap(),
            "https://93.115.172.37:443".parse().unwrap(),
            "https://185.198.234.25:443".parse().unwrap(),
            "https://84.247.187.76:443".parse().unwrap(),
            "https://89.35.131.39:443".parse().unwrap(),
            "https://93.115.172.38:443".parse().unwrap(),
            "https://134.255.183.250:443".parse().unwrap(),
            "https://85.190.243.3:443".parse().unwrap(),
            "https://185.192.96.70:443".parse().unwrap(),
            "https://134.255.183.248:443".parse().unwrap(),
            "https://52.36.102.91:443".parse().unwrap(),
            "https://139.99.201.103:443".parse().unwrap(),
            "https://134.255.183.247:443".parse().unwrap(),
            "https://213.199.34.250:443".parse().unwrap(),
            "https://161.97.74.173:443".parse().unwrap(),
            "https://45.135.180.79:443".parse().unwrap(),
            "https://45.135.180.130:443".parse().unwrap(),
            "https://173.212.251.130:443".parse().unwrap(),
            "https://157.173.122.157:443".parse().unwrap(),
            "https://49.13.237.193:443".parse().unwrap(),
            "https://37.27.83.17:443".parse().unwrap(),
            "https://45.135.180.114:443".parse().unwrap(),
            "https://89.35.131.61:443".parse().unwrap(),
            "https://86.107.101.74:443".parse().unwrap(),
            "https://134.255.182.187:443".parse().unwrap(),
            "https://157.173.202.14:443".parse().unwrap(),
            "https://62.171.170.14:443".parse().unwrap(),
            "https://5.252.55.190:443".parse().unwrap(),
            "https://198.7.115.43:443".parse().unwrap(),
            "https://157.173.122.158:443".parse().unwrap(),
            "https://108.61.165.170:443".parse().unwrap(),
            "https://157.10.199.79:443".parse().unwrap(),
            "https://89.35.131.219:443".parse().unwrap(),
            "https://185.166.217.154:443".parse().unwrap(),
            "https://31.220.88.116:443".parse().unwrap(),
            "https://149.202.78.214:443".parse().unwrap(),
            "https://195.26.254.228:443".parse().unwrap(),
            "https://217.77.12.101:443".parse().unwrap(),
            "https://43.167.240.90:443".parse().unwrap(),
            "https://157.10.199.82:443".parse().unwrap(),
            "https://5.252.55.189:443".parse().unwrap(),
            "https://167.86.93.21:443".parse().unwrap(),
            "https://195.26.241.252:443".parse().unwrap(),
            "https://161.97.170.251:443".parse().unwrap(),
            "https://51.195.47.118:443".parse().unwrap(),
            "https://45.135.180.70:443".parse().unwrap(),
            "https://167.88.169.16:443".parse().unwrap(),
            "https://62.169.17.112:443".parse().unwrap(),
            "https://82.211.21.18:443".parse().unwrap(),
            "https://52.10.213.198:443".parse().unwrap(),
            "https://139.84.231.221:443".parse().unwrap(),
            "https://51.75.60.227:443".parse().unwrap(),
            "https://93.190.140.162:443".parse().unwrap(),
            "https://198.7.115.38:443".parse().unwrap(),
            "https://37.60.236.161:443".parse().unwrap(),
            "https://37.60.244.220:443".parse().unwrap(),
            "https://46.254.241.9:443".parse().unwrap(),
            "https://167.86.94.138:443".parse().unwrap(),
            "https://192.95.32.205:443".parse().unwrap(),
            "https://95.179.241.182:443".parse().unwrap(),
            "https://65.109.84.204:443".parse().unwrap(),
            "https://93.115.172.36:443".parse().unwrap(),
            "https://82.211.21.16:443".parse().unwrap(),
            "https://158.220.89.188:443".parse().unwrap(),
            "https://95.216.146.18:443".parse().unwrap(),
            "https://167.114.153.110:443".parse().unwrap(),
            "https://89.250.75.61:443".parse().unwrap(),
            "https://185.194.216.84:443".parse().unwrap(),
            "https://158.220.87.156:443".parse().unwrap(),
            "https://31.220.84.93:443".parse().unwrap(),
            "https://185.197.250.227:443".parse().unwrap(),
            "https://162.250.188.207:443".parse().unwrap(),
            "https://207.180.231.37:443".parse().unwrap(),
            "https://207.180.231.39:443".parse().unwrap(),
            "https://66.70.170.22:443".parse().unwrap(),
            "https://149.28.247.165:443".parse().unwrap(),
            "https://45.85.147.192:443".parse().unwrap(),
            "https://157.173.122.156:443".parse().unwrap(),
            "https://213.199.34.251:443".parse().unwrap(),
            "https://95.171.21.131:443".parse().unwrap(),
            "https://87.228.24.64:443".parse().unwrap(),
            "https://5.189.151.7:443".parse().unwrap(),
            "https://90.16.41.190:443".parse().unwrap(),
            "https://38.242.231.212:443".parse().unwrap(),
            "https://38.143.58.210:443".parse().unwrap(),
            "https://157.66.81.130:443".parse().unwrap(),
            "https://217.77.12.102:443".parse().unwrap(),
            "https://157.10.199.125:443".parse().unwrap(),
            "https://46.254.241.8:443".parse().unwrap(),
            "https://49.12.102.105:443".parse().unwrap(),
            "https://134.255.182.189:443".parse().unwrap(),
            "https://81.17.101.141:443".parse().unwrap(),
            "https://64.23.134.67:443".parse().unwrap(),
            "https://93.190.140.190:443".parse().unwrap(),
            "https://86.107.101.128:443".parse().unwrap(),
            "https://54.69.95.118:443".parse().unwrap(),
            "https://158.220.122.13:443".parse().unwrap(),
            "https://82.211.25.69:443".parse().unwrap(),
            "https://144.217.69.169:443".parse().unwrap(),
            "https://93.190.140.111:443".parse().unwrap(),
            "https://5.189.140.20:443".parse().unwrap(),
            "https://93.190.140.114:443".parse().unwrap(),
            "https://135.181.110.216:443".parse().unwrap(),
            "https://207.180.213.141:443".parse().unwrap(),
            "https://45.76.141.74:443".parse().unwrap(),
            "https://185.194.216.38:443".parse().unwrap(),
            "https://161.97.66.31:443".parse().unwrap(),
            "https://188.245.90.255:443".parse().unwrap(),
            "https://65.109.84.201:443".parse().unwrap(),
            "https://164.68.114.36:443".parse().unwrap(),
            "https://167.88.165.175:443".parse().unwrap(),
            "https://43.167.239.145:443".parse().unwrap(),
            "https://37.60.236.201:443".parse().unwrap(),
            "https://185.239.208.110:443".parse().unwrap(),
            "https://95.179.139.125:443".parse().unwrap(),
            "https://213.199.34.248:443".parse().unwrap(),
            "https://178.18.254.136:443".parse().unwrap(),
            "https://82.211.21.40:443".parse().unwrap(),
            "https://213.199.35.18:443".parse().unwrap(),
            "https://38.102.124.86:443".parse().unwrap(),
            "https://45.77.129.235:443".parse().unwrap(),
            "https://81.0.249.58:443".parse().unwrap(),
            "https://37.60.243.59:443".parse().unwrap(),
            "https://37.60.236.247:443".parse().unwrap(),
            "https://89.35.131.218:443".parse().unwrap(),
            "https://5.189.145.80:443".parse().unwrap(),
            "https://149.102.152.219:443".parse().unwrap(),
            "https://77.221.148.204:443".parse().unwrap(),
            "https://46.254.241.11:443".parse().unwrap(),
            "https://207.180.218.245:443".parse().unwrap(),
            "https://89.35.131.158:443".parse().unwrap(),
            "https://5.252.55.188:443".parse().unwrap(),
            "https://185.215.166.126:443".parse().unwrap(),
            "https://164.132.55.103:443".parse().unwrap(),
            "https://162.250.190.133:443".parse().unwrap(),
            "https://157.66.81.218:443".parse().unwrap(),
            "https://5.39.27.224:443".parse().unwrap(),
            "https://213.159.77.221:443".parse().unwrap(),
            "https://213.199.35.15:443".parse().unwrap(),
            "https://114.132.172.215:443".parse().unwrap(),
        ];

        let address_list = dash_sdk::sdk::AddressList::from_iter(mainnet_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Dash)
            .with_context_provider(WasmContext {});

        Self(sdk_builder)
    }

    #[wasm_bindgen(js_name = "mainnetTrusted")]
    pub fn new_mainnet_trusted() -> Result<Self, WasmSdkError> {
        use crate::context_provider::WasmTrustedContext;

        // Use the cached context if available, otherwise create a new one
        let trusted_context = {
            let guard = MAINNET_TRUSTED_CONTEXT.lock().unwrap();
            guard.clone()
        }
        .map(Ok)
        .unwrap_or_else(|| {
            WasmTrustedContext::new_mainnet()
                .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))
        })?;

        // Mainnet addresses from mnowatch.org
        let mainnet_addresses = vec![
            "https://149.28.241.190:443".parse().unwrap(),
            "https://198.7.115.48:443".parse().unwrap(),
            "https://134.255.182.186:443".parse().unwrap(),
            "https://93.115.172.39:443".parse().unwrap(),
            "https://5.189.164.253:443".parse().unwrap(),
            "https://178.215.237.134:443".parse().unwrap(),
            "https://157.66.81.162:443".parse().unwrap(),
            "https://173.212.232.90:443".parse().unwrap(),
            "https://178.215.237.135:443".parse().unwrap(),
            "https://5.182.33.231:443".parse().unwrap(),
            "https://109.199.104.243:443".parse().unwrap(),
            "https://37.60.236.212:443".parse().unwrap(),
            "https://23.88.63.58:443".parse().unwrap(),
            "https://207.244.247.40:443".parse().unwrap(),
            "https://45.32.70.131:443".parse().unwrap(),
            "https://158.220.122.76:443".parse().unwrap(),
            "https://52.33.9.172:443".parse().unwrap(),
            "https://194.163.166.185:443".parse().unwrap(),
            "https://185.158.107.124:443".parse().unwrap(),
            "https://185.198.234.17:443".parse().unwrap(),
            "https://93.190.140.101:443".parse().unwrap(),
            "https://194.163.153.225:443".parse().unwrap(),
            "https://194.146.13.7:443".parse().unwrap(),
            "https://158.247.208.247:443".parse().unwrap(),
            "https://93.190.140.112:443".parse().unwrap(),
            "https://75.119.132.2:443".parse().unwrap(),
            "https://173.212.239.247:443".parse().unwrap(),
            "https://51.38.142.61:443".parse().unwrap(),
            "https://44.240.99.214:443".parse().unwrap(),
            "https://5.75.133.148:443".parse().unwrap(),
            "https://62.84.182.155:443".parse().unwrap(),
            "https://89.35.131.149:443".parse().unwrap(),
            "https://192.248.178.237:443".parse().unwrap(),
            "https://45.77.11.194:443".parse().unwrap(),
            "https://37.60.243.119:443".parse().unwrap(),
            "https://46.254.241.7:443".parse().unwrap(),
            "https://194.195.87.34:443".parse().unwrap(),
            "https://43.163.251.51:443".parse().unwrap(),
            "https://184.174.36.201:443".parse().unwrap(),
            "https://85.239.244.103:443".parse().unwrap(),
            "https://65.108.246.145:443".parse().unwrap(),
            "https://194.163.170.55:443".parse().unwrap(),
            "https://2.58.82.231:443".parse().unwrap(),
            "https://5.252.55.187:443".parse().unwrap(),
            "https://198.7.119.139:443".parse().unwrap(),
            "https://213.199.44.112:443".parse().unwrap(),
            "https://155.138.220.69:443".parse().unwrap(),
            "https://209.145.48.154:443".parse().unwrap(),
            "https://162.212.35.100:443".parse().unwrap(),
            "https://185.239.209.6:443".parse().unwrap(),
            "https://157.173.113.158:443".parse().unwrap(),
            "https://134.255.182.185:443".parse().unwrap(),
            "https://173.212.239.124:443".parse().unwrap(),
            "https://144.126.141.62:443".parse().unwrap(),
            "https://51.38.142.62:443".parse().unwrap(),
            "https://157.10.199.77:443".parse().unwrap(),
            "https://5.189.186.78:443".parse().unwrap(),
            "https://164.68.118.37:443".parse().unwrap(),
            "https://158.220.92.144:443".parse().unwrap(),
            "https://192.248.175.198:443".parse().unwrap(),
            "https://43.167.244.109:443".parse().unwrap(),
            "https://146.59.45.235:443".parse().unwrap(),
            "https://104.200.24.196:443".parse().unwrap(),
            "https://146.59.153.204:443".parse().unwrap(),
            "https://37.60.236.225:443".parse().unwrap(),
            "https://172.233.66.70:443".parse().unwrap(),
            "https://57.128.212.163:443".parse().unwrap(),
            "https://82.208.20.153:443".parse().unwrap(),
            "https://51.195.235.166:443".parse().unwrap(),
            "https://158.220.122.74:443".parse().unwrap(),
            "https://82.211.21.38:443".parse().unwrap(),
            "https://93.115.172.37:443".parse().unwrap(),
            "https://185.198.234.25:443".parse().unwrap(),
            "https://84.247.187.76:443".parse().unwrap(),
            "https://89.35.131.39:443".parse().unwrap(),
            "https://93.115.172.38:443".parse().unwrap(),
            "https://134.255.183.250:443".parse().unwrap(),
            "https://85.190.243.3:443".parse().unwrap(),
            "https://185.192.96.70:443".parse().unwrap(),
            "https://134.255.183.248:443".parse().unwrap(),
            "https://52.36.102.91:443".parse().unwrap(),
            "https://139.99.201.103:443".parse().unwrap(),
            "https://134.255.183.247:443".parse().unwrap(),
            "https://213.199.34.250:443".parse().unwrap(),
            "https://161.97.74.173:443".parse().unwrap(),
            "https://45.135.180.79:443".parse().unwrap(),
            "https://45.135.180.130:443".parse().unwrap(),
            "https://173.212.251.130:443".parse().unwrap(),
            "https://157.173.122.157:443".parse().unwrap(),
            "https://49.13.237.193:443".parse().unwrap(),
            "https://37.27.83.17:443".parse().unwrap(),
            "https://45.135.180.114:443".parse().unwrap(),
            "https://89.35.131.61:443".parse().unwrap(),
            "https://86.107.101.74:443".parse().unwrap(),
            "https://134.255.182.187:443".parse().unwrap(),
            "https://157.173.202.14:443".parse().unwrap(),
            "https://62.171.170.14:443".parse().unwrap(),
            "https://5.252.55.190:443".parse().unwrap(),
            "https://198.7.115.43:443".parse().unwrap(),
            "https://157.173.122.158:443".parse().unwrap(),
            "https://108.61.165.170:443".parse().unwrap(),
            "https://157.10.199.79:443".parse().unwrap(),
            "https://89.35.131.219:443".parse().unwrap(),
            "https://185.166.217.154:443".parse().unwrap(),
            "https://31.220.88.116:443".parse().unwrap(),
            "https://149.202.78.214:443".parse().unwrap(),
            "https://195.26.254.228:443".parse().unwrap(),
            "https://217.77.12.101:443".parse().unwrap(),
            "https://43.167.240.90:443".parse().unwrap(),
            "https://157.10.199.82:443".parse().unwrap(),
            "https://5.252.55.189:443".parse().unwrap(),
            "https://167.86.93.21:443".parse().unwrap(),
            "https://195.26.241.252:443".parse().unwrap(),
            "https://161.97.170.251:443".parse().unwrap(),
            "https://51.195.47.118:443".parse().unwrap(),
            "https://45.135.180.70:443".parse().unwrap(),
            "https://167.88.169.16:443".parse().unwrap(),
            "https://62.169.17.112:443".parse().unwrap(),
            "https://82.211.21.18:443".parse().unwrap(),
            "https://52.10.213.198:443".parse().unwrap(),
            "https://139.84.231.221:443".parse().unwrap(),
            "https://51.75.60.227:443".parse().unwrap(),
            "https://93.190.140.162:443".parse().unwrap(),
            "https://198.7.115.38:443".parse().unwrap(),
            "https://37.60.236.161:443".parse().unwrap(),
            "https://37.60.244.220:443".parse().unwrap(),
            "https://46.254.241.9:443".parse().unwrap(),
            "https://167.86.94.138:443".parse().unwrap(),
            "https://192.95.32.205:443".parse().unwrap(),
            "https://95.179.241.182:443".parse().unwrap(),
            "https://65.109.84.204:443".parse().unwrap(),
            "https://93.115.172.36:443".parse().unwrap(),
            "https://82.211.21.16:443".parse().unwrap(),
            "https://158.220.89.188:443".parse().unwrap(),
            "https://95.216.146.18:443".parse().unwrap(),
            "https://167.114.153.110:443".parse().unwrap(),
            "https://89.250.75.61:443".parse().unwrap(),
            "https://185.194.216.84:443".parse().unwrap(),
            "https://158.220.87.156:443".parse().unwrap(),
            "https://31.220.84.93:443".parse().unwrap(),
            "https://185.197.250.227:443".parse().unwrap(),
            "https://162.250.188.207:443".parse().unwrap(),
            "https://207.180.231.37:443".parse().unwrap(),
            "https://207.180.231.39:443".parse().unwrap(),
            "https://66.70.170.22:443".parse().unwrap(),
            "https://149.28.247.165:443".parse().unwrap(),
            "https://45.85.147.192:443".parse().unwrap(),
            "https://157.173.122.156:443".parse().unwrap(),
            "https://213.199.34.251:443".parse().unwrap(),
            "https://95.171.21.131:443".parse().unwrap(),
            "https://87.228.24.64:443".parse().unwrap(),
            "https://5.189.151.7:443".parse().unwrap(),
            "https://90.16.41.190:443".parse().unwrap(),
            "https://38.242.231.212:443".parse().unwrap(),
            "https://38.143.58.210:443".parse().unwrap(),
            "https://157.66.81.130:443".parse().unwrap(),
            "https://217.77.12.102:443".parse().unwrap(),
            "https://157.10.199.125:443".parse().unwrap(),
            "https://46.254.241.8:443".parse().unwrap(),
            "https://49.12.102.105:443".parse().unwrap(),
            "https://134.255.182.189:443".parse().unwrap(),
            "https://81.17.101.141:443".parse().unwrap(),
            "https://64.23.134.67:443".parse().unwrap(),
            "https://93.190.140.190:443".parse().unwrap(),
            "https://86.107.101.128:443".parse().unwrap(),
            "https://54.69.95.118:443".parse().unwrap(),
            "https://158.220.122.13:443".parse().unwrap(),
            "https://82.211.25.69:443".parse().unwrap(),
            "https://144.217.69.169:443".parse().unwrap(),
            "https://93.190.140.111:443".parse().unwrap(),
            "https://5.189.140.20:443".parse().unwrap(),
            "https://93.190.140.114:443".parse().unwrap(),
            "https://135.181.110.216:443".parse().unwrap(),
            "https://207.180.213.141:443".parse().unwrap(),
            "https://45.76.141.74:443".parse().unwrap(),
            "https://185.194.216.38:443".parse().unwrap(),
            "https://161.97.66.31:443".parse().unwrap(),
            "https://188.245.90.255:443".parse().unwrap(),
            "https://65.109.84.201:443".parse().unwrap(),
            "https://164.68.114.36:443".parse().unwrap(),
            "https://167.88.165.175:443".parse().unwrap(),
            "https://43.167.239.145:443".parse().unwrap(),
            "https://37.60.236.201:443".parse().unwrap(),
            "https://185.239.208.110:443".parse().unwrap(),
            "https://95.179.139.125:443".parse().unwrap(),
            "https://213.199.34.248:443".parse().unwrap(),
            "https://178.18.254.136:443".parse().unwrap(),
            "https://82.211.21.40:443".parse().unwrap(),
            "https://213.199.35.18:443".parse().unwrap(),
            "https://38.102.124.86:443".parse().unwrap(),
            "https://45.77.129.235:443".parse().unwrap(),
            "https://81.0.249.58:443".parse().unwrap(),
            "https://37.60.243.59:443".parse().unwrap(),
            "https://37.60.236.247:443".parse().unwrap(),
            "https://89.35.131.218:443".parse().unwrap(),
            "https://5.189.145.80:443".parse().unwrap(),
            "https://149.102.152.219:443".parse().unwrap(),
            "https://77.221.148.204:443".parse().unwrap(),
            "https://46.254.241.11:443".parse().unwrap(),
            "https://207.180.218.245:443".parse().unwrap(),
            "https://89.35.131.158:443".parse().unwrap(),
            "https://5.252.55.188:443".parse().unwrap(),
            "https://185.215.166.126:443".parse().unwrap(),
            "https://164.132.55.103:443".parse().unwrap(),
            "https://162.250.190.133:443".parse().unwrap(),
            "https://157.66.81.218:443".parse().unwrap(),
            "https://5.39.27.224:443".parse().unwrap(),
            "https://213.159.77.221:443".parse().unwrap(),
            "https://213.199.35.15:443".parse().unwrap(),
            "https://114.132.172.215:443".parse().unwrap(),
        ];

        let address_list = dash_sdk::sdk::AddressList::from_iter(mainnet_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Dash)
            .with_context_provider(trusted_context);

        Ok(Self(sdk_builder))
    }

    #[wasm_bindgen(js_name = "testnet")]
    pub fn new_testnet() -> Self {
        // Testnet addresses from https://quorums.testnet.networks.dash.org/masternodes
        // Using HTTPS endpoints for ENABLED nodes with successful version checks
        let testnet_addresses = vec![
            "https://52.12.176.90:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://35.82.197.197:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://44.240.98.102:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://52.34.144.50:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://44.239.39.153:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://35.164.23.245:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://54.149.33.167:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://52.24.124.162:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
        ];

        let address_list = dash_sdk::sdk::AddressList::from_iter(testnet_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Testnet)
            .with_context_provider(WasmContext {});

        Self(sdk_builder)
    }

    #[wasm_bindgen(js_name = "testnetTrusted")]
    pub fn new_testnet_trusted() -> Result<Self, WasmSdkError> {
        use crate::context_provider::WasmTrustedContext;

        // Use the cached context if available, otherwise create a new one
        let trusted_context = {
            let guard = TESTNET_TRUSTED_CONTEXT.lock().unwrap();
            guard.clone()
        }
        .map(Ok)
        .unwrap_or_else(|| {
            WasmTrustedContext::new_testnet()
                .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))
        })?;

        // Testnet addresses from https://quorums.testnet.networks.dash.org/masternodes
        // Using HTTPS endpoints for ENABLED nodes with successful version checks
        let testnet_addresses = vec![
            "https://52.12.176.90:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://35.82.197.197:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://44.240.98.102:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://52.34.144.50:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://44.239.39.153:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://35.164.23.245:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://54.149.33.167:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
            "https://52.24.124.162:1443".parse().unwrap(), // ENABLED, dapiVersion: 2.0.0-rc.17
        ];

        let address_list = dash_sdk::sdk::AddressList::from_iter(testnet_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Testnet)
            .with_context_provider(trusted_context);

        Ok(Self(sdk_builder))
    }

    pub fn build(self) -> Result<WasmSdk, WasmSdkError> {
        self.0.build().map(WasmSdk).map_err(WasmSdkError::from)
    }

    #[wasm_bindgen(js_name = "withContextProvider")]
    pub fn with_context_provider(self, context_provider: WasmContext) -> Self {
        WasmSdkBuilder(self.0.with_context_provider(context_provider))
    }

    /// Configure platform version to use.
    ///
    /// Available versions:
    /// - 1: Platform version 1
    /// - 2: Platform version 2
    /// - ... up to latest version
    ///
    /// Defaults to latest version if not specified.
    #[wasm_bindgen(js_name = "withVersion")]
    pub fn with_version(self, version_number: u32) -> Result<Self, WasmSdkError> {
        let version = PlatformVersion::get(version_number).map_err(|e| {
            WasmSdkError::invalid_argument(format!(
                "Invalid platform version {}: {}",
                version_number, e
            ))
        })?;

        Ok(WasmSdkBuilder(self.0.with_version(version)))
    }

    /// Configure request settings for the SDK.
    ///
    /// Settings include:
    /// - connect_timeout_ms: Timeout for establishing connection (in milliseconds)
    /// - timeout_ms: Timeout for single request (in milliseconds)
    /// - retries: Number of retries in case of failed requests
    /// - ban_failed_address: Whether to ban DAPI address if node not responded or responded with error
    #[wasm_bindgen(js_name = "withSettings")]
    pub fn with_settings(
        self,
        connect_timeout_ms: Option<u32>,
        timeout_ms: Option<u32>,
        retries: Option<u32>,
        ban_failed_address: Option<bool>,
    ) -> Self {
        let mut settings = RequestSettings::default();

        if let Some(connect_timeout) = connect_timeout_ms {
            settings.connect_timeout = Some(Duration::from_millis(connect_timeout as u64));
        }

        if let Some(timeout) = timeout_ms {
            settings.timeout = Some(Duration::from_millis(timeout as u64));
        }

        if let Some(retries) = retries {
            settings.retries = Some(retries as usize);
        }

        if let Some(ban) = ban_failed_address {
            settings.ban_failed_address = Some(ban);
        }

        WasmSdkBuilder(self.0.with_settings(settings))
    }

    #[wasm_bindgen(js_name = "withProofs")]
    pub fn with_proofs(self, enable_proofs: bool) -> Self {
        WasmSdkBuilder(self.0.with_proofs(enable_proofs))
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Configure tracing/logging level or filter (static, global)
    ///
    /// Accepts simple levels: "off", "error", "warn", "info", "debug", "trace"
    /// or a full EnvFilter string like: "wasm_sdk=debug,rs_dapi_client=warn"
    #[wasm_bindgen(js_name = "setLogLevel")]
    pub fn set_log_level(level_or_filter: &str) -> Result<(), WasmSdkError> {
        crate::logging::set_log_level(level_or_filter)
    }
}

#[wasm_bindgen]
impl WasmSdkBuilder {
    /// Configure tracing/logging via the builder
    /// Returns a new builder with logging configured
    #[wasm_bindgen(js_name = "withLogs")]
    pub fn with_logs(self, level_or_filter: &str) -> Result<Self, WasmSdkError> {
        crate::logging::set_log_level(level_or_filter)?;
        Ok(self)
    }
}
