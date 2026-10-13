//! A single authenticated lookup binds first-after index selection and the full identity.
//! These Grove/transport fixtures do not attest the quorum signature; signed FromProof
//! coverage exercises that separate boundary with a deterministic signing fixture.

use dapi_grpc::platform::v0::{platform_server::PlatformServer, Proof, ResponseMetadata};
use dapi_grpc::tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use dapi_grpc::tonic::transport::{Channel, Endpoint, Server};
use dapi_grpc::tonic::{Request, Status};
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::identifier::Identifier;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::error::proof::ProofError;
use drive::error::Error as DriveError;
use drive::grovedb::{Element, Error as GroveError, GroveDb, PathQuery, Query, QueryItem};
use drive::verify::grovedb_proof_envelope::require_supported_grovedb_proof_envelope;
use drive_abci::config::PlatformConfig;
use drive_abci::platform_types::platform::Platform;
use drive_abci::query::QueryService;
use drive_abci::rpc::core::DefaultCoreRPC;
use drive_abci::test::helpers::setup::TestPlatformBuilder;
use hyper_util::rt::TokioIo;
use prost::bytes::BufMut;
use prost::Message;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio_stream::StreamExt;

// Independent wire capture accepts either allocation, so the same test can execute
// against a server with the retired nested result or the standard complete Proof.
#[derive(Clone, PartialEq, Message)]
struct CapturedResponse {
    #[prost(message, optional, tag = "1")]
    v0: Option<CapturedResponseV0>,
}

#[derive(Clone, PartialEq, Message)]
struct CapturedResponseV0 {
    #[prost(message, optional, tag = "2")]
    legacy: Option<CapturedDoubleProof>,
    #[prost(message, optional, tag = "3")]
    metadata: Option<ResponseMetadata>,
    #[prost(message, optional, tag = "4")]
    proof: Option<Proof>,
}

#[derive(Clone, PartialEq, Message)]
struct CapturedDoubleProof {
    #[prost(message, optional, tag = "1")]
    index: Option<Proof>,
    #[prost(bytes = "vec", optional, tag = "2")]
    identity: Option<Vec<u8>>,
}

#[derive(Default)]
struct RawCodec;
struct RawEncoder;
struct RawDecoder;

impl Codec for RawCodec {
    type Encode = Vec<u8>;
    type Decode = CapturedResponse;
    type Encoder = RawEncoder;
    type Decoder = RawDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        RawEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        RawDecoder
    }
}

impl Encoder for RawEncoder {
    type Item = Vec<u8>;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        dst.put_slice(&item);
        Ok(())
    }
}

impl Decoder for RawDecoder {
    type Item = CapturedResponse;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Status> {
        CapturedResponse::decode(src)
            .map(Some)
            .map_err(|error| Status::internal(error.to_string()))
    }
}

struct Fixture {
    platform: Arc<Platform<DefaultCoreRPC>>,
    _directory: TempDir,
    version: &'static PlatformVersion,
    hash: [u8; 20],
    identities: Vec<Identity>,
}

impl Fixture {
    fn new(version: &'static PlatformVersion) -> Self {
        let mut config = PlatformConfig::default_for_network(Network::Testnet);
        config.core.consensus_rpc.host = "http://127.0.0.1".to_owned();
        config.core.consensus_rpc.port = 1;
        let temporary = TestPlatformBuilder::new()
            .with_config(config)
            .build_with_default_rpc();
        let mut state = temporary.platform.state.load_full().as_ref().clone();
        state.current_protocol_version_in_consensus = version.protocol_version;
        temporary.platform.state.store(Arc::new(state));
        let drive = &temporary.platform.drive;
        drive
            .create_initial_state_structure(None, version)
            .expect("initialize query state");
        let mut rng = StdRng::seed_from_u64(506);
        let key = IdentityPublicKey::random_voting_key_with_rng(3, &mut rng, version)
            .expect("deterministic voting key")
            .0;
        let hash = key.public_key_hash().expect("key hash");
        let mut identities = Vec::new();
        // Insertion order deliberately differs from lexicographic identity-ID order.
        for (id, seed) in [(0x30, 16), (0x10, 14), (0x20, 15)] {
            let mut identity =
                Identity::random_identity(3, Some(seed), version).expect("deterministic identity");
            identity.set_id(Identifier::new([id; 32]));
            identity.set_balance(700_000 + u64::from(id));
            identity.set_revision(2);
            identity.add_public_key(key.clone());
            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    version,
                )
                .expect("store full identity and key index");
            identities.push(identity);
        }
        identities.sort_by_key(|identity| identity.id());
        Self {
            platform: Arc::new(temporary.platform),
            _directory: temporary.tempdir,
            version,
            hash,
            identities,
        }
    }

    fn root(&self) -> [u8; 32] {
        self.platform
            .drive
            .grove
            .root_hash(None, &self.version.drive.grove_version)
            .unwrap()
            .expect("stored root")
    }

    fn client(&self) -> (Channel, tokio::task::JoinHandle<()>) {
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let service = QueryService::new(Arc::clone(&self.platform));
        let server = tokio::spawn(async move {
            let incoming = tokio_stream::iter([Ok::<_, std::io::Error>(server_io)])
                .chain(tokio_stream::pending());
            Server::builder()
                .add_service(PlatformServer::new(service))
                .serve_with_incoming(incoming)
                .await
                .expect("serve in-memory query node");
        });
        let mut io = Some(client_io);
        let channel = Endpoint::from_static("http://identity.in-memory")
            .timeout(Duration::from_secs(15))
            .connect_with_connector_lazy(tower::service_fn(move |_| {
                let io = io.take();
                async move {
                    io.map(TokioIo::new)
                        .ok_or_else(|| std::io::Error::other("single-use channel"))
                }
            }));
        (channel, server)
    }
}

