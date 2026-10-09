use super::*;
use dapi_grpc::platform::v0::{
    get_contested_resource_identity_votes_request as identity_request,
    get_data_contracts_request as contracts_request,
    get_vote_polls_by_end_date_request as polls_request, GetDataContractsRequest,
};
use drive_proof_verifier::types::{
    DataContracts, ResourceVotesByIdentity, VotePollsGroupedByTimestamp,
};
use drive_proof_verifier::Error as ProofError;

fn polls_response(proof: Proof, metadata: ResponseMetadata) -> GetVotePollsByEndDateResponse {
    GetVotePollsByEndDateResponse {
        version: Some(polls_response::Version::V0(
            polls_response::GetVotePollsByEndDateResponseV0 {
                result: Some(
                    polls_response::get_vote_polls_by_end_date_response_v0::Result::Proof(proof),
                ),
                metadata: Some(metadata),
            },
        )),
    }
}

fn votes_response(
    proof: Proof,
    metadata: ResponseMetadata,
) -> GetContestedResourceIdentityVotesResponse {
    GetContestedResourceIdentityVotesResponse {
        version: Some(identity_response::Version::V0(
            identity_response::GetContestedResourceIdentityVotesResponseV0 {
                result: Some(identity_response::get_contested_resource_identity_votes_response_v0::Result::Proof(proof)),
                metadata: Some(metadata),
            },
        )),
    }
}

#[tokio::test]
async fn should_preserve_exact_voting_pages_for_short_full_descending_and_explicit_limits() {
    let version = PlatformVersion::latest();
    for count in [
        3,
        DEFAULT_QUERY_LIMIT as usize,
        DEFAULT_QUERY_LIMIT as usize + 1,
    ] {
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = fixture::populate(&drive, count, version);
        let root = drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .unwrap()
            .expect("root");
        for ascending in [true, false] {
            for requested_limit in [None, Some(2)] {
                let limit = requested_limit.unwrap_or(DEFAULT_QUERY_LIMIT);
                let query = VotePollsByEndDateDriveQuery {
                    order_ascending: ascending,
                    ..polls_query(requested_limit)
                };
                let explicit = VotePollsByEndDateDriveQuery {
                    limit: Some(limit),
                    ..query.clone()
                };
                let (bytes, _) = explicit
                    .clone()
                    .execute_with_proof(&drive, None, None, version)
                    .expect("poll proof");
                let (proof, metadata) = authenticated(bytes, root);
                let wire: GetVotePollsByEndDateRequest =
                    explicit.query(&Sdk::new_mock().query_settings()).unwrap();
                let sdk = sdk_for(&[wire], polls_response(proof, metadata), contract.clone());
                let mut indices: Vec<_> = (0..count).collect();
                if !ascending {
                    indices.reverse();
                }
                indices.truncate(limit as usize);
                let expected: Vec<_> = indices
                    .into_iter()
                    .map(|i| {
                        (
                            fixture::END_TIME + i as u64,
                            vec![fixture::poll(&contract, i)],
                        )
                    })
                    .collect();
                assert_eq!(
                    VotePoll::fetch_many(&sdk, query)
                        .await
                        .expect("poll page")
                        .0,
                    expected
                );

                let query = ContestedResourceVotesGivenByIdentityQuery {
                    order_ascending: ascending,
                    ..identity_query(requested_limit)
                };
                let explicit = ContestedResourceVotesGivenByIdentityQuery {
                    limit: Some(limit),
                    ..query.clone()
                };
                let (bytes, _) = explicit
                    .clone()
                    .execute_with_proof(&drive, None, None, version)
                    .expect("identity proof");
                let (proof, metadata) = authenticated(bytes, root);
                let wire: GetContestedResourceIdentityVotesRequest =
                    explicit.query(&Sdk::new_mock().query_settings()).unwrap();
                let sdk = sdk_for(&[wire], votes_response(proof, metadata), contract.clone());
                let mut expected: Vec<_> = (0..count)
                    .map(|i| {
                        let poll = fixture::poll(&contract, i);
                        (
                            poll.unique_id().unwrap(),
                            Some(ResourceVote::V0(ResourceVoteV0 {
                                vote_poll: poll,
                                resource_vote_choice: ResourceVoteChoice::Abstain,
                            })),
                        )
                    })
                    .collect();
                expected.sort_by_key(|(id, _)| *id);
                if !ascending {
                    expected.reverse();
                }
                expected.truncate(limit as usize);
                assert_eq!(
                    ResourceVote::fetch_many(&sdk, query)
                        .await
                        .expect("identity page")
                        .into_iter()
                        .collect::<Vec<_>>(),
                    expected
                );
            }
        }
    }
}

