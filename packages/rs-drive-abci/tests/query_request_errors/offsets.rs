use super::*;
use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::tests::json_document::json_document_to_json_value;

fn key_request(
    kind: KeyRequestKind,
    offset: Option<u32>,
    prove: bool,
) -> wire::GetIdentityKeysRequest {
    GetIdentityKeysRequestV0 {
        identity_id: vec![8; 32],
        request_type: Some(wire::KeyRequestType {
            request: Some(kind),
        }),
        limit: Some(2),
        offset,
        prove,
    }
    .into()
}

pub(super) fn search_kind() -> KeyRequestKind {
    KeyRequestKind::SearchKey(wire::SearchKey {
        purpose_map: [(
            0,
            wire::SecurityLevelMap {
                security_level_map: [(0, 0), (1, 0)].into_iter().collect(),
            },
        )]
        .into_iter()
        .collect(),
    })
}

pub(super) fn history_contract(fixture: &QueryFixture) -> dpp::data_contract::DataContract {
    let mut value = json_document_to_json_value(
        "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
    )
    .expect("profile-history JSON");
    value["documentSchemas"]
        .as_object_mut()
        .expect("schemas")
        .retain(|name, _| name == "profile");
    value["documentSchemas"]["profile"]["canBeDeleted"] = serde_json::Value::Bool(false);
    for (index, schema) in value["documentSchemas"]["profile"]["indices"]
        .as_array_mut()
        .expect("profile indexes")
        .iter_mut()
        .enumerate()
    {
        schema["name"] = serde_json::Value::String(format!("historyIndex{index}"));
    }
    let contract = dpp::data_contract::DataContract::from_json(value, true, fixture.version)
        .expect("valid profile-history contract");
    fixture
        .platform
        .drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            None,
            fixture.version,
        )
        .expect("store profile-history contract");
    contract
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_refuse_unsupported_positive_offsets_over_tonic() {
    let mut failures = Vec::new();
    for version in [
        PlatformVersion::get(12).unwrap(),
        PlatformVersion::get(13).unwrap(),
        PlatformVersion::latest(),
    ] {
        let fixture = QueryFixture::new_with_version(version);
        let (contract_id, _) = fixture.store_contract();
        let profile = history_contract(&fixture);
        let (mut client, server) = fixture.client();
        for offset in [1, u16::MAX as u32] {
            for (name, kind, prove) in [
                (
                    "proved AllKeys",
                    KeyRequestKind::AllKeys(wire::AllKeys {}),
                    true,
                ),
                ("proved SearchKey", search_kind(), true),
                ("raw SearchKey", search_kind(), false),
            ] {
                let status = client
                    .get_identity_keys(key_request(kind, Some(offset), prove))
                    .await
                    .expect_err("unsupported offset must be refused");
                if status.code() != Code::InvalidArgument || !status.message().contains(if prove { "requesting proof with offset error: proof requests do not support positive offsets" } else { "search key requests without a proof do not support positive offsets" }) {
                    failures.push(format!("PV{} {name} offset={offset}: {status}", version.protocol_version));
                }
            }
            let status = client
                .get_data_contract_history(wire::GetDataContractHistoryRequest::from(
                    GetDataContractHistoryRequestV0 {
                        id: contract_id.to_vec(),
                        limit: Some(2),
                        offset: Some(offset),
                        start_at_ms: 0,
                        prove: true,
                    },
                ))
                .await
                .expect_err("contract-history proof cannot page by offset");
            if status.code() != Code::InvalidArgument || !status.message().contains("requesting proof with offset error: proof requests do not support positive offsets") { failures.push(format!("PV{} contract history offset={offset}: {status}", version.protocol_version)); }
            let status = client
                .get_document_history(wire::GetDocumentHistoryRequest::from(
                    wire::get_document_history_request::GetDocumentHistoryRequestV0 {
                        data_contract_id: profile.id().to_vec(),
                        document_type_name: "profile".to_owned(),
                        document_id: vec![9; 32],
                        limit: Some(2),
                        offset: Some(offset),
                        start_at_ms: 0,
                        prove: true,
                    },
                ))
                .await
                .expect_err("document-history proof cannot page by offset");
            if status.code() != Code::InvalidArgument || !status.message().contains("requesting proof with offset error: proof requests do not support positive offsets") { failures.push(format!("PV{} document history offset={offset}: {status}", version.protocol_version)); }
            let status = client
                .get_contested_resource_identity_votes(
                    wire::GetContestedResourceIdentityVotesRequest::from(
                        GetContestedResourceIdentityVotesRequestV0 {
                            identity_id: vec![8; 32],
                            limit: Some(2),
                            offset: Some(offset),
                            order_ascending: true,
                            start_at_vote_poll_id_info: None,
                            prove: true,
                        },
                    ),
                )
                .await
                .expect_err("identity-votes proof cannot page by offset");
            if status.code() != Code::InvalidArgument || !status.message().contains("requesting proof with offset error: proof requests do not support positive offsets") { failures.push(format!("PV{} identity votes offset={offset}: {status}", version.protocol_version)); }
        }
        server.abort();
    }
    assert!(
        failures.is_empty(),
        "unsupported caller offsets reached nodes as internal faults:\n{}",
        failures.join("\n")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_serve_explicit_zero_search_offsets_like_omitted_offsets() {
    let mut failures = Vec::new();
    for version in [
        PlatformVersion::get(12).unwrap(),
        PlatformVersion::get(13).unwrap(),
        PlatformVersion::latest(),
    ] {
        let fixture = QueryFixture::new_with_version(version);
        let identity = Identity::random_identity(5, Some(91823), version).expect("seeded identity");
        let id = identity.id().to_vec();
        fixture
            .platform
            .drive
            .add_new_identity(identity, false, &BlockInfo::default(), true, None, version)
            .expect("store identity");
        let (mut client, server) = fixture.client();
        let mut omitted = None;
        for offset in [None, Some(0)] {
            let mut request = key_request(search_kind(), offset, false);
            let Some(wire::get_identity_keys_request::Version::V0(v0)) = &mut request.version
            else {
                panic!("V0 request")
            };
            v0.identity_id = id.clone();
            match client.get_identity_keys(request).await {
                Ok(response) => {
                    let Some(get_identity_keys_response::Version::V0(v0)) =
                        response.into_inner().version
                    else {
                        panic!("V0 response")
                    };
                    let Some(get_identity_keys_response_v0::Result::Keys(keys)) = v0.result else {
                        panic!("raw key response")
                    };
                    assert!(
                        keys.keys_bytes.is_empty(),
                        "the current-key search preserves its existing empty selection"
                    );
                    if offset.is_none() {
                        omitted = Some(keys.keys_bytes);
                    } else {
                        assert_eq!(Some(keys.keys_bytes), omitted);
                    }
                }
                Err(status) => failures.push(format!(
                    "PV{} raw SearchKey offset={offset:?}: {status}",
                    version.protocol_version
                )),
            }
        }
        server.abort();
    }
    assert!(
        failures.is_empty(),
        "explicit zero must preserve the omitted-offset selection:\n{}",
        failures.join("\n")
    );
}

macro_rules! node_query {
    ($name:ident, $request:ty, $response:ty, $method:ident) => {
        #[derive(Clone, Debug)]
        pub(super) struct $name {
            pub(super) fixture: Arc<QueryFixture>,
            pub(super) request: $request,
            pub(super) attempts: Arc<Mutex<Vec<Uri>>>,
        }
        impl Mockable for $name {}
        impl TransportRequest for $name {
            type Client = QueryNodeClient;
            type Response = $response;
            const SETTINGS_OVERRIDES: RequestSettings = RequestSettings::default();
            fn method_name(&self) -> &'static str {
                stringify!($method)
            }
            fn execute_transport<'c>(
                self,
                client: &'c mut Self::Client,
                _settings: &AppliedRequestSettings,
            ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
                self.attempts
                    .lock()
                    .expect("attempt log")
                    .push(client.uri.clone());
                let (mut grpc, server) = self.fixture.client_at(client.uri.clone());
                Box::pin(async move {
                    let result = grpc
                        .$method(self.request)
                        .await
                        .map(|response| response.into_inner())
                        .map_err(TransportError::Grpc);
                    server.abort();
                    result
                })
            }
        }
    };
}