fn versions() -> [&'static PlatformVersion; 3] {
    [
        PlatformVersion::get(12).expect("PV12"),
        PlatformVersion::get(13).expect("PV13"),
        PlatformVersion::latest(),
    ]
}

fn bounded_query(fixture: &Fixture, id: [u8; 32], after: Option<[u8; 32]>) -> PathQuery {
    let mut index = Drive::identity_id_by_non_unique_public_key_hash_query(fixture.hash, after);
    let range = match after {
        Some(after) => QueryItem::RangeAfterToInclusive(after.to_vec()..=id.to_vec()),
        None => QueryItem::RangeToInclusive(..=id.to_vec()),
    };
    index
        .query
        .query
        .set_subquery(Query::new_single_query_item(range));
    let identity = Drive::full_identity_query(&id, &fixture.version.drive.grove_version)
        .expect("existing full identity query");
    PathQuery::merge(
        vec![&index, &identity],
        &fixture.version.drive.grove_version,
    )
    .expect("limit-free bounded merge")
}

fn verify_complete(fixture: &Fixture, proof: &[u8], expected: &Identity, after: Option<[u8; 32]>) {
    let id = expected.id().to_buffer();
    let (bootstrap_root, selected) = Drive::verify_identity_id_by_non_unique_public_key_hash(
        proof,
        true,
        fixture.hash,
        after,
        fixture.version,
    )
    .expect("original cap1 subset bootstrap");
    assert_eq!(selected, Some(id));
    let (root, rows) = GroveDb::verify_query(
        proof,
        &bounded_query(fixture, id, after),
        &fixture.version.drive.grove_version,
    )
    .expect("strict bounded composition");
    assert_eq!(root, fixture.root());
    assert_eq!(bootstrap_root, root);
    let mut index_path =
        Drive::identity_id_by_non_unique_public_key_hash_query(fixture.hash, after).path;
    index_path.push(fixture.hash.to_vec());
    let index_rows: Vec<_> = rows
        .iter()
        .filter(|(path, _, _)| *path == index_path)
        .collect();
    assert_eq!(
        index_rows.len(),
        1,
        "first-after range contains exactly one ID"
    );
    assert_eq!(index_rows[0].1, id);
    assert!(matches!(index_rows[0].2, Some(Element::Item(ref value, _)) if value.is_empty()));
    let (identity_root, identity) =
        Drive::verify_full_identity_by_identity_id(proof, true, id, fixture.version)
            .expect("existing full-identity subset decoder");
    assert_eq!(identity_root, root);
    assert_eq!(identity, Some(expected.clone()));
}

fn request_bytes(hash: [u8; 20], after: Option<[u8; 32]>) -> Vec<u8> {
    // Existing V0: outer message tag1; hash tag1, optional cursor tag2, prove tag3.
    let mut inner = vec![0x0a, 20];
    inner.extend_from_slice(&hash);
    if let Some(after) = after {
        inner.extend_from_slice(&[0x12, 32]);
        inner.extend_from_slice(&after);
    }
    inner.extend_from_slice(&[0x18, 1]);
    let mut outer = vec![0x0a, inner.len() as u8];
    outer.extend(inner);
    outer
}

async fn capture(fixture: &Fixture, after: Option<[u8; 32]>) -> CapturedResponse {
    let (channel, server) = fixture.client();
    let mut client = dapi_grpc::tonic::client::Grpc::new(channel);
    client.ready().await.expect("transport ready");
    let response = client
        .unary(
            Request::new(request_bytes(fixture.hash, after)),
            dapi_grpc::tonic::codegen::http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getIdentityByNonUniquePublicKeyHash",
            ),
            RawCodec,
        )
        .await
        .expect("real tonic identity response")
        .into_inner();
    server.abort();
    response
}

