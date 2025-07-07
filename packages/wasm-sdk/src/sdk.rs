use crate::context_provider::WasmContext;
use dash_sdk::dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dash_sdk::dpp::dashcore::{Network, PrivateKey};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::DataContractFactory;
use dash_sdk::dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::IdentityV0;
use dash_sdk::dpp::prelude::AssetLockProof;
use dash_sdk::dpp::serialization::PlatformSerializableWithPlatformVersion;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::{DataContract, Document, DocumentQuery, Fetch, Identifier, Identity};
use dash_sdk::sdk::AddressList;
use dash_sdk::{Sdk, SdkBuilder};
use platform_value::platform_value;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsError;
use web_sys::{console, js_sys};

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
}

impl WasmSdk {
    /// Clone the inner Sdk (not exposed to WASM)
    pub(crate) fn inner_clone(&self) -> Sdk {
        self.0.clone()
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
    pub fn new_mainnet() -> Self {
        // Mainnet addresses from mnowatch.org
        let mainnet_addresses = vec![
            "https://149.28.241.190:1443".parse().unwrap(),
            "https://198.7.115.48:1443".parse().unwrap(),
            "https://134.255.182.186:1443".parse().unwrap(),
            "https://93.115.172.39:1443".parse().unwrap(),
            "https://5.189.164.253:1443".parse().unwrap(),
            "https://178.215.237.134:1443".parse().unwrap(),
            "https://157.66.81.162:1443".parse().unwrap(),
            "https://173.212.232.90:1443".parse().unwrap(),
            "https://178.215.237.135:1443".parse().unwrap(),
            "https://5.182.33.231:1443".parse().unwrap(),
            "https://109.199.104.243:1443".parse().unwrap(),
            "https://37.60.236.212:1443".parse().unwrap(),
            "https://23.88.63.58:1443".parse().unwrap(),
            "https://207.244.247.40:1443".parse().unwrap(),
            "https://45.32.70.131:1443".parse().unwrap(),
            "https://158.220.122.76:1443".parse().unwrap(),
            "https://52.33.9.172:1443".parse().unwrap(),
            "https://194.163.166.185:1443".parse().unwrap(),
            "https://185.158.107.124:1443".parse().unwrap(),
            "https://185.198.234.17:1443".parse().unwrap(),
            "https://93.190.140.101:1443".parse().unwrap(),
            "https://194.163.153.225:1443".parse().unwrap(),
            "https://194.146.13.7:1443".parse().unwrap(),
            "https://158.247.208.247:1443".parse().unwrap(),
            "https://93.190.140.112:1443".parse().unwrap(),
            "https://75.119.132.2:1443".parse().unwrap(),
            "https://173.212.239.247:1443".parse().unwrap(),
            "https://51.38.142.61:1443".parse().unwrap(),
            "https://44.240.99.214:1443".parse().unwrap(),
            "https://5.75.133.148:1443".parse().unwrap(),
            "https://62.84.182.155:1443".parse().unwrap(),
            "https://89.35.131.149:1443".parse().unwrap(),
            "https://192.248.178.237:1443".parse().unwrap(),
            "https://45.77.11.194:1443".parse().unwrap(),
            "https://37.60.243.119:1443".parse().unwrap(),
            "https://46.254.241.7:1443".parse().unwrap(),
            "https://194.195.87.34:1443".parse().unwrap(),
            "https://43.163.251.51:1443".parse().unwrap(),
            "https://184.174.36.201:1443".parse().unwrap(),
            "https://85.239.244.103:1443".parse().unwrap(),
            "https://65.108.246.145:1443".parse().unwrap(),
            "https://194.163.170.55:1443".parse().unwrap(),
            "https://2.58.82.231:1443".parse().unwrap(),
            "https://5.252.55.187:1443".parse().unwrap(),
            "https://198.7.119.139:1443".parse().unwrap(),
            "https://213.199.44.112:1443".parse().unwrap(),
            "https://155.138.220.69:1443".parse().unwrap(),
            "https://209.145.48.154:1443".parse().unwrap(),
            "https://162.212.35.100:1443".parse().unwrap(),
            "https://185.239.209.6:1443".parse().unwrap(),
            "https://157.173.113.158:1443".parse().unwrap(),
            "https://134.255.182.185:1443".parse().unwrap(),
            "https://173.212.239.124:1443".parse().unwrap(),
            "https://144.126.141.62:1443".parse().unwrap(),
            "https://51.38.142.62:1443".parse().unwrap(),
            "https://157.10.199.77:1443".parse().unwrap(),
            "https://5.189.186.78:1443".parse().unwrap(),
            "https://164.68.118.37:1443".parse().unwrap(),
            "https://158.220.92.144:1443".parse().unwrap(),
            "https://192.248.175.198:1443".parse().unwrap(),
            "https://43.167.244.109:1443".parse().unwrap(),
            "https://146.59.45.235:1443".parse().unwrap(),
            "https://104.200.24.196:1443".parse().unwrap(),
            "https://146.59.153.204:1443".parse().unwrap(),
            "https://37.60.236.225:1443".parse().unwrap(),
            "https://172.233.66.70:1443".parse().unwrap(),
            "https://57.128.212.163:1443".parse().unwrap(),
            "https://82.208.20.153:1443".parse().unwrap(),
            "https://51.195.235.166:1443".parse().unwrap(),
            "https://158.220.122.74:1443".parse().unwrap(),
            "https://82.211.21.38:1443".parse().unwrap(),
            "https://93.115.172.37:1443".parse().unwrap(),
            "https://185.198.234.25:1443".parse().unwrap(),
            "https://84.247.187.76:1443".parse().unwrap(),
            "https://89.35.131.39:1443".parse().unwrap(),
            "https://93.115.172.38:1443".parse().unwrap(),
            "https://134.255.183.250:1443".parse().unwrap(),
            "https://85.190.243.3:1443".parse().unwrap(),
            "https://185.192.96.70:1443".parse().unwrap(),
            "https://134.255.183.248:1443".parse().unwrap(),
            "https://52.36.102.91:1443".parse().unwrap(),
            "https://139.99.201.103:1443".parse().unwrap(),
            "https://134.255.183.247:1443".parse().unwrap(),
            "https://213.199.34.250:1443".parse().unwrap(),
            "https://161.97.74.173:1443".parse().unwrap(),
            "https://45.135.180.79:1443".parse().unwrap(),
            "https://45.135.180.130:1443".parse().unwrap(),
            "https://173.212.251.130:1443".parse().unwrap(),
            "https://157.173.122.157:1443".parse().unwrap(),
            "https://49.13.237.193:1443".parse().unwrap(),
            "https://37.27.83.17:1443".parse().unwrap(),
            "https://45.135.180.114:1443".parse().unwrap(),
            "https://89.35.131.61:1443".parse().unwrap(),
            "https://86.107.101.74:1443".parse().unwrap(),
            "https://134.255.182.187:1443".parse().unwrap(),
            "https://157.173.202.14:1443".parse().unwrap(),
            "https://62.171.170.14:1443".parse().unwrap(),
            "https://5.252.55.190:1443".parse().unwrap(),
            "https://198.7.115.43:1443".parse().unwrap(),
            "https://157.173.122.158:1443".parse().unwrap(),
            "https://108.61.165.170:1443".parse().unwrap(),
            "https://157.10.199.79:1443".parse().unwrap(),
            "https://89.35.131.219:1443".parse().unwrap(),
            "https://185.166.217.154:1443".parse().unwrap(),
            "https://31.220.88.116:1443".parse().unwrap(),
            "https://149.202.78.214:1443".parse().unwrap(),
            "https://195.26.254.228:1443".parse().unwrap(),
            "https://217.77.12.101:1443".parse().unwrap(),
            "https://43.167.240.90:1443".parse().unwrap(),
            "https://157.10.199.82:1443".parse().unwrap(),
            "https://5.252.55.189:1443".parse().unwrap(),
            "https://167.86.93.21:1443".parse().unwrap(),
            "https://195.26.241.252:1443".parse().unwrap(),
            "https://161.97.170.251:1443".parse().unwrap(),
            "https://51.195.47.118:1443".parse().unwrap(),
            "https://45.135.180.70:1443".parse().unwrap(),
            "https://167.88.169.16:1443".parse().unwrap(),
            "https://62.169.17.112:1443".parse().unwrap(),
            "https://82.211.21.18:1443".parse().unwrap(),
            "https://52.10.213.198:1443".parse().unwrap(),
            "https://139.84.231.221:1443".parse().unwrap(),
            "https://51.75.60.227:1443".parse().unwrap(),
            "https://93.190.140.162:1443".parse().unwrap(),
            "https://198.7.115.38:1443".parse().unwrap(),
            "https://37.60.236.161:1443".parse().unwrap(),
            "https://37.60.244.220:1443".parse().unwrap(),
            "https://46.254.241.9:1443".parse().unwrap(),
            "https://167.86.94.138:1443".parse().unwrap(),
            "https://192.95.32.205:1443".parse().unwrap(),
            "https://95.179.241.182:1443".parse().unwrap(),
            "https://65.109.84.204:1443".parse().unwrap(),
            "https://93.115.172.36:1443".parse().unwrap(),
            "https://82.211.21.16:1443".parse().unwrap(),
            "https://158.220.89.188:1443".parse().unwrap(),
            "https://95.216.146.18:1443".parse().unwrap(),
            "https://167.114.153.110:1443".parse().unwrap(),
            "https://89.250.75.61:1443".parse().unwrap(),
            "https://185.194.216.84:1443".parse().unwrap(),
            "https://158.220.87.156:1443".parse().unwrap(),
            "https://31.220.84.93:1443".parse().unwrap(),
            "https://185.197.250.227:1443".parse().unwrap(),
            "https://162.250.188.207:1443".parse().unwrap(),
            "https://207.180.231.37:1443".parse().unwrap(),
            "https://207.180.231.39:1443".parse().unwrap(),
            "https://66.70.170.22:1443".parse().unwrap(),
            "https://149.28.247.165:1443".parse().unwrap(),
            "https://45.85.147.192:1443".parse().unwrap(),
            "https://157.173.122.156:1443".parse().unwrap(),
            "https://213.199.34.251:1443".parse().unwrap(),
            "https://95.171.21.131:1443".parse().unwrap(),
            "https://87.228.24.64:1443".parse().unwrap(),
            "https://5.189.151.7:1443".parse().unwrap(),
            "https://90.16.41.190:1443".parse().unwrap(),
            "https://38.242.231.212:1443".parse().unwrap(),
            "https://38.143.58.210:1443".parse().unwrap(),
            "https://157.66.81.130:1443".parse().unwrap(),
            "https://217.77.12.102:1443".parse().unwrap(),
            "https://157.10.199.125:1443".parse().unwrap(),
            "https://46.254.241.8:1443".parse().unwrap(),
            "https://49.12.102.105:1443".parse().unwrap(),
            "https://134.255.182.189:1443".parse().unwrap(),
            "https://81.17.101.141:1443".parse().unwrap(),
            "https://64.23.134.67:1443".parse().unwrap(),
            "https://93.190.140.190:1443".parse().unwrap(),
            "https://86.107.101.128:1443".parse().unwrap(),
            "https://54.69.95.118:1443".parse().unwrap(),
            "https://158.220.122.13:1443".parse().unwrap(),
            "https://82.211.25.69:1443".parse().unwrap(),
            "https://144.217.69.169:1443".parse().unwrap(),
            "https://93.190.140.111:1443".parse().unwrap(),
            "https://5.189.140.20:1443".parse().unwrap(),
            "https://93.190.140.114:1443".parse().unwrap(),
            "https://135.181.110.216:1443".parse().unwrap(),
            "https://207.180.213.141:1443".parse().unwrap(),
            "https://45.76.141.74:1443".parse().unwrap(),
            "https://185.194.216.38:1443".parse().unwrap(),
            "https://161.97.66.31:1443".parse().unwrap(),
            "https://188.245.90.255:1443".parse().unwrap(),
            "https://65.109.84.201:1443".parse().unwrap(),
            "https://164.68.114.36:1443".parse().unwrap(),
            "https://167.88.165.175:1443".parse().unwrap(),
            "https://43.167.239.145:1443".parse().unwrap(),
            "https://37.60.236.201:1443".parse().unwrap(),
            "https://185.239.208.110:1443".parse().unwrap(),
            "https://95.179.139.125:1443".parse().unwrap(),
            "https://213.199.34.248:1443".parse().unwrap(),
            "https://178.18.254.136:1443".parse().unwrap(),
            "https://82.211.21.40:1443".parse().unwrap(),
            "https://213.199.35.18:1443".parse().unwrap(),
            "https://38.102.124.86:1443".parse().unwrap(),
            "https://45.77.129.235:1443".parse().unwrap(),
            "https://81.0.249.58:1443".parse().unwrap(),
            "https://37.60.243.59:1443".parse().unwrap(),
            "https://37.60.236.247:1443".parse().unwrap(),
            "https://89.35.131.218:1443".parse().unwrap(),
            "https://5.189.145.80:1443".parse().unwrap(),
            "https://149.102.152.219:1443".parse().unwrap(),
            "https://77.221.148.204:1443".parse().unwrap(),
            "https://46.254.241.11:1443".parse().unwrap(),
            "https://207.180.218.245:1443".parse().unwrap(),
            "https://89.35.131.158:1443".parse().unwrap(),
            "https://5.252.55.188:1443".parse().unwrap(),
            "https://185.215.166.126:1443".parse().unwrap(),
            "https://164.132.55.103:1443".parse().unwrap(),
            "https://162.250.190.133:1443".parse().unwrap(),
            "https://157.66.81.218:1443".parse().unwrap(),
            "https://5.39.27.224:1443".parse().unwrap(),
            "https://213.159.77.221:1443".parse().unwrap(),
            "https://213.199.35.15:1443".parse().unwrap(),
            "https://114.132.172.215:1443".parse().unwrap(),
        ];

        let address_list = dash_sdk::sdk::AddressList::from_iter(mainnet_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Dash)
            .with_context_provider(WasmContext {});

        Self(sdk_builder)
    }

    pub fn new_mainnet_trusted() -> Result<Self, JsError> {
        use crate::context_provider::WasmTrustedContext;

        // Use the cached context if available, otherwise create a new one
        let trusted_context = {
            let guard = MAINNET_TRUSTED_CONTEXT.lock().unwrap();
            guard.clone()
        }
        .map(Ok)
        .unwrap_or_else(|| {
            WasmTrustedContext::new_mainnet().map_err(|e| {
                JsError::new(&format!("Failed to create mainnet trusted context: {}", e))
            })
        })?;

        // Mainnet addresses from mnowatch.org
        let mainnet_addresses = vec![
            "https://149.28.241.190:1443".parse().unwrap(),
            "https://198.7.115.48:1443".parse().unwrap(),
            "https://134.255.182.186:1443".parse().unwrap(),
            "https://93.115.172.39:1443".parse().unwrap(),
            "https://5.189.164.253:1443".parse().unwrap(),
            "https://178.215.237.134:1443".parse().unwrap(),
            "https://157.66.81.162:1443".parse().unwrap(),
            "https://173.212.232.90:1443".parse().unwrap(),
            "https://178.215.237.135:1443".parse().unwrap(),
            "https://5.182.33.231:1443".parse().unwrap(),
            "https://109.199.104.243:1443".parse().unwrap(),
            "https://37.60.236.212:1443".parse().unwrap(),
            "https://23.88.63.58:1443".parse().unwrap(),
            "https://207.244.247.40:1443".parse().unwrap(),
            "https://45.32.70.131:1443".parse().unwrap(),
            "https://158.220.122.76:1443".parse().unwrap(),
            "https://52.33.9.172:1443".parse().unwrap(),
            "https://194.163.166.185:1443".parse().unwrap(),
            "https://185.158.107.124:1443".parse().unwrap(),
            "https://185.198.234.17:1443".parse().unwrap(),
            "https://93.190.140.101:1443".parse().unwrap(),
            "https://194.163.153.225:1443".parse().unwrap(),
            "https://194.146.13.7:1443".parse().unwrap(),
            "https://158.247.208.247:1443".parse().unwrap(),
            "https://93.190.140.112:1443".parse().unwrap(),
            "https://75.119.132.2:1443".parse().unwrap(),
            "https://173.212.239.247:1443".parse().unwrap(),
            "https://51.38.142.61:1443".parse().unwrap(),
            "https://44.240.99.214:1443".parse().unwrap(),
            "https://5.75.133.148:1443".parse().unwrap(),
            "https://62.84.182.155:1443".parse().unwrap(),
            "https://89.35.131.149:1443".parse().unwrap(),
            "https://192.248.178.237:1443".parse().unwrap(),
            "https://45.77.11.194:1443".parse().unwrap(),
            "https://37.60.243.119:1443".parse().unwrap(),
            "https://46.254.241.7:1443".parse().unwrap(),
            "https://194.195.87.34:1443".parse().unwrap(),
            "https://43.163.251.51:1443".parse().unwrap(),
            "https://184.174.36.201:1443".parse().unwrap(),
            "https://85.239.244.103:1443".parse().unwrap(),
            "https://65.108.246.145:1443".parse().unwrap(),
            "https://194.163.170.55:1443".parse().unwrap(),
            "https://2.58.82.231:1443".parse().unwrap(),
            "https://5.252.55.187:1443".parse().unwrap(),
            "https://198.7.119.139:1443".parse().unwrap(),
            "https://213.199.44.112:1443".parse().unwrap(),
            "https://155.138.220.69:1443".parse().unwrap(),
            "https://209.145.48.154:1443".parse().unwrap(),
            "https://162.212.35.100:1443".parse().unwrap(),
            "https://185.239.209.6:1443".parse().unwrap(),
            "https://157.173.113.158:1443".parse().unwrap(),
            "https://134.255.182.185:1443".parse().unwrap(),
            "https://173.212.239.124:1443".parse().unwrap(),
            "https://144.126.141.62:1443".parse().unwrap(),
            "https://51.38.142.62:1443".parse().unwrap(),
            "https://157.10.199.77:1443".parse().unwrap(),
            "https://5.189.186.78:1443".parse().unwrap(),
            "https://164.68.118.37:1443".parse().unwrap(),
            "https://158.220.92.144:1443".parse().unwrap(),
            "https://192.248.175.198:1443".parse().unwrap(),
            "https://43.167.244.109:1443".parse().unwrap(),
            "https://146.59.45.235:1443".parse().unwrap(),
            "https://104.200.24.196:1443".parse().unwrap(),
            "https://146.59.153.204:1443".parse().unwrap(),
            "https://37.60.236.225:1443".parse().unwrap(),
            "https://172.233.66.70:1443".parse().unwrap(),
            "https://57.128.212.163:1443".parse().unwrap(),
            "https://82.208.20.153:1443".parse().unwrap(),
            "https://51.195.235.166:1443".parse().unwrap(),
            "https://158.220.122.74:1443".parse().unwrap(),
            "https://82.211.21.38:1443".parse().unwrap(),
            "https://93.115.172.37:1443".parse().unwrap(),
            "https://185.198.234.25:1443".parse().unwrap(),
            "https://84.247.187.76:1443".parse().unwrap(),
            "https://89.35.131.39:1443".parse().unwrap(),
            "https://93.115.172.38:1443".parse().unwrap(),
            "https://134.255.183.250:1443".parse().unwrap(),
            "https://85.190.243.3:1443".parse().unwrap(),
            "https://185.192.96.70:1443".parse().unwrap(),
            "https://134.255.183.248:1443".parse().unwrap(),
            "https://52.36.102.91:1443".parse().unwrap(),
            "https://139.99.201.103:1443".parse().unwrap(),
            "https://134.255.183.247:1443".parse().unwrap(),
            "https://213.199.34.250:1443".parse().unwrap(),
            "https://161.97.74.173:1443".parse().unwrap(),
            "https://45.135.180.79:1443".parse().unwrap(),
            "https://45.135.180.130:1443".parse().unwrap(),
            "https://173.212.251.130:1443".parse().unwrap(),
            "https://157.173.122.157:1443".parse().unwrap(),
            "https://49.13.237.193:1443".parse().unwrap(),
            "https://37.27.83.17:1443".parse().unwrap(),
            "https://45.135.180.114:1443".parse().unwrap(),
            "https://89.35.131.61:1443".parse().unwrap(),
            "https://86.107.101.74:1443".parse().unwrap(),
            "https://134.255.182.187:1443".parse().unwrap(),
            "https://157.173.202.14:1443".parse().unwrap(),
            "https://62.171.170.14:1443".parse().unwrap(),
            "https://5.252.55.190:1443".parse().unwrap(),
            "https://198.7.115.43:1443".parse().unwrap(),
            "https://157.173.122.158:1443".parse().unwrap(),
            "https://108.61.165.170:1443".parse().unwrap(),
            "https://157.10.199.79:1443".parse().unwrap(),
            "https://89.35.131.219:1443".parse().unwrap(),
            "https://185.166.217.154:1443".parse().unwrap(),
            "https://31.220.88.116:1443".parse().unwrap(),
            "https://149.202.78.214:1443".parse().unwrap(),
            "https://195.26.254.228:1443".parse().unwrap(),
            "https://217.77.12.101:1443".parse().unwrap(),
            "https://43.167.240.90:1443".parse().unwrap(),
            "https://157.10.199.82:1443".parse().unwrap(),
            "https://5.252.55.189:1443".parse().unwrap(),
            "https://167.86.93.21:1443".parse().unwrap(),
            "https://195.26.241.252:1443".parse().unwrap(),
            "https://161.97.170.251:1443".parse().unwrap(),
            "https://51.195.47.118:1443".parse().unwrap(),
            "https://45.135.180.70:1443".parse().unwrap(),
            "https://167.88.169.16:1443".parse().unwrap(),
            "https://62.169.17.112:1443".parse().unwrap(),
            "https://82.211.21.18:1443".parse().unwrap(),
            "https://52.10.213.198:1443".parse().unwrap(),
            "https://139.84.231.221:1443".parse().unwrap(),
            "https://51.75.60.227:1443".parse().unwrap(),
            "https://93.190.140.162:1443".parse().unwrap(),
            "https://198.7.115.38:1443".parse().unwrap(),
            "https://37.60.236.161:1443".parse().unwrap(),
            "https://37.60.244.220:1443".parse().unwrap(),
            "https://46.254.241.9:1443".parse().unwrap(),
            "https://167.86.94.138:1443".parse().unwrap(),
            "https://192.95.32.205:1443".parse().unwrap(),
            "https://95.179.241.182:1443".parse().unwrap(),
            "https://65.109.84.204:1443".parse().unwrap(),
            "https://93.115.172.36:1443".parse().unwrap(),
            "https://82.211.21.16:1443".parse().unwrap(),
            "https://158.220.89.188:1443".parse().unwrap(),
            "https://95.216.146.18:1443".parse().unwrap(),
            "https://167.114.153.110:1443".parse().unwrap(),
            "https://89.250.75.61:1443".parse().unwrap(),
            "https://185.194.216.84:1443".parse().unwrap(),
            "https://158.220.87.156:1443".parse().unwrap(),
            "https://31.220.84.93:1443".parse().unwrap(),
            "https://185.197.250.227:1443".parse().unwrap(),
            "https://162.250.188.207:1443".parse().unwrap(),
            "https://207.180.231.37:1443".parse().unwrap(),
            "https://207.180.231.39:1443".parse().unwrap(),
            "https://66.70.170.22:1443".parse().unwrap(),
            "https://149.28.247.165:1443".parse().unwrap(),
            "https://45.85.147.192:1443".parse().unwrap(),
            "https://157.173.122.156:1443".parse().unwrap(),
            "https://213.199.34.251:1443".parse().unwrap(),
            "https://95.171.21.131:1443".parse().unwrap(),
            "https://87.228.24.64:1443".parse().unwrap(),
            "https://5.189.151.7:1443".parse().unwrap(),
            "https://90.16.41.190:1443".parse().unwrap(),
            "https://38.242.231.212:1443".parse().unwrap(),
            "https://38.143.58.210:1443".parse().unwrap(),
            "https://157.66.81.130:1443".parse().unwrap(),
            "https://217.77.12.102:1443".parse().unwrap(),
            "https://157.10.199.125:1443".parse().unwrap(),
            "https://46.254.241.8:1443".parse().unwrap(),
            "https://49.12.102.105:1443".parse().unwrap(),
            "https://134.255.182.189:1443".parse().unwrap(),
            "https://81.17.101.141:1443".parse().unwrap(),
            "https://64.23.134.67:1443".parse().unwrap(),
            "https://93.190.140.190:1443".parse().unwrap(),
            "https://86.107.101.128:1443".parse().unwrap(),
            "https://54.69.95.118:1443".parse().unwrap(),
            "https://158.220.122.13:1443".parse().unwrap(),
            "https://82.211.25.69:1443".parse().unwrap(),
            "https://144.217.69.169:1443".parse().unwrap(),
            "https://93.190.140.111:1443".parse().unwrap(),
            "https://5.189.140.20:1443".parse().unwrap(),
            "https://93.190.140.114:1443".parse().unwrap(),
            "https://135.181.110.216:1443".parse().unwrap(),
            "https://207.180.213.141:1443".parse().unwrap(),
            "https://45.76.141.74:1443".parse().unwrap(),
            "https://185.194.216.38:1443".parse().unwrap(),
            "https://161.97.66.31:1443".parse().unwrap(),
            "https://188.245.90.255:1443".parse().unwrap(),
            "https://65.109.84.201:1443".parse().unwrap(),
            "https://164.68.114.36:1443".parse().unwrap(),
            "https://167.88.165.175:1443".parse().unwrap(),
            "https://43.167.239.145:1443".parse().unwrap(),
            "https://37.60.236.201:1443".parse().unwrap(),
            "https://185.239.208.110:1443".parse().unwrap(),
            "https://95.179.139.125:1443".parse().unwrap(),
            "https://213.199.34.248:1443".parse().unwrap(),
            "https://178.18.254.136:1443".parse().unwrap(),
            "https://82.211.21.40:1443".parse().unwrap(),
            "https://213.199.35.18:1443".parse().unwrap(),
            "https://38.102.124.86:1443".parse().unwrap(),
            "https://45.77.129.235:1443".parse().unwrap(),
            "https://81.0.249.58:1443".parse().unwrap(),
            "https://37.60.243.59:1443".parse().unwrap(),
            "https://37.60.236.247:1443".parse().unwrap(),
            "https://89.35.131.218:1443".parse().unwrap(),
            "https://5.189.145.80:1443".parse().unwrap(),
            "https://149.102.152.219:1443".parse().unwrap(),
            "https://77.221.148.204:1443".parse().unwrap(),
            "https://46.254.241.11:1443".parse().unwrap(),
            "https://207.180.218.245:1443".parse().unwrap(),
            "https://89.35.131.158:1443".parse().unwrap(),
            "https://5.252.55.188:1443".parse().unwrap(),
            "https://185.215.166.126:1443".parse().unwrap(),
            "https://164.132.55.103:1443".parse().unwrap(),
            "https://162.250.190.133:1443".parse().unwrap(),
            "https://157.66.81.218:1443".parse().unwrap(),
            "https://5.39.27.224:1443".parse().unwrap(),
            "https://213.159.77.221:1443".parse().unwrap(),
            "https://213.199.35.15:1443".parse().unwrap(),
            "https://114.132.172.215:1443".parse().unwrap(),
        ];

        let address_list = dash_sdk::sdk::AddressList::from_iter(mainnet_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Dash)
            .with_context_provider(trusted_context);

        Ok(Self(sdk_builder))
    }

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

    pub fn new_testnet_trusted() -> Result<Self, JsError> {
        use crate::context_provider::WasmTrustedContext;

        // Use the cached context if available, otherwise create a new one
        let trusted_context = {
            let guard = TESTNET_TRUSTED_CONTEXT.lock().unwrap();
            guard.clone()
        }
        .map(Ok)
        .unwrap_or_else(|| {
            WasmTrustedContext::new_testnet().map_err(|e| {
                JsError::new(&format!("Failed to create testnet trusted context: {}", e))
            })
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

    pub fn build(self) -> Result<WasmSdk, JsError> {
        Ok(WasmSdk(self.0.build()?))
    }

    pub fn with_context_provider(self, context_provider: WasmContext) -> Self {
        WasmSdkBuilder(self.0.with_context_provider(context_provider))
    }
}

// Store shared trusted contexts
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub(crate) static MAINNET_TRUSTED_CONTEXT: Lazy<Mutex<Option<crate::context_provider::WasmTrustedContext>>> =
    Lazy::new(|| Mutex::new(None));
pub(crate) static TESTNET_TRUSTED_CONTEXT: Lazy<Mutex<Option<crate::context_provider::WasmTrustedContext>>> =
    Lazy::new(|| Mutex::new(None));

#[wasm_bindgen]
pub async fn prefetch_trusted_quorums_mainnet() -> Result<(), JsError> {
    use crate::context_provider::WasmTrustedContext;

    let trusted_context = WasmTrustedContext::new_mainnet()
        .map_err(|e| JsError::new(&format!("Failed to create trusted context: {}", e)))?;

    trusted_context
        .prefetch_quorums()
        .await
        .map_err(|e| JsError::new(&format!("Failed to prefetch quorums: {}", e)))?;

    // Store the context for later use
    *MAINNET_TRUSTED_CONTEXT.lock().unwrap() = Some(trusted_context);

    Ok(())
}

#[wasm_bindgen]
pub async fn prefetch_trusted_quorums_testnet() -> Result<(), JsError> {
    use crate::context_provider::WasmTrustedContext;

    let trusted_context = WasmTrustedContext::new_testnet()
        .map_err(|e| JsError::new(&format!("Failed to create trusted context: {}", e)))?;

    trusted_context
        .prefetch_quorums()
        .await
        .map_err(|e| JsError::new(&format!("Failed to prefetch quorums: {}", e)))?;

    // Store the context for later use
    *TESTNET_TRUSTED_CONTEXT.lock().unwrap() = Some(trusted_context);

    Ok(())
}

// Query functions have been moved to src/queries/ modules

#[wasm_bindgen]
pub async fn identity_put(sdk: &WasmSdk) {
    // This is just a mock implementation to show how to use the SDK and ensure proper linking
    // of all required dependencies. This function is not supposed to work.
    let id = Identifier::from_bytes(&[0; 32]).expect("create identifier");

    let identity = Identity::V0(IdentityV0 {
        id,
        public_keys: BTreeMap::new(),
        balance: 0,
        revision: 0,
    });

    let asset_lock_proof = AssetLockProof::default();
    let asset_lock_proof_private_key =
        PrivateKey::from_slice(&[0; 32], Network::Testnet).expect("create private key");

    let signer = MockSigner;
    let _pushed: Identity = identity
        .put_to_platform(
            sdk,
            asset_lock_proof,
            &asset_lock_proof_private_key,
            &signer,
            None,
        )
        .await
        .expect("put identity")
        .broadcast_and_wait(sdk, None)
        .await
        .unwrap();
}

#[wasm_bindgen]
pub async fn epoch_testing() {
    let sdk = SdkBuilder::new(AddressList::new())
        .build()
        .expect("build sdk");

    let _ei = ExtendedEpochInfo::fetch(&sdk, 0)
        .await
        .expect("fetch extended epoch info")
        .expect("extended epoch info not found");
}

#[wasm_bindgen]
pub async fn docs_testing(sdk: &WasmSdk) {
    let id = Identifier::random();

    let factory = DataContractFactory::new(1).expect("create data contract factory");
    factory
        .create(id, 1, platform_value!({}), None, None)
        .expect("create data contract");

    let dc = DataContract::fetch(sdk, id)
        .await
        .expect("fetch data contract")
        .expect("data contract not found");

    let dcs = dc
        .serialize_to_bytes_with_platform_version(sdk.0.version())
        .expect("serialize data contract");

    let query = DocumentQuery::new(dc.clone(), "asd").expect("create query");
    let doc = Document::fetch(sdk, query)
        .await
        .expect("fetch document")
        .expect("document not found");

    let document_type = dc
        .document_type_for_name("aaa")
        .expect("document type for name");
    let doc_serialized = doc
        .serialize(document_type, &dc, sdk.0.version())
        .expect("serialize document");

    let msg = js_sys::JsString::from_str(&format!("{:?} {:?} ", dcs, doc_serialized))
        .expect("create js string");
    console::log_1(&msg);
}

#[derive(Clone, Debug)]
struct MockSigner;
impl Signer for MockSigner {
    fn can_sign_with(&self, _identity_public_key: &dash_sdk::platform::IdentityPublicKey) -> bool {
        true
    }
    fn sign(
        &self,
        _identity_public_key: &dash_sdk::platform::IdentityPublicKey,
        _data: &[u8],
    ) -> Result<dash_sdk::dpp::platform_value::BinaryData, dash_sdk::dpp::ProtocolError> {
        todo!("signature creation is not implemented due to lack of dash platform wallet support in wasm")
    }
}
