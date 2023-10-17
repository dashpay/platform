use dpp::prelude::{DataContract, Identifier};
use rs_dapi_client::AddressList;
use rs_sdk::platform::Fetch;

// Some constants we need to connect to the platform
include!("../tests/fetch/common.rs");

/// Read data contract.
///
/// This example demonstrates how to connect to running platform and try to read a data contract.
#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    // Replace const below with data contract identifier of data contract, 32 bytes
    const DATA_CONTRACT_ID_BYTES: [u8; 32] = [1; 32];

    // Create an instance of the SDK; we use constants defined in `common.rs` for convenience.
    // Note that in future versions of the SDK, core user and password will not be needed.
    let uri = http::Uri::from_maybe_shared(format!("http://{}:{}", PLATFORM_IP, PLATFORM_PORT))
        .expect("platform address uri");
    let mut sdk = rs_sdk::SdkBuilder::new(AddressList::from_iter(vec![uri]))
        .with_core(PLATFORM_IP, CORE_PORT, CORE_USER, CORE_PASSWORD)
        .build()
        .expect("cannot initialize api");

    // Convert bytes to identifier object that can be used as a Query
    let id = Identifier::from_bytes(&DATA_CONTRACT_ID_BYTES).expect("parse identity id");

    // Execute the fetch operation
    let result = DataContract::fetch(&mut sdk, id).await;

    // Check the result; note that in our case, we expect to not find the data contract, as the
    // identifier is not valid.
    assert!(matches!(result, Ok(None)), "result: {:?}", result);
}
