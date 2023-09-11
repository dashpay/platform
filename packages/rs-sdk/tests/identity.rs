use dpp::{identity::accessors::IdentityGettersV0, prelude::Identifier};
use rs_sdk::platform::Fetch;

include!("common.rs");

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_identity_read() {
    pub const IDENTITY_ID_BYTES: [u8; 32] = [
        65, 63, 57, 243, 204, 9, 106, 71, 187, 2, 94, 221, 190, 127, 141, 114, 137, 209, 243, 50,
        60, 215, 90, 101, 229, 15, 115, 5, 44, 117, 182, 217,
    ];
    let id = Identifier::from_bytes(&IDENTITY_ID_BYTES).expect("parse identity id");

    let api = setup_api();

    let identity = dpp::prelude::Identity::fetch(&api, id)
        .await
        .unwrap()
        .expect("identity should exist");

    assert_eq!(identity.id(), &id);
}