node_query!(
    KeysQuery,
    wire::GetIdentityKeysRequest,
    wire::GetIdentityKeysResponse,
    get_identity_keys
);
node_query!(
    ContractHistoryQuery,
    wire::GetDataContractHistoryRequest,
    wire::GetDataContractHistoryResponse,
    get_data_contract_history
);
node_query!(
    DocumentHistoryQuery,
    wire::GetDocumentHistoryRequest,
    wire::GetDocumentHistoryResponse,
    get_document_history
);
node_query!(
    IdentityVotesQuery,
    wire::GetContestedResourceIdentityVotesRequest,
    wire::GetContestedResourceIdentityVotesResponse,
    get_contested_resource_identity_votes
);
node_query!(
    DocumentsQuery,
    wire::GetDocumentsRequest,
    wire::GetDocumentsResponse,
    get_documents
);

pub(super) async fn assert_no_node_bans<R: TransportRequest<Client = QueryNodeClient>>(
    query: R,
    attempts: &Arc<Mutex<Vec<Uri>>>,
) {
    let client = DapiClient::new(
        query_nodes(13),
        RequestSettings {
            retries: Some(5),
            ..RequestSettings::default()
        },
    );
    let mut failures = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), async {
        for call in 1..=3 {
            let error = client
                .execute(query.clone(), RequestSettings::default())
                .await
                .expect_err("invalid caller offset");
            let bans = client.address_list().ban_info();
            let attempts = attempts.lock().expect("attempt log").len();
            if grpc_code(&error.inner) != Some(Code::InvalidArgument)
                || error.retries != 0
                || error.address.is_none()
                || attempts != call
                || bans.iter().any(|node| node.banned || node.ban_count != 0)
            {
                failures.push(format!(
                    "call={call}: {error:?}; attempts={attempts}; bans={bans:?}"
                ));
            }
        }
    })
    .await
    .expect("bounded three-call scenario cannot wait for bans to expire");
    assert!(
        failures.is_empty(),
        "bad caller offset consumed healthy nodes:\n{}",
        failures.join("\n")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_keep_all_thirteen_nodes_available_after_three_bad_offset_requests() {
    let fixture = Arc::new(QueryFixture::new());
    let (contract_id, _) = fixture.store_contract();
    let profile = history_contract(&fixture);
    for (kind, prove) in [
        (KeyRequestKind::AllKeys(wire::AllKeys {}), true),
        (search_kind(), true),
        (search_kind(), false),
    ] {
        let attempts = Arc::default();
        assert_no_node_bans(
            KeysQuery {
                fixture: Arc::clone(&fixture),
                request: key_request(kind, Some(1), prove),
                attempts: Arc::clone(&attempts),
            },
            &attempts,
        )
        .await;
    }
    let attempts = Arc::default();
    assert_no_node_bans(
        ContractHistoryQuery {
            fixture: Arc::clone(&fixture),
            request: GetDataContractHistoryRequestV0 {
                id: contract_id.to_vec(),
                limit: Some(2),
                offset: Some(1),
                start_at_ms: 0,
                prove: true,
            }
            .into(),
            attempts: Arc::clone(&attempts),
        },
        &attempts,
    )
    .await;
    let attempts = Arc::default();
    assert_no_node_bans(
        DocumentHistoryQuery {
            fixture: Arc::clone(&fixture),
            request: wire::get_document_history_request::GetDocumentHistoryRequestV0 {
                data_contract_id: profile.id().to_vec(),
                document_type_name: "profile".to_owned(),
                document_id: vec![9; 32],
                limit: Some(2),
                offset: Some(1),
                start_at_ms: 0,
                prove: true,
            }
            .into(),
            attempts: Arc::clone(&attempts),
        },
        &attempts,
    )
    .await;
    let attempts = Arc::default();
    assert_no_node_bans(
        IdentityVotesQuery {
            fixture,
            request: GetContestedResourceIdentityVotesRequestV0 {
                identity_id: vec![8; 32],
                limit: Some(2),
                offset: Some(1),
                order_ascending: true,
                start_at_vote_poll_id_info: None,
                prove: true,
            }
            .into(),
            attempts: Arc::clone(&attempts),
        },
        &attempts,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_validate_proof_offsets_before_missing_history_lookups() {
    for version in [
        PlatformVersion::get(12).unwrap(),
        PlatformVersion::get(13).unwrap(),
        PlatformVersion::latest(),
    ] {
        let fixture = QueryFixture::new_with_version(version);
        let profile = history_contract(&fixture);
        let (mut client, server) = fixture.client();
        for (id, name) in [
            (vec![0xEE; 32], "profile"),
            (profile.id().to_vec(), "absentType"),
        ] {
            for prove in [false, true] {
                let request = wire::get_document_history_request::GetDocumentHistoryRequestV0 {
                    data_contract_id: id.clone(),
                    document_type_name: name.to_owned(),
                    document_id: vec![9; 32],
                    limit: Some(2),
                    offset: Some(1),
                    start_at_ms: 0,
                    prove,
                };
                let status = client
                    .get_document_history(wire::GetDocumentHistoryRequest::from(request))
                    .await
                    .expect_err("missing history or unsupported proof offset");
                assert_eq!(
                    status.code(),
                    if prove {
                        Code::InvalidArgument
                    } else {
                        Code::NotFound
                    }
                );
                if prove {
                    assert!(status.message().contains("requesting proof with offset error: proof requests do not support positive offsets"),"{status}");
                }
            }
        }
        let mut request = key_request(search_kind(), Some(1), true);
        let Some(wire::get_identity_keys_request::Version::V0(v0)) = &mut request.version else {
            panic!("V0")
        };
        v0.identity_id = vec![8; 31];
        let status = client
            .get_identity_keys(request)
            .await
            .expect_err("malformed identifier precedes offset policy");
        assert_eq!(status.code(), Code::InvalidArgument);
        assert!(status.message().contains("32 bytes long"));
        let mut request = key_request(search_kind(), Some(1), false);
        let Some(wire::get_identity_keys_request::Version::V0(v0)) = &mut request.version else {
            panic!("V0")
        };
        v0.limit = None;
        let status = client
            .get_identity_keys(request)
            .await
            .expect_err("omitted search cap precedes offset policy");
        assert_eq!(status.code(), Code::InvalidArgument);
        assert!(status.message().contains("must set a limit"));
        for (limit, offset, kind, expected) in [
            (Some(0), Some(1), search_kind(), "limit 0 out of bounds"),
            (
                Some(u32::MAX),
                Some(1),
                search_kind(),
                "limit out of bounds",
            ),
            (
                Some(2),
                Some(u16::MAX as u32 + 1),
                search_kind(),
                "offset out of bounds",
            ),
            (
                Some(2),
                Some(1),
                KeyRequestKind::SearchKey(wire::SearchKey {
                    purpose_map: [(
                        0,
                        wire::SecurityLevelMap {
                            security_level_map: [(0, 99)].into_iter().collect(),
                        },
                    )]
                    .into_iter()
                    .collect(),
                }),
                "unknown key kind request type 99",
            ),
        ] {
            let mut request = key_request(kind, offset, true);
            let Some(wire::get_identity_keys_request::Version::V0(v0)) = &mut request.version
            else {
                panic!("V0")
            };
            v0.limit = limit;
            let status = client
                .get_identity_keys(request)
                .await
                .expect_err("structural validation precedes proof offset policy");
            assert_eq!(status.code(), Code::InvalidArgument);
            assert!(status.message().contains(expected), "{status}");
        }
        server.abort();
    }
}