#[test]
fn should_preserve_explicit_unproved_unversioned_and_unrelated_query_bytes() {
    for prove in [false, true] {
        for limit in [None, Some(0), Some(7)] {
            let polls = GetVotePollsByEndDateRequest {
                version: Some(polls_request::Version::V0(
                    polls_request::GetVotePollsByEndDateRequestV0 {
                        prove,
                        limit,
                        ascending: false,
                        start_time_info: Some(
                            polls_request::get_vote_polls_by_end_date_request_v0::StartAtTimeInfo {
                                start_time_ms: fixture::END_TIME,
                                start_time_included: false,
                            },
                        ),
                        end_time_info: Some(
                            polls_request::get_vote_polls_by_end_date_request_v0::EndAtTimeInfo {
                                end_time_ms: fixture::END_TIME + 20,
                                end_time_included: true,
                            },
                        ),
                        ..Default::default()
                    },
                )),
            };
            let votes = GetContestedResourceIdentityVotesRequest { version: Some(identity_request::Version::V0(identity_request::GetContestedResourceIdentityVotesRequestV0 {
                identity_id: fixture::VOTER.to_vec(), prove, limit, order_ascending: false,
                start_at_vote_poll_id_info: Some(identity_request::get_contested_resource_identity_votes_request_v0::StartAtVotePollIdInfo { start_at_poll_identifier: vec![9; 32], start_poll_identifier_included: false }),
                ..Default::default()
            })) };
            let mut expected_polls = polls;
            let mut expected_votes = votes.clone();
            if prove && limit.is_none() {
                let Some(polls_request::Version::V0(v0)) = expected_polls.version.as_mut() else {
                    unreachable!()
                };
                v0.limit = Some(DEFAULT_QUERY_LIMIT as u32);
                let Some(identity_request::Version::V0(v0)) = expected_votes.version.as_mut()
                else {
                    unreachable!()
                };
                v0.limit = Some(DEFAULT_QUERY_LIMIT as u32);
            }
            assert_eq!(
                <VotePoll as FetchMany<u64, VotePollsGroupedByTimestamp>>::prepare_query(polls)
                    .mock_serialize()
                    .unwrap(),
                expected_polls.mock_serialize().unwrap()
            );
            assert_eq!(
                <ResourceVote as FetchMany<Identifier, ResourceVotesByIdentity>>::prepare_query(
                    votes
                )
                .mock_serialize()
                .unwrap(),
                expected_votes.mock_serialize().unwrap()
            );
        }
    }
    let polls = GetVotePollsByEndDateRequest { version: None };
    assert_eq!(
        <VotePoll as FetchMany<u64, VotePollsGroupedByTimestamp>>::prepare_query(polls),
        polls
    );
    let votes = GetContestedResourceIdentityVotesRequest { version: None };
    assert_eq!(
        <ResourceVote as FetchMany<Identifier, ResourceVotesByIdentity>>::prepare_query(
            votes.clone()
        ),
        votes
    );
    let contracts = GetDataContractsRequest {
        version: Some(contracts_request::Version::V0(
            contracts_request::GetDataContractsRequestV0 {
                ids: vec![vec![8; 32]],
                prove: true,
            },
        )),
    };
    assert_eq!(
        <DataContract as FetchMany<Identifier, DataContracts>>::prepare_query(contracts.clone())
            .mock_serialize()
            .unwrap(),
        contracts.mock_serialize().unwrap()
    );
}

