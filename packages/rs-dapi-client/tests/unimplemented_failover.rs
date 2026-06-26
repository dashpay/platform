//! Failover behavior for gRPC `Unimplemented` during a mixed-version network
//! rollout: a node running an older build answers UNIMPLEMENTED for a method
//! that upgraded nodes already serve. The executor must ban that node and
//! retry the request on another address. If every node is unimplemented, the
//! error must still surface instead of retrying forever.

mod common;

use std::sync::{Arc, Mutex};

use common::{FakeResponse, ScriptedRequest};
use dapi_grpc::tonic::{Code, Status};
use rs_dapi_client::transport::TransportError;
use rs_dapi_client::{
    Address, AddressList, CanRetry, DapiClient, DapiClientError, DapiRequestExecutor,
    RequestSettings, Uri,
};

#[tokio::test]
async fn unimplemented_node_is_banned_and_request_retried_on_another() {
    // The closure captures its own state: exactly one UNIMPLEMENTED response,
    // then success.  `error_uris` records which node answered UNIMPLEMENTED.
    let error_uris: Arc<Mutex<Vec<Uri>>> = Default::default();
    let error_uris_c = error_uris.clone();

    let request = ScriptedRequest::new(move |uri| {
        let mut uris = error_uris_c.lock().unwrap();
        if uris.is_empty() {
            // First call: simulate an old node that doesn't have the method yet.
            uris.push(uri);
            Err(TransportError::Grpc(Status::unimplemented(
                "Operation is not implemented or not supported",
            )))
        } else {
            // Subsequent calls: upgraded node responds successfully.
            Ok(FakeResponse)
        }
    });

    let address_list: AddressList = "http://127.0.0.1:10001,http://127.0.0.1:10002"
        .parse()
        .expect("valid address list");
    let client = DapiClient::new(address_list, RequestSettings::default());

    let response = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect("request should succeed on the upgraded node");

    let old_node_uri = {
        let uris = error_uris.lock().unwrap();
        assert_eq!(
            uris.len(),
            1,
            "exactly one node should have answered UNIMPLEMENTED"
        );
        uris[0].clone()
    };

    assert_eq!(response.retries, 1);
    assert_ne!(
        response.address.uri(),
        &old_node_uri,
        "retry must go to a different node"
    );

    let old_node = Address::try_from(old_node_uri).expect("valid address");
    assert!(
        client.address_list().is_banned(&old_node),
        "the node that answered UNIMPLEMENTED must be banned"
    );
}

#[tokio::test]
async fn unimplemented_on_all_nodes_still_surfaces_error() {
    // No node implements the method: every attempt answers UNIMPLEMENTED.
    // `hit_uris` counts total attempts (all of which are errors here).
    let request = ScriptedRequest::new(|_uri| {
        Err(TransportError::Grpc(Status::unimplemented(
            "Operation is not implemented or not supported",
        )))
    });
    let address_list: AddressList = "http://127.0.0.1:10003,http://127.0.0.1:10004"
        .parse()
        .expect("valid address list");
    let client = DapiClient::new(address_list, RequestSettings::default());

    let error = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect_err("request must fail when no node implements the method");

    // Both nodes were tried once each before the address list was exhausted.
    assert_eq!(request.hit_uris.lock().unwrap().len(), 2);
    assert!(
        !error.can_retry(),
        "exhausted-addresses error must not be retried by callers"
    );

    match error.inner {
        DapiClientError::NoAvailableAddressesToRetry(transport_error) => match *transport_error {
            TransportError::Grpc(status) => assert_eq!(status.code(), Code::Unimplemented),
        },
        other => panic!("expected NoAvailableAddressesToRetry, got: {other:?}"),
    }
}

#[tokio::test]
async fn unimplemented_surfaces_retryable_when_pool_exceeds_retry_budget() {
    // The non-retryable `NoAvailableAddressesToRetry` collapse (above) only holds
    // when the live address list is exhausted before the retry budget, i.e.
    // `live_addresses <= settings.retries`. When the pool is LARGER than the retry
    // budget, the per-call retry cap trips first: the executor surfaces the raw
    // (still-retryable) `Unimplemented` after banning `retries + 1` nodes, leaving
    // the rest of the pool live. A caller honoring `CanRetry` re-enters and bans
    // more nodes each round (rs-sdk additionally caps via `total_retries`), so
    // termination is still bounded — this test pins the contract for that branch.
    let settings = RequestSettings {
        retries: Some(1),
        ..Default::default()
    };

    // Three live nodes, retry budget of 1 → at most retries + 1 = 2 attempts, so
    // the retry cap trips before all three addresses are banned.
    let request = ScriptedRequest::new(|_uri| {
        Err(TransportError::Grpc(Status::unimplemented(
            "Operation is not implemented or not supported",
        )))
    });
    let address_list: AddressList =
        "http://127.0.0.1:10005,http://127.0.0.1:10006,http://127.0.0.1:10007"
            .parse()
            .expect("valid address list");
    let client = DapiClient::new(address_list, settings);

    let error = client
        .execute(request.clone(), settings)
        .await
        .expect_err("request must fail when no node implements the method");

    // retries + 1 = 2 nodes were tried (and banned) before the retry cap tripped;
    // the third address is never reached.
    assert_eq!(
        request.hit_uris.lock().unwrap().len(),
        2,
        "retry budget (1) caps attempts at 2 before the 3-node pool is exhausted"
    );

    // Because the retry cap — not address exhaustion — terminated the loop, the
    // raw Unimplemented surfaces and is still retryable (unlike the exhausted-pool
    // case, which collapses to the non-retryable NoAvailableAddressesToRetry).
    assert!(
        error.can_retry(),
        "with more live nodes than retries, the surfaced Unimplemented stays retryable"
    );
    match error.inner {
        DapiClientError::Transport(TransportError::Grpc(status)) => {
            assert_eq!(status.code(), Code::Unimplemented)
        }
        other => panic!("expected raw Transport(Grpc(Unimplemented)), got: {other:?}"),
    }
}
