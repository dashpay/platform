use super::*;
use dpp::dashcore::hashes::sha256;
use drive_proof_verifier::{
    types::{ResourceVotesByIdentity, VotePollsGroupedByTimestamp},
    FromProof,
};
use std::path::Path;

#[tokio::test]
async fn should_verify_recorded_voting_proofs_for_legacy_and_explicit_default_requests() {
    let mut verified_vectors = 0;
    let vectors = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
    for namespace in [
        "vote_polls_by_ts_ok",
        "vote_polls_by_ts_order",
        "test_prefunded_specialized_balance_ok",
    ] {
        let directory = vectors.join(namespace);
        for file in std::fs::read_dir(&directory).unwrap() {
            let file = file.unwrap().path();
            if !file
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("msg_GetVotePollsByEndDateRequest_")
            {
                continue;
            }
            let dump = DumpData::<GetVotePollsByEndDateRequest>::load(&file).unwrap();
            let (request, response) = dump.deserialize();
            let Some(dapi_grpc::platform::v0::get_vote_polls_by_end_date_request::Version::V0(v0)) =
                &request.version
            else {
                panic!("versioned vector")
            };
            if v0.limit.is_some() {
                continue;
            }
            let original_sdk = SdkBuilder::new_mock()
                .with_dump_dir(&directory)
                .build()
                .unwrap();
            let provider = original_sdk.context_provider().unwrap();
            let response = response.unwrap();
            let legacy = VotePollsGroupedByTimestamp::maybe_from_proof_with_metadata(
                request,
                response.inner.clone(),
                original_sdk.network,
                original_sdk.version(),
                &provider,
            )
            .expect("legacy request proof and signature");
            let explicit =
                <VotePoll as FetchMany<u64, VotePollsGroupedByTimestamp>>::prepare_query(request);
            let mut copied = dump.clone();
            copied.serialized_request = explicit.mock_serialize().unwrap();
            assert_eq!(copied.serialized_response, dump.serialized_response);
            let temporary = tempfile::TempDir::new().unwrap();
            let filename = copied.filename().unwrap();
            copied.save(&temporary.path().join(&filename)).unwrap();
            let sdk = SdkBuilder::new_mock()
                .with_context_provider(provider)
                .with_dump_dir(temporary.path())
                .build()
                .unwrap();
            let verified = VotePoll::fetch_many_with_metadata_and_proof(&sdk, request, None)
                .await
                .expect("explicit default proof and signature");
            assert_eq!(verified.0 .0, legacy.0.unwrap_or_default().0);
            assert_eq!((verified.1, verified.2), (legacy.1, legacy.2));
            verified_vectors += 1;
            println!(
                "verified-vector {namespace} {} {filename} response-sha256 {}",
                file.file_name().unwrap().to_str().unwrap(),
                sha256::Hash::hash(&dump.serialized_response)
            );
            let committed =
                DumpData::<GetVotePollsByEndDateRequest>::load(directory.join(&filename))
                    .expect("explicit-default fixture");
            assert_eq!(
                committed.serialized_request.strip_suffix(b"\n").unwrap(),
                copied.serialized_request.as_slice()
            );
            assert_eq!(committed.serialized_response, dump.serialized_response);
        }
    }
    for namespace in [
        "contested_resource_identity_votes_ok",
        "contested_resource_identity_votes_not_found",
    ] {
        let directory = vectors.join(namespace);
        for file in std::fs::read_dir(&directory).unwrap() {
            let file = file.unwrap().path();
            if !file
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("msg_GetContestedResourceIdentityVotesRequest_")
            {
                continue;
            }
            let dump = DumpData::<GetContestedResourceIdentityVotesRequest>::load(&file).unwrap();
            let (request, response) = dump.deserialize();
            let Some(
                dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::Version::V0(
                    v0,
                ),
            ) = &request.version
            else {
                panic!("versioned vector")
            };
            if v0.limit.is_some() {
                continue;
            }
            let original_sdk = SdkBuilder::new_mock()
                .with_dump_dir(&directory)
                .build()
                .unwrap();
            let provider = original_sdk.context_provider().unwrap();
            let response = response.unwrap();
            let legacy = ResourceVotesByIdentity::maybe_from_proof_with_metadata(
                request.clone(),
                response.inner.clone(),
                original_sdk.network,
                original_sdk.version(),
                &provider,
            )
            .expect("legacy request proof and signature");
            let explicit =
                <ResourceVote as FetchMany<Identifier, ResourceVotesByIdentity>>::prepare_query(
                    request.clone(),
                );
            let mut copied = dump.clone();
            copied.serialized_request = explicit.mock_serialize().unwrap();
            assert_eq!(copied.serialized_response, dump.serialized_response);
            let temporary = tempfile::TempDir::new().unwrap();
            let filename = copied.filename().unwrap();
            copied.save(&temporary.path().join(&filename)).unwrap();
            let sdk = SdkBuilder::new_mock()
                .with_context_provider(provider)
                .with_dump_dir(temporary.path())
                .build()
                .unwrap();
            let verified = ResourceVote::fetch_many_with_metadata_and_proof(&sdk, request, None)
                .await
                .expect("explicit default proof and signature");
            assert_eq!(verified, (legacy.0.unwrap_or_default(), legacy.1, legacy.2));
            verified_vectors += 1;
            println!(
                "verified-vector {namespace} {} {filename} response-sha256 {}",
                file.file_name().unwrap().to_str().unwrap(),
                sha256::Hash::hash(&dump.serialized_response)
            );
            let committed = DumpData::<GetContestedResourceIdentityVotesRequest>::load(
                directory.join(&filename),
            )
            .expect("explicit-default fixture");
            assert_eq!(
                committed.serialized_request.strip_suffix(b"\n").unwrap(),
                copied.serialized_request.as_slice()
            );
            assert_eq!(committed.serialized_response, dump.serialized_response);
        }
    }
    assert_eq!(
        verified_vectors, 6,
        "all original voting requests are covered"
    );
}