#[tokio::test]
async fn should_bind_a_raw_proved_request_when_sdk_default_proofs_are_disabled() {
    let version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = fixture::populate(&drive, DEFAULT_QUERY_LIMIT as usize + 1, version);
    let root = drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .unwrap()
        .expect("root");
    let (bytes, _) = polls_query(Some(DEFAULT_QUERY_LIMIT))
        .execute_with_proof(&drive, None, None, version)
        .unwrap();
    let (proof, metadata) = authenticated(bytes, root);
    let settings_sdk = Sdk::new_mock();
    let omitted: GetVotePollsByEndDateRequest = polls_query(None)
        .query(&settings_sdk.query_settings())
        .unwrap();
    let explicit: GetVotePollsByEndDateRequest = polls_query(Some(DEFAULT_QUERY_LIMIT))
        .query(&settings_sdk.query_settings())
        .unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    let response = Ok(ExecutionResponse {
        inner: polls_response(proof, metadata),
        retries: 0,
        address: "http://127.0.0.1:9000".parse().unwrap(),
    });
    let dump = DumpData::new(&explicit, &response);
    dump.save(&dir.path().join(dump.filename().unwrap()))
        .unwrap();
    let sdk = SdkBuilder::new_mock()
        .with_proofs(false)
        .with_version(version)
        .with_context_provider(Provider(Arc::new(contract.clone())))
        .with_settings(RequestSettings {
            retries: Some(0),
            ..Default::default()
        })
        .with_dump_dir(dir.path())
        .build()
        .unwrap();
    let expected: Vec<_> = (0..DEFAULT_QUERY_LIMIT as usize)
        .map(|i| {
            (
                fixture::END_TIME + i as u64,
                vec![fixture::poll(&contract, i)],
            )
        })
        .collect();
    assert_eq!(
        VotePoll::fetch_many(&sdk, omitted)
            .await
            .expect("raw prove flag governs the query")
            .0,
        expected
    );
}

#[tokio::test]
async fn should_refuse_tampered_proof_wrong_signed_root_signature_and_changed_limit() {
    let version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = fixture::populate(&drive, DEFAULT_QUERY_LIMIT as usize + 1, version);
    let root = drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .unwrap()
        .expect("root");
    let (bytes, _) = polls_query(Some(DEFAULT_QUERY_LIMIT))
        .execute_with_proof(&drive, None, None, version)
        .unwrap();
    let (valid, metadata) = authenticated(bytes.clone(), root);
    for corruption in 0..4 {
        let mut proof = valid.clone();
        let mut query = polls_query(None);
        match corruption {
            0 => proof.grovedb_proof.truncate(proof.grovedb_proof.len() / 2),
            1 => proof = authenticated(bytes.clone(), [0; 32]).0,
            2 => {
                proof.signature = signing_key()
                    .sign(SignatureSchemes::Basic, &[0; 32])
                    .unwrap()
                    .as_raw_value()
                    .to_compressed()
                    .to_vec()
            }
            3 => query.limit = Some(DEFAULT_QUERY_LIMIT + 1),
            _ => unreachable!(),
        }
        let explicit = VotePollsByEndDateDriveQuery {
            limit: Some(query.limit.unwrap_or(DEFAULT_QUERY_LIMIT)),
            ..query.clone()
        };
        let wire: GetVotePollsByEndDateRequest =
            explicit.query(&Sdk::new_mock().query_settings()).unwrap();
        let sdk = sdk_for(
            &[wire],
            polls_response(proof, metadata.clone()),
            contract.clone(),
        );
        let error = VotePoll::fetch_many(&sdk, query)
            .await
            .expect_err("untrusted page must be refused");
        match (corruption, error) {
            (0 | 3, dash_sdk::Error::Proof(ProofError::GroveDBError { .. })) => (),
            (
                1 | 2,
                dash_sdk::Error::Proof(
                    ProofError::InvalidSignature { .. }
                    | ProofError::SignatureVerificationError { .. },
                ),
            ) => (),
            (_, other) => panic!("unexpected rejection: {other}"),
        }
    }
}