fn verify_absence(fixture: &Fixture, proof: &[u8], hash: [u8; 20], after: Option<[u8; 32]>) {
    require_supported_grovedb_proof_envelope(proof, "absence proof")
        .expect("supported absence envelope");
    let query = Drive::identity_id_by_non_unique_public_key_hash_query(hash, after);
    let (root, rows) = GroveDb::verify_query(proof, &query, &fixture.version.drive.grove_version)
        .expect("strict complete-range absence");
    assert_eq!(root, fixture.root());
    assert!(
        rows.is_empty(),
        "absence must authenticate zero remaining IDs"
    );
    let (subset_root, id) = Drive::verify_identity_id_by_non_unique_public_key_hash(
        proof,
        true,
        hash,
        after,
        fixture.version,
    )
    .expect("absence subset bootstrap");
    assert_eq!(subset_root, root);
    assert_eq!(id, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_return_one_complete_nonunique_proof_over_real_transport() {
    let mut wrong_format = Vec::new();
    for version in versions() {
        let fixture = Fixture::new(version);
        for (name, after, expected) in [
            ("first", None, Some(&fixture.identities[0])),
            (
                "next",
                Some(fixture.identities[0].id().to_buffer()),
                Some(&fixture.identities[1]),
            ),
            (
                "exhausted",
                Some(fixture.identities[2].id().to_buffer()),
                None,
            ),
        ] {
            let capture = capture(&fixture, after).await;
            let response = capture.v0.expect("V0 response");
            if response.proof.is_none() || response.legacy.is_some() {
                wrong_format.push(format!(
                    "PV{} {name}: standard Proof at tag4 required, retired tag2 forbidden",
                    version.protocol_version
                ));
                continue;
            }
            let proof = response.proof.expect("standard complete proof");
            match expected {
                Some(expected) => verify_complete(&fixture, &proof.grovedb_proof, expected, after),
                None => verify_absence(&fixture, &proof.grovedb_proof, fixture.hash, after),
            }
            assert_eq!(
                response.metadata.expect("metadata").protocol_version,
                version.protocol_version
            );
        }
    }
    assert!(wrong_format.is_empty(), "{}", wrong_format.join("\n"));
}

#[test]
fn should_verify_bounded_nonunique_composition_with_existing_apis() {
    for version in versions() {
        let fixture = Fixture::new(version);
        for (after, expected) in [
            (None, &fixture.identities[0]),
            (
                Some(fixture.identities[0].id().to_buffer()),
                &fixture.identities[1],
            ),
        ] {
            let query = bounded_query(&fixture, expected.id().to_buffer(), after);
            let proof = fixture
                .platform
                .drive
                .grove_get_proved_path_query(&query, None, &mut vec![], &version.drive)
                .expect("one bounded merged proof");
            require_supported_grovedb_proof_envelope(&proof, "merged proof")
                .expect("supported proof envelope");
            verify_complete(&fixture, &proof, expected, after);
            println!(
                "primitive PV{} after={after:?} merged_bytes={}",
                version.protocol_version,
                proof.len()
            );
        }
        for (hash, after) in [
            ([0xff; 20], None),
            (fixture.hash, Some(fixture.identities[2].id().to_buffer())),
        ] {
            let query = Drive::identity_id_by_non_unique_public_key_hash_query(hash, after);
            let proof = fixture
                .platform
                .drive
                .grove_get_proved_path_query(&query, None, &mut vec![], &version.drive)
                .expect("complete remaining index range");
            require_supported_grovedb_proof_envelope(&proof, "absence proof")
                .expect("supported absence envelope");
            let (root, rows) = GroveDb::verify_query(&proof, &query, &version.drive.grove_version)
                .expect("strict absence");
            assert_eq!(root, fixture.root());
            assert!(
                rows.is_empty(),
                "absence must authenticate zero remaining IDs: {rows:?}"
            );
            let (subset_root, id) = Drive::verify_identity_id_by_non_unique_public_key_hash(
                &proof, true, hash, after, version,
            )
            .expect("absence subset bootstrap");
            assert_eq!(subset_root, root);
            assert_eq!(id, None);
        }
    }
}

#[test]
fn should_expose_skipped_earlier_identity_and_reject_wrong_identity_component() {
    for version in versions() {
        let fixture = Fixture::new(version);
        let first = fixture.identities[0].id().to_buffer();
        let later = fixture.identities[1].id().to_buffer();
        let later_query = bounded_query(&fixture, later, None);
        let later_proof = fixture
            .platform
            .drive
            .grove_get_proved_path_query(&later_query, None, &mut vec![], &version.drive)
            .expect("proof for a later endpoint includes the earlier ID");
        let (root, rows) =
            GroveDb::verify_query(&later_proof, &later_query, &version.drive.grove_version)
                .expect("complete later prefix");
        assert_eq!(root, fixture.root());
        let mut index_path =
            Drive::identity_id_by_non_unique_public_key_hash_query(fixture.hash, None).path;
        index_path.push(fixture.hash.to_vec());
        let ids: Vec<_> = rows
            .iter()
            .filter(|(path, _, _)| *path == index_path)
            .map(|(_, key, _)| key.clone())
            .collect();
        assert_eq!(
            ids,
            vec![first.to_vec(), later.to_vec()],
            "a later endpoint cannot meet cardinality1"
        );
        let bootstrap = Drive::verify_identity_id_by_non_unique_public_key_hash(
            &later_proof,
            true,
            fixture.hash,
            None,
            version,
        );
        assert!(
            matches!(bootstrap, Err(DriveError::GroveDB(error)) if matches!(error.as_ref(), GroveError::InvalidProof(_, _))),
            "cap1 refuses an over-cap prefix proof with a typed proof error"
        );
        assert!(
            GroveDb::verify_query(
                &later_proof,
                &bounded_query(&fixture, first, None),
                &version.drive.grove_version,
            )
            .is_err(),
            "bootstrap rebuild refuses a later identity's component"
        );

        let mut index = Drive::identity_id_by_non_unique_public_key_hash_query(fixture.hash, None);
        index
            .query
            .query
            .set_subquery(Query::new_single_query_item(QueryItem::RangeToInclusive(
                ..=first.to_vec(),
            )));
        let wrong_identity = Drive::full_identity_query(&later, &version.drive.grove_version)
            .expect("wrong identity component");
        let mut membership_index =
            Drive::identity_id_by_non_unique_public_key_hash_query(fixture.hash, None);
        membership_index
            .query
            .query
            .set_subquery(Query::new_single_query_item(QueryItem::Key(later.to_vec())));
        let membership_query = PathQuery::merge(
            vec![&membership_index, &wrong_identity],
            &version.drive.grove_version,
        )
        .expect("membership alone omits the earlier ID");
        let membership_proof = fixture
            .platform
            .drive
            .grove_get_proved_path_query(&membership_query, None, &mut vec![], &version.drive)
            .expect("prove membership of a later ID");
        assert!(
            GroveDb::verify_query(
                &membership_proof,
                &later_query,
                &version.drive.grove_version,
            )
            .is_err(),
            "membership cannot replace first-after range completeness"
        );
        let wrong_query =
            PathQuery::merge(vec![&index, &wrong_identity], &version.drive.grove_version)
                .expect("create deliberately unbound composition");
        let wrong_proof = fixture
            .platform
            .drive
            .grove_get_proved_path_query(&wrong_query, None, &mut vec![], &version.drive)
            .expect("prove unrelated identity");
        assert!(
            GroveDb::verify_query(
                &wrong_proof,
                &bounded_query(&fixture, first, None),
                &version.drive.grove_version,
            )
            .is_err(),
            "original request binds the identity component to the selected index ID"
        );
        println!(
            "adversarial PV{}: skipped earlier ID exposed; wrong identity component rejected",
            version.protocol_version
        );
    }
}

mod signed {
    use super::*;
    use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::GetIdentityByNonUniquePublicKeyHashRequestV0;
    use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::{
        get_identity_by_non_unique_public_key_hash_response_v0::Result as LookupResult,
        GetIdentityByNonUniquePublicKeyHashResponseV0, Version,
    };
    use dapi_grpc::platform::v0::{
        platform_client::PlatformClient, GetIdentityByNonUniquePublicKeyHashRequest,
        GetIdentityByNonUniquePublicKeyHashResponse,
    };
    use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract};
    use drive_proof_verifier::{ContextProvider, ContextProviderError, FromProof};
    use tenderdash_abci::{
        proto::types::{CanonicalVote, SignedMsgType, StateId},
        signatures::{Hashable, Signable},
    };

    struct Provider([u8; 48]);
    impl ContextProvider for Provider {
        fn get_data_contract(
            &self,
            _: &Identifier,
            _: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok(None)
        }
        fn get_token_configuration(
            &self,
            _: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }
        fn get_quorum_public_key(
            &self,
            _: u32,
            _: [u8; 32],
            _: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Ok(self.0)
        }
        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

    fn key() -> SecretKey<Bls12381G2Impl> {
        SecretKey::from_hash(b"nonunique-merged-proof-quorum-fixture")
    }
    fn provider() -> Provider {
        let bytes: Vec<u8> = (&key().public_key()).into();
        Provider(bytes.try_into().expect("compressed quorum public key"))
    }
    fn request(
        hash: [u8; 20],
        after: Option<[u8; 32]>,
        prove: bool,
    ) -> GetIdentityByNonUniquePublicKeyHashRequest {
        GetIdentityByNonUniquePublicKeyHashRequestV0 {
            public_key_hash: hash.to_vec(),
            start_after: after.map(|id| id.to_vec()),
            prove,
        }
        .into()
    }
    fn sign(proof: &mut Proof, metadata: &ResponseMetadata, root: [u8; 32]) {
        proof.quorum_hash = vec![2; 32];
        proof.quorum_type = 106;
        proof.round = 1;
        proof.block_id_hash = vec![3; 32];
        let state = StateId {
            app_version: metadata.protocol_version as u64,
            core_chain_locked_height: metadata.core_chain_locked_height,
            time: metadata.time_ms,
            app_hash: root.to_vec(),
            height: metadata.height,
        };
        let vote = CanonicalVote {
            r#type: SignedMsgType::Precommit.into(),
            block_id: proof.block_id_hash.clone(),
            chain_id: metadata.chain_id.clone(),
            height: metadata.height as i64,
            round: proof.round as i64,
            state_id: state
                .calculate_msg_hash(
                    &metadata.chain_id,
                    metadata.height as i64,
                    proof.round as i32,
                )
                .expect("state digest"),
        };
        let digest = vote
            .calculate_sign_hash(
                &metadata.chain_id,
                proof.quorum_type as u8,
                &[2; 32],
                metadata.height as i64,
                proof.round as i32,
            )
            .expect("vote digest");
        proof.signature = key()
            .sign(SignatureSchemes::Basic, &digest)
            .expect("sign state root")
            .as_raw_value()
            .to_compressed()
            .to_vec();
    }
    fn response(
        mut proof: Proof,
        metadata: ResponseMetadata,
        root: [u8; 32],
    ) -> GetIdentityByNonUniquePublicKeyHashResponse {
        sign(&mut proof, &metadata, root);
        GetIdentityByNonUniquePublicKeyHashResponse {
            version: Some(Version::V0(GetIdentityByNonUniquePublicKeyHashResponseV0 {
                result: Some(LookupResult::Proof(proof)),
                metadata: Some(metadata),
            })),
        }
    }
    fn verify(
        response: GetIdentityByNonUniquePublicKeyHashResponse,
        hash: [u8; 20],
        after: Option<[u8; 32]>,
        security: &PlatformVersion,
    ) -> Result<(Option<Identity>, ResponseMetadata, Proof), Box<drive_proof_verifier::Error>> {
        <Identity as FromProof<GetIdentityByNonUniquePublicKeyHashRequest>>::maybe_from_proof_with_metadata(
            request(hash, after, true), response, Network::Testnet, security, &provider(),
        ).map_err(Box::new)
    }

    #[tokio::test]
    async fn should_bind_transport_selection_payload_and_signed_root_across_versions() {
        for version in versions() {
            let fixture = Fixture::new(version);
            for (after, expected) in [
                (None, Some(fixture.identities[0].clone())),
                (
                    Some(fixture.identities[0].id().to_buffer()),
                    Some(fixture.identities[1].clone()),
                ),
                (Some(fixture.identities[2].id().to_buffer()), None),
            ] {
                let wire = capture(&fixture, after).await;
                let v0 = wire.v0.expect("V0");
                let proof = v0.proof.expect("standard complete proof");
                let metadata = v0.metadata.expect("transport metadata");
                let signed = response(proof.clone(), metadata.clone(), fixture.root());
                let encoded = signed.encode_to_vec();
                let decoded =
                    GetIdentityByNonUniquePublicKeyHashResponse::decode(encoded.as_slice())
                        .expect("new client round trip");
                // A retired decoder cannot treat the new tag 4 as its nested tag 2.
                assert!(CapturedResponse::decode(encoded.as_slice())
                    .expect("retired tag capture")
                    .v0
                    .expect("V0")
                    .legacy
                    .is_none());
                let (identity, returned_metadata, complete) =
                    verify(decoded, fixture.hash, after, PlatformVersion::latest())
                        .expect("valid signed merged lookup");
                assert_eq!(identity, expected);
                assert_eq!(returned_metadata, metadata);
                assert_eq!(complete.grovedb_proof, proof.grovedb_proof);
                assert_eq!(complete.signature.len(), 96);
                let (channel, server) = fixture.client();
                let unproved = PlatformClient::new(channel)
                    .get_identity_by_non_unique_public_key_hash(request(fixture.hash, after, false))
                    .await
                    .expect("unproved transport")
                    .into_inner();
                server.abort();
                let Some(Version::V0(v0)) = unproved.version else {
                    panic!("V0 identity response")
                };
                let Some(LookupResult::Identity(bytes)) = v0.result else {
                    panic!("unproved identity field remains tag 1")
                };
                if let Some(expected) = expected {
                    use dpp::serialization::PlatformDeserializableUntrusted;
                    assert_eq!(
                        Identity::deserialize_from_bytes_untrusted_no_limit(
                            &bytes.identity.expect("serialized identity")
                        )
                        .expect("identity payload"),
                        expected
                    );
                } else {
                    assert!(bytes.identity.is_none());
                }
            }
            let proof = fixture
                .platform
                .drive
                .prove_full_identity_by_non_unique_public_key_hash([0xff; 20], None, None, version)
                .expect("absent hash proof");
            let metadata = ResponseMetadata {
                height: 100,
                core_chain_locked_height: 1,
                epoch: 0,
                time_ms: 1_000_000,
                protocol_version: version.protocol_version,
                chain_id: "test-chain".into(),
            };
            assert!(verify(
                response(
                    Proof {
                        grovedb_proof: proof,
                        ..Proof::default()
                    },
                    metadata,
                    fixture.root()
                ),
                [0xff; 20],
                None,
                PlatformVersion::latest()
            )
            .expect("signed absence")
            .0
            .is_none());
        }
    }

    #[tokio::test]
    async fn should_reject_unbound_roots_and_mutated_signed_metadata() {
        let fixture = Fixture::new(PlatformVersion::get(13).expect("signed PV13"));
        let v0 = capture(&fixture, None).await.v0.expect("V0");
        let proof = v0.proof.expect("proof");
        let metadata = v0.metadata.expect("metadata");
        let valid = response(proof.clone(), metadata.clone(), fixture.root());
        verify(valid.clone(), fixture.hash, None, PlatformVersion::latest())
            .expect("signed PV13 at security floor14");
        let mut wrong_root = fixture.root();
        wrong_root[0] ^= 1;
        assert!(matches!(
            verify(
                response(proof, metadata, wrong_root),
                fixture.hash,
                None,
                PlatformVersion::latest()
            ),
            Err(error) if matches!(error.as_ref(), drive_proof_verifier::Error::InvalidSignature { .. })
        ));
        for field in 0..9 {
            let mut invalid = valid.clone();
            let Some(Version::V0(ref mut v0)) = invalid.version else {
                unreachable!()
            };
            let metadata = v0.metadata.as_mut().expect("metadata");
            let Some(LookupResult::Proof(ref mut proof)) = v0.result else {
                unreachable!()
            };
            match field {
                0 => metadata.protocol_version = 12,
                1 => metadata.height += 1,
                2 => metadata.time_ms += 1,
                3 => metadata.core_chain_locked_height += 1,
                4 => metadata.chain_id.push('x'),
                5 => proof.round += 1,
                6 => proof.block_id_hash[0] ^= 1,
                7 => proof.quorum_hash[0] ^= 1,
                _ => proof.signature[0] ^= 1,
            }
            assert!(
                verify(invalid, fixture.hash, None, PlatformVersion::latest()).is_err(),
                "signed field {field} cannot be changed"
            );
        }
        for protocol_version in [
            0,
            PlatformVersion::latest().protocol_version + 1,
            0x1000_0000,
            0x1000_0001,
        ] {
            let mut invalid = valid.clone();
            let Some(Version::V0(ref mut v0)) = invalid.version else {
                unreachable!()
            };
            v0.metadata.as_mut().expect("metadata").protocol_version = protocol_version;
            assert!(
                matches!(
                    verify(invalid, fixture.hash, None, PlatformVersion::latest()),
                    Err(error) if matches!(error.as_ref(), drive_proof_verifier::Error::ResponseDecodeError { .. })
                ),
                "unsupported query version {protocol_version} is an error, not a mock-table panic"
            );
        }
    }

    #[tokio::test]
    async fn should_reject_signed_skipped_incomplete_and_unrelated_identity_proofs() {
        for version in versions() {
            let fixture = Fixture::new(version);
            let first = fixture.identities[0].id().to_buffer();
            let later = fixture.identities[1].id().to_buffer();
            let v0 = capture(&fixture, None).await.v0.expect("V0");
            let metadata = v0.metadata.expect("metadata");
            let index = Drive::identity_id_by_non_unique_public_key_hash_query(fixture.hash, None);
            let mut membership = index.clone();
            let mut only_later = Query::new();
            only_later.insert_key(later.to_vec());
            membership.query.query.default_subquery_branch.subquery = Some(Box::new(only_later));
            let mismatched = PathQuery::merge(
                vec![
                    &bounded_query(&fixture, first, None),
                    &Drive::full_identity_query(&later, &version.drive.grove_version)
                        .expect("full later identity"),
                ],
                &version.drive.grove_version,
            )
            .expect("unrelated identity query");
            fixture
                .platform
                .drive
                .grove
                .insert(
                    &[] as &[&[u8]],
                    &[0xff],
                    Element::new_item(vec![0x51; 32]),
                    None,
                    None,
                    &version.drive.grove_version,
                )
                .unwrap()
                .expect("unrelated stored scalar");
            let mut unrelated = Query::new();
            unrelated.insert_key(vec![0xff]);
            let extra = PathQuery::new_unsized(vec![], unrelated);
            let expanded = PathQuery::merge(
                vec![&bounded_query(&fixture, first, None), &extra],
                &version.drive.grove_version,
            )
            .expect("unrelated branch query");
            let scalar_blob = fixture
                .platform
                .drive
                .grove
                .prove_query(&expanded, None, &version.drive.grove_version)
                .unwrap()
                .expect("extra scalar proof");
            let original = bounded_query(&fixture, first, None);
            let canonical_blob = fixture
                .platform
                .drive
                .grove
                .prove_query(&original, None, &version.drive.grove_version)
                .unwrap()
                .expect("original proof");
            let (scalar_root, scalar_rows) =
                GroveDb::verify_query(&scalar_blob, &original, &version.drive.grove_version)
                    .expect("strict query result for scalar diagnostic");
            println!("scalar witness diagnostic PV{} expanded={} canonical={} equal_bytes={} strict_root={} strict_rows={} unrequested_scalar_in_rows={}", version.protocol_version, scalar_blob.len(), canonical_blob.len(), scalar_blob==canonical_blob, hex::encode(scalar_root), scalar_rows.len(), scalar_rows.iter().any(|(path,key,_)| path.is_empty() && key==&vec![0xff]));
            assert_eq!(scalar_root, fixture.root());
            assert!(!scalar_rows
                .iter()
                .any(|(path, key, _)| path.is_empty() && key == &vec![0xff]));
            // Non-queried Merk authentication nodes are not additional query results.
            // They cannot change the selected identity or bypass the signed root check.
            let (identity, returned_metadata, returned_proof) = verify(
                response(
                    Proof {
                        grovedb_proof: scalar_blob.clone(),
                        ..Proof::default()
                    },
                    metadata.clone(),
                    fixture.root(),
                ),
                fixture.hash,
                None,
                version,
            )
            .expect("extra authentication data preserves the original result");
            assert_eq!(identity, Some(fixture.identities[0].clone()));
            assert_eq!(returned_metadata, metadata);
            assert_eq!(returned_proof.grovedb_proof, scalar_blob);
            for (case, query) in [
                ("later prefix", bounded_query(&fixture, later, None)),
                ("later membership", membership),
                (
                    "identity only",
                    Drive::full_identity_query(&first, &version.drive.grove_version)
                        .expect("full first identity"),
                ),
                ("index only", index),
                ("extra identity", mismatched),
            ] {
                let blob = fixture
                    .platform
                    .drive
                    .grove
                    .prove_query(&query, None, &version.drive.grove_version)
                    .unwrap()
                    .expect("malicious request-shaped proof at authentic root");
                let signed = response(
                    Proof {
                        grovedb_proof: blob,
                        ..Proof::default()
                    },
                    metadata.clone(),
                    fixture.root(),
                );
                assert!(verify(signed, fixture.hash, None, version).is_err(), "PV{} case={case}: a valid root/signature cannot legitimize a different selection or incomplete identity", version.protocol_version);
            }
        }
    }

    #[test]
    fn should_reject_malformed_index_values_keys_and_missing_identity() {
        for version in versions() {
            for case in 0..2 {
                let fixture = Fixture::new(version);
                let first = fixture.identities[0].id().to_buffer();
                let mut index =
                    Drive::identity_id_by_non_unique_public_key_hash_query(fixture.hash, None);
                let mut index_path = index.path.clone();
                index_path.push(fixture.hash.to_vec());
                let (key, element) = match case {
                    0 => (first.to_vec(), Element::new_item(vec![0x51])),
                    1 => (vec![0x01; 31], Element::new_item(vec![])),
                    _ => unreachable!("only malformed value and malformed length"),
                };
                fixture
                    .platform
                    .drive
                    .grove
                    .insert(
                        index_path.as_slice(),
                        &key,
                        element,
                        None,
                        None,
                        &version.drive.grove_version,
                    )
                    .unwrap()
                    .expect("malformed committed index fixture");
                index.query.query.set_subquery(Query::new_single_query_item(
                    QueryItem::RangeToInclusive(..=key),
                ));
                let full = Drive::full_identity_query(&first, &version.drive.grove_version)
                    .expect("unrelated first identity");
                let query = PathQuery::merge(vec![&index, &full], &version.drive.grove_version)
                    .expect("proof of malformed index entry");
                let blob = fixture
                    .platform
                    .drive
                    .grove
                    .prove_query(&query, None, &version.drive.grove_version)
                    .unwrap()
                    .expect("proof at actual stored root");
                Drive::verify_full_identity_by_non_unique_public_key_hash(
                    &blob,
                    fixture.hash,
                    None,
                    version,
                    version,
                )
                .expect_err("malformed index cannot produce a valid lookup");
            }
            let fixture = Fixture::new(version);
            let missing_id = [0; 32];
            assert!(fixture
                .identities
                .iter()
                .all(|identity| missing_id < identity.id().to_buffer()));
            let mut index_path =
                Drive::identity_id_by_non_unique_public_key_hash_query(fixture.hash, None).path;
            index_path.push(fixture.hash.to_vec());
            fixture
                .platform
                .drive
                .grove
                .insert(
                    index_path.as_slice(),
                    &missing_id,
                    Element::new_item(vec![]),
                    None,
                    None,
                    &version.drive.grove_version,
                )
                .unwrap()
                .expect("commit dangling indexed identity");
            let blob = fixture
                .platform
                .drive
                .prove_full_identity_by_non_unique_public_key_hash(
                    fixture.hash,
                    None,
                    None,
                    version,
                )
                .expect("actual producer proves the indexed identity absence");
            let (root, identity) =
                Drive::verify_full_identity_by_identity_id(&blob, true, missing_id, version)
                    .expect("full identity absence is complete at the stored root");
            assert_eq!(root, fixture.root());
            assert_eq!(identity, None);
            let error = Drive::verify_full_identity_by_non_unique_public_key_hash(
                &blob,
                fixture.hash,
                None,
                version,
                version,
            )
            .expect_err("a dangling index is not an absent hash lookup");
            assert!(
                matches!(
                    error,
                    DriveError::Proof(ProofError::IncompleteProof(
                        "non-unique index references a missing full identity"
                    ))
                ),
                "PV{} dangling indexed identity: {error:?}",
                version.protocol_version
            );
            let fixture = Fixture::new(version);
            let wire = fixture
                .platform
                .drive
                .prove_full_identity_by_non_unique_public_key_hash(
                    fixture.hash,
                    None,
                    None,
                    version,
                )
                .expect("complete proof");
            for bytes in [vec![], wire[..wire.len() / 2].to_vec(), vec![0], vec![1]] {
                assert!(
                    Drive::verify_full_identity_by_non_unique_public_key_hash(
                        &bytes,
                        fixture.hash,
                        None,
                        version,
                        version
                    )
                    .is_err(),
                    "truncated or unsupported envelope cannot become absence"
                );
            }
        }
    }

    #[tokio::test]
    async fn should_export_deterministic_single_proof_sdk_vectors() {
        let temporary = TempDir::new().expect("scratch export");
        let destination = std::env::var_os("NONUNIQUE_VECTOR_OUTPUT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| temporary.path().to_path_buf());
        std::fs::create_dir_all(&destination).expect("scratch vector directory");
        let fixture = Fixture::new(PlatformVersion::latest());
        let mut after = None;
        let mut direct_vectors = Vec::new();
        for case in 0..4 {
            let v0 = capture(&fixture, after).await.v0.expect("V0");
            let signed = response(
                v0.proof.expect("proof"),
                v0.metadata.expect("metadata"),
                fixture.root(),
            );
            let (identity, _, _) = verify(signed.clone(), fixture.hash, after, fixture.version)
                .expect("verify vector before export");
            direct_vectors.push(serde_json::json!({
                "proof": hex::encode(verify(signed.clone(), fixture.hash, after, fixture.version).expect("verified blob").2.grovedb_proof),
                "root": hex::encode(fixture.root()), "hash": hex::encode(fixture.hash),
                "after": after.map(hex::encode), "identity_id": identity.as_ref().map(|id| hex::encode(id.id().to_buffer())),
                "balance": identity.as_ref().map(|id| id.balance()), "revision": identity.as_ref().map(|id| id.revision()),
                "key_count": identity.as_ref().map(|id| id.public_keys().len()),
            }));
            let req = serde_json::json!({"GetIdentityByNonUniquePublicKeyHash": request(fixture.hash, after, true)});
            let resp = serde_json::json!({"GetIdentityByNonUniquePublicKeyHash": signed});
            let request_bytes = serde_json::to_vec_pretty(&req).expect("request JSON");
            let response_bytes = serde_json::to_vec_pretty(&resp).expect("response JSON");
            let mut bytes = request_bytes;
            bytes.extend_from_slice(b"\n\0\n");
            bytes.extend(
                serde_json::to_vec(&serde_json::json!({"Ok": response_bytes}))
                    .expect("mock result"),
            );
            std::fs::write(
                destination.join(format!("msg_IdentityRequest_{case}.json")),
                bytes,
            )
            .expect("write scratch vector");
            after = identity.map(|identity| identity.id().to_buffer());
        }
        std::fs::write(
            destination.join(format!("quorum_pubkey-106-{}.json", hex::encode([2; 32]))),
            hex::encode(provider().0),
        )
        .expect("quorum key vector");
        std::fs::write(
            destination.join("nonunique-fixtures.json"),
            serde_json::to_vec_pretty(&direct_vectors).expect("direct vectors"),
        )
        .expect("write direct WASM vectors");
        println!(
            "deterministic SDK vector hash={}",
            hex::encode(fixture.hash)
        );
    }
}
