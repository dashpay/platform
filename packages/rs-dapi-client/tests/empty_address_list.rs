//! A [DapiClient] built on an empty address list executes requests on
//! addresses added to the shared list after construction.

mod common;

use common::{FakeResponse, ScriptedRequest};
use rs_dapi_client::{Address, AddressList, DapiClient, DapiRequestExecutor, RequestSettings};

#[tokio::test]
async fn executes_on_address_added_after_construction() {
    let client = DapiClient::new(AddressList::new(), RequestSettings::default());
    let request = ScriptedRequest::new(|_uri| Ok(FakeResponse));

    let address: Address = "http://127.0.0.1:10001".parse().expect("valid address");
    let mut address_list = client.address_list().clone();
    assert!(address_list.add(address.clone()));

    let response = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect("request should succeed on the added address");

    assert_eq!(response.address, address);
    assert_eq!(
        *request.hit_uris.lock().unwrap(),
        vec![address.uri().clone()]
    );
}
