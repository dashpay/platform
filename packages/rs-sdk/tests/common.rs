pub const PLATFORM_IP: &str = "10.56.229.104";
pub const CORE_PORT: u16 = 30002;
pub const CORE_USER: &str = "PdXjj4HC";
pub const CORE_PASSWORD: &str = "POv4lqSbzO7m";
pub const PLATFORM_PORT: u16 = 2443;

/// Existing identity ID, created as part of platform test suite run
pub const IDENTITY_ID_BYTES: [u8; 32] = [
    65, 63, 57, 243, 204, 9, 106, 71, 187, 2, 94, 221, 190, 127, 141, 114, 137, 209, 243, 50, 60,
    215, 90, 101, 229, 15, 115, 5, 44, 117, 182, 217,
];

// allow unused because we just include() this file, and the functions are used in other files
#[allow(unused)]
fn setup_api() -> rs_sdk::Sdk {
    use rs_dapi_client::AddressList;
    use rs_sdk::core::CoreClient;

    let uri = http::Uri::from_maybe_shared(format!("http://{}:{}", PLATFORM_IP, PLATFORM_PORT))
        .expect("platform address");
    let addresses = AddressList::from(vec![uri]);
    let api = rs_sdk::SdkBuilder::new(addresses)
        .with_core(PLATFORM_IP, CORE_PORT, CORE_USER, CORE_PASSWORD)
        .build()
        .expect("cannot initialize api");

    api
}

// allow unused because we just include() this file, and the functions are used in other files
#[allow(unused)]
#[cfg(feature = "mocks")]
async fn setup_mock_api() -> rs_sdk::Sdk {
    rs_sdk::SdkBuilder::new_mock()
        .build()
        .expect("cannot initialize mock sdk")
}

// allow unused because we just include() this file, and the functions are used in other files
#[allow(unused)]
fn base64_identifier(base64str: &str) -> dpp::prelude::Identifier {
    let b64 = base64::engine::general_purpose::STANDARD;
    let bytes = base64::Engine::decode(&b64, base64str).expect("base64 decode identifier");
    dpp::prelude::Identifier::from_bytes(bytes.as_ref()).expect("invalid identifier format")
}
