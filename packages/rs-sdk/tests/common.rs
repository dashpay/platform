use rs_dapi_client::AddressList;
use rs_sdk::{core::CoreClient, sdk::Sdk};

pub const PLATFORM_IP: &str = "10.56.229.104";
pub const CORE_PORT: u16 = 30002;
pub const CORE_USER: &str = "PdXjj4HC";
pub const CORE_PASSWORD: &str = "POv4lqSbzO7m";
pub const PLATFORM_PORT: u16 = 2443;

// allow unused because we just include() this file, and the functions are used in other files
#[allow(unused)]
fn setup_api() -> impl Sdk {
    let core = CoreClient::new(PLATFORM_IP, CORE_PORT, CORE_USER, CORE_PASSWORD)
        .expect("core not initialized");
    let uri = http::Uri::from_maybe_shared(format!("http://{}:{}", PLATFORM_IP, PLATFORM_PORT))
        .expect("platform address");
    let addresses = AddressList::from(vec![uri]);
    let api = rs_sdk::sdk::DashPlatformSdk::new(addresses, Box::new(core))
        .expect("cannot initialize api");

    api
}

// allow unused because we just include() this file, and the functions are used in other files
#[allow(unused)]
fn base64_identifier(base64str: &str) -> dpp::prelude::Identifier {
    let b64 = base64::engine::general_purpose::STANDARD;
    let bytes = base64::Engine::decode(&b64, base64str).expect("base64 decode identifier");
    dpp::prelude::Identifier::from_bytes(bytes.as_ref()).expect("invalid identifier format")
}
