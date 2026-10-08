//! A request every node refuses is the caller's mistake, not a node's: the client must
//! neither ban the node that refused it nor send it on to the next one. A client that did
//! both banned every node in a few calls with an empty id list.

mod common;

use common::ScriptedRequest;
use dapi_grpc::platform::v0::get_data_contracts_request::{self, GetDataContractsRequestV0};
use dapi_grpc::platform::v0::get_identity_keys_request::{self, GetIdentityKeysRequestV0};
use dapi_grpc::platform::v0::{
    key_request_type, GetDataContractsRequest, GetIdentityKeysRequest, KeyRequestType, SpecificKeys,
};
use dapi_grpc::tonic::{Code, Status};
use rs_dapi_client::transport::{TransportError, TransportRequest};
use rs_dapi_client::{
    Address, AddressList, CanRetry, DapiClient, DapiClientError, DapiRequestExecutor,
    RequestSettings,
};

fn two_nodes() -> AddressList {
    "http://127.0.0.1:10011,http://127.0.0.1:10012"
        .parse()
        .expect("valid address list")
}

fn banning() -> RequestSettings {
    RequestSettings {
        ban_failed_address: Some(true),
        ..RequestSettings::default()
    }
}

/// Asserts the status reached the caller unchanged, from one node, and banned none.
async fn assert_refused_by_one_node_without_a_ban(status: Status) {
    let code = status.code();
    let message = status.message().to_string();
    let reply = message.clone();
    let request = ScriptedRequest::new(move |_uri| {
        Err(TransportError::Grpc(Status::new(code, reply.clone())))
    });
    let client = DapiClient::new(two_nodes(), RequestSettings::default());

    let error = client
        .execute(request.clone(), banning())
        .await
        .expect_err("the node refused the request");

    assert_eq!(
        request.hit_uris.lock().unwrap().len(),
        1,
        "{message}: the request must not be sent to another node"
    );
    assert!(!error.can_retry(), "{message}: the refusal is final");
    match error.inner {
        DapiClientError::Transport(TransportError::Grpc(returned)) => {
            assert_eq!(returned.code(), code);
            assert_eq!(returned.message(), message);
        }
        other => panic!("{message}: expected the node's status, got {other:?}"),
    }
    let refusing_node = error.address.expect("the refusing node is known");
    assert!(
        !client.address_list().is_banned(&refusing_node),
        "{message}: the refusing node must not be banned"
    );
}

#[tokio::test]
async fn should_neither_ban_nor_retry_a_node_that_answers_invalid_argument() {
    assert_refused_by_one_node_without_a_ban(Status::invalid_argument(
        "ids must contain at least one identifier",
    ))
    .await;
}

/// Older nodes answer some refusals with UNKNOWN or INTERNAL; these are the messages they
/// send for an empty id list or a limit of 0.
#[tokio::test]
async fn should_neither_ban_nor_retry_an_older_node_that_refuses_the_request() {
    for status in [
        Status::internal(
            "query: storage: query: query invalid limit error: limit greater than max limit 100",
        ),
        Status::unknown(
            "drive error: query: no query items error: We did not ask for the votes of any \
             validators",
        ),
        Status::internal(
            "query: storage: grovedb: invalid query: proved path queries can not be for limit 0",
        ),
        Status::unknown(
            "drive error: grovedb: invalid query: proved path queries can not be for limit 0",
        ),
    ] {
        assert_refused_by_one_node_without_a_ban(status).await;
    }
}

/// Only a refusal of the request is final: a fault inside the node still bans it and moves
/// the request to the next node.
#[tokio::test]
async fn should_still_ban_and_retry_a_node_that_fails_on_its_own() {
    let request = ScriptedRequest::new(|_uri| {
        Err(TransportError::Grpc(Status::internal(
            "query: storage: grovedb: corrupted data",
        )))
    });
    let client = DapiClient::new(two_nodes(), RequestSettings::default());

    let error = client
        .execute(request.clone(), banning())
        .await
        .expect_err("every node failed");

    let hit = request.hit_uris.lock().unwrap().clone();
    assert_eq!(hit.len(), 2, "the request moves to the next node");
    for uri in hit {
        let address = Address::try_from(uri).expect("valid address");
        assert!(client.address_list().is_banned(&address));
    }
    assert!(matches!(
        error.inner,
        DapiClientError::NoAvailableAddressesToRetry(_)
    ));
}

/// A request that names nothing is refused without a node: no node is asked, so none can
/// answer it with an error that gets it banned.
#[tokio::test]
async fn should_refuse_a_request_that_names_nothing_without_sending_it() {
    let client = DapiClient::new(two_nodes(), RequestSettings::default());
    let request = GetDataContractsRequest {
        version: Some(get_data_contracts_request::Version::V0(
            GetDataContractsRequestV0 {
                ids: vec![],
                prove: true,
            },
        )),
    };

    let error = client
        .execute(request, banning())
        .await
        .expect_err("an empty id list is refused");

    assert_eq!(error.retries, 0);
    assert!(error.address.is_none(), "no node was asked");
    assert!(!error.can_retry());
    match error.inner {
        DapiClientError::Transport(TransportError::Grpc(status)) => {
            assert_eq!(status.code(), Code::InvalidArgument);
            assert_eq!(status.message(), "ids must contain at least one identifier");
        }
        other => panic!("expected an INVALID_ARGUMENT status, got {other:?}"),
    }
    assert!(client
        .address_list()
        .ban_info()
        .iter()
        .all(|info| !info.banned));
}

#[test]
fn should_name_nothing_only_when_the_selection_is_empty() {
    let keys = |key_ids: Vec<u32>| GetIdentityKeysRequest {
        version: Some(get_identity_keys_request::Version::V0(
            GetIdentityKeysRequestV0 {
                identity_id: vec![1; 32],
                request_type: Some(KeyRequestType {
                    request: Some(key_request_type::Request::SpecificKeys(SpecificKeys {
                        key_ids,
                    })),
                }),
                limit: None,
                offset: None,
                prove: true,
            },
        )),
    };
    assert_eq!(
        keys(vec![]).names_nothing(),
        Some("key_ids must name at least one key")
    );
    assert_eq!(keys(vec![0]).names_nothing(), None);

    let contracts = |ids: Vec<Vec<u8>>| GetDataContractsRequest {
        version: Some(get_data_contracts_request::Version::V0(
            GetDataContractsRequestV0 { ids, prove: true },
        )),
    };
    assert_eq!(contracts(vec![vec![1; 32]]).names_nothing(), None);
    assert_eq!(
        GetDataContractsRequest { version: None }.names_nothing(),
        None
    );
}
