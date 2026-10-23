//! getIdentityByNonUniquePublicKeyHash over a real gRPC transport, in both request versions.
//!
//! A V0 request is answered with two proofs, one of the first index entry after the cursor
//! and one of that identity, at every protocol version. A V1 request is answered with one
//! proof that binds the index selection and the full identity, and only protocol versions
//! whose bounds accept V1 serve it. The wire is read with hand-written prost types, so a
//! field number that moved cannot hide behind the generated code. The transport tests
//! check proofs against the stored root; the `signed` module re-signs captured answers with
//! a deterministic quorum key and verifies them through `FromProof`.

use dapi_grpc::platform::v0::{platform_server::PlatformServer, Proof, ResponseMetadata};
use dapi_grpc::tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use dapi_grpc::tonic::transport::{Channel, Endpoint, Server};
use dapi_grpc::tonic::{Code, Request, Status};
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::identifier::Identifier;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::serialization::PlatformDeserializableUntrusted;
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

// Wire layout of the response, written independently of the generated types.
#[derive(Clone, PartialEq, Message)]
struct CapturedResponse {
    #[prost(message, optional, tag = "1")]
    v0: Option<CapturedResponseV0>,
    #[prost(message, optional, tag = "2")]
    v1: Option<CapturedResponseV1>,
}

#[derive(Clone, PartialEq, Message)]
struct CapturedResponseV0 {
    #[prost(message, optional, tag = "1")]
    identity: Option<CapturedIdentity>,
    #[prost(message, optional, tag = "2")]
    proof: Option<CapturedTwoProofs>,
    #[prost(message, optional, tag = "3")]
    metadata: Option<ResponseMetadata>,
    // Reserved: clients that know only field 2 would see no proof in a V0 answer here.
    #[prost(message, optional, tag = "4")]
    reserved: Option<Proof>,
}

#[derive(Clone, PartialEq, Message)]
struct CapturedTwoProofs {
    #[prost(message, optional, tag = "1")]
    index: Option<Proof>,
    #[prost(bytes = "vec", optional, tag = "2")]
    identity: Option<Vec<u8>>,
}

#[derive(Clone, PartialEq, Message)]
struct CapturedResponseV1 {
    #[prost(message, optional, tag = "1")]
    identity: Option<CapturedIdentity>,
    #[prost(message, optional, tag = "2")]
    proof: Option<Proof>,
    #[prost(message, optional, tag = "3")]
    metadata: Option<ResponseMetadata>,
}

#[derive(Clone, PartialEq, Message)]
struct CapturedIdentity {
    #[prost(bytes = "vec", optional, tag = "1")]
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

/// The request version, which chooses the shape of a proved answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Envelope {
    /// Answered with separate index and identity proofs.
    V0,
    /// Answered with one proof of the index selection and the identity.
    V1,
}

impl Envelope {
    /// The protocol versions under test whose bounds accept this request version.
    fn served_versions(self) -> Vec<&'static PlatformVersion> {
        match self {
            Envelope::V0 => versions(),
            Envelope::V1 => versions()
                .into_iter()
                .filter(|version| version.protocol_version >= 14)
                .collect(),
        }
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

    fn id(&self, index: usize) -> [u8; 32] {
        self.identities[index].id().to_buffer()
    }

    fn prove(&self, query: &PathQuery) -> Vec<u8> {
        self.platform
            .drive
            .grove_get_proved_path_query(query, None, &mut vec![], &self.version.drive)
            .expect("proof at the stored root")
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

/// Every protocol version from 12 through this build's latest.
fn versions() -> Vec<&'static PlatformVersion> {
    (12..=PlatformVersion::latest().protocol_version)
        .map(|version| PlatformVersion::get(version).expect("known protocol version"))
        .collect()
}

/// One lookup and the identity a correct answer to it holds.
struct Case<'a> {
    name: &'static str,
    hash: [u8; 20],
    after: Option<[u8; 32]>,
    expected: Option<&'a Identity>,
}

/// Every lookup the paging contract covers: each page, a cursor before, between and past
/// the stored identity IDs (0x10.., 0x20.. and 0x30..), and a key hash no identity uses.
fn cases(fixture: &Fixture) -> Vec<Case<'_>> {
    let ids: Vec<_> = (0..3).map(|index| fixture.id(index)).collect();
    assert_eq!(ids, [[0x10; 32], [0x20; 32], [0x30; 32]]);
    let identities = &fixture.identities;
    let case = |name, after, expected| Case {
        name,
        hash: fixture.hash,
        after,
        expected,
    };
    vec![
        case("first", None, Some(&identities[0])),
        case("after the first", Some(ids[0]), Some(&identities[1])),
        case("after the second", Some(ids[1]), Some(&identities[2])),
        case("exhausted after the third", Some(ids[2]), None),
        case("cursor before all", Some([0x01; 32]), Some(&identities[0])),
        case("cursor between", Some([0x18; 32]), Some(&identities[1])),
        case("cursor past all", Some([0xf0; 32]), None),
        Case {
            name: "unknown key hash",
            hash: [0xff; 20],
            after: None,
            expected: None,
        },
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

fn request_bytes(
    envelope: Envelope,
    hash: [u8; 20],
    after: Option<[u8; 32]>,
    prove: bool,
) -> Vec<u8> {
    // Both versions lay out the same fields: hash 1, optional cursor 2, prove 3. The
    // outer oneof field is the version: V0 is field 1, V1 is field 2.
    let mut inner = vec![0x0a, 20];
    inner.extend_from_slice(&hash);
    if let Some(after) = after {
        inner.extend_from_slice(&[0x12, 32]);
        inner.extend_from_slice(&after);
    }
    if prove {
        inner.extend_from_slice(&[0x18, 1]);
    }
    let field = match envelope {
        Envelope::V0 => 0x0a,
        Envelope::V1 => 0x12,
    };
    let mut outer = vec![field, inner.len() as u8];
    outer.extend(inner);
    outer
}

/// Sends one request over the real gRPC transport. A request the node refuses comes back
/// as the error status.
async fn capture(
    fixture: &Fixture,
    envelope: Envelope,
    hash: [u8; 20],
    after: Option<[u8; 32]>,
    prove: bool,
) -> Result<CapturedResponse, Status> {
    let (channel, server) = fixture.client();
    let mut client = dapi_grpc::tonic::client::Grpc::new(channel);
    client.ready().await.expect("transport ready");
    let response = client
        .unary(
            Request::new(request_bytes(envelope, hash, after, prove)),
            dapi_grpc::tonic::codegen::http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getIdentityByNonUniquePublicKeyHash",
            ),
            RawCodec,
        )
        .await
        .map(|response| response.into_inner());
    server.abort();
    response
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_answer_each_request_version_in_its_own_shape_over_real_transport() {
    for envelope in [Envelope::V0, Envelope::V1] {
        for version in envelope.served_versions() {
            let fixture = Fixture::new(version);
            for case in cases(&fixture) {
                let context = format!("{envelope:?} PV{} {}", version.protocol_version, case.name);
                let wire = capture(&fixture, envelope, case.hash, case.after, true)
                    .await
                    .unwrap_or_else(|status| panic!("{context}: refused with {status}"));
                let metadata = match envelope {
                    Envelope::V0 => {
                        assert_eq!(wire.v1, None, "{context}: answered in V1");
                        let v0 = wire.v0.expect("V0 answer");
                        assert_eq!(v0.identity, None, "{context}: unproved result");
                        assert_eq!(v0.reserved, None, "{context}: proof in reserved field 4");
                        let proved = v0.proof.expect("two proofs in result field 2");
                        let index = proved.index.expect("index proof");
                        // The node answers V0 with the two-proof producer's output.
                        let produced = fixture
                            .platform
                            .drive
                            .prove_full_identity_by_non_unique_public_key_hash_two_proofs(
                                case.hash, case.after, None, version,
                            )
                            .expect("two-proof producer");
                        assert_eq!(
                            index.grovedb_proof, produced.identity_id_public_key_hash_proof,
                            "{context}"
                        );
                        assert_eq!(proved.identity, produced.identity_proof, "{context}");
                        // An absence answer carries no identity proof.
                        assert_eq!(
                            proved.identity.is_some(),
                            case.expected.is_some(),
                            "{context}"
                        );
                        let (root, identity) =
                            Drive::verify_full_identity_by_non_unique_public_key_hash_two_proofs(
                                &index.grovedb_proof,
                                proved.identity.as_deref(),
                                case.hash,
                                case.after,
                                version,
                                version,
                            )
                            .unwrap_or_else(|error| panic!("{context}: {error}"));
                        assert_eq!(root, fixture.root(), "{context}");
                        assert_eq!(identity.as_ref(), case.expected, "{context}");
                        v0.metadata
                    }
                    Envelope::V1 => {
                        assert_eq!(wire.v0, None, "{context}: answered in V0");
                        let v1 = wire.v1.expect("V1 answer");
                        assert_eq!(v1.identity, None, "{context}: unproved result");
                        let proof = v1.proof.expect("one proof in result field 2");
                        // The node answers V1 with the single-proof producer's output.
                        assert_eq!(
                            proof.grovedb_proof,
                            fixture
                                .platform
                                .drive
                                .prove_full_identity_by_non_unique_public_key_hash(
                                    case.hash, case.after, None, version,
                                )
                                .expect("single-proof producer"),
                            "{context}"
                        );
                        match case.expected {
                            Some(expected) => verify_complete(
                                &fixture,
                                &proof.grovedb_proof,
                                expected,
                                case.after,
                            ),
                            None => verify_absence(
                                &fixture,
                                &proof.grovedb_proof,
                                case.hash,
                                case.after,
                            ),
                        }
                        v1.metadata
                    }
                };
                assert_eq!(
                    metadata.expect("metadata").protocol_version,
                    version.protocol_version,
                    "{context}"
                );
            }
        }
    }
}

/// Protocol versions below 14 accept only V0. The refusal is a retryable status rather than
/// an answer, because another node may run a protocol version that serves V1.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_refuse_v1_below_protocol_14_over_real_transport() {
    for version in [
        PlatformVersion::get(12).expect("PV12"),
        PlatformVersion::get(13).expect("PV13"),
    ] {
        let fixture = Fixture::new(version);
        let served = capture(&fixture, Envelope::V0, fixture.hash, None, true)
            .await
            .expect("the same node serves V0");
        assert!(served.v0.is_some_and(|v0| v0.proof.is_some()));
        for prove in [true, false] {
            let context = format!("PV{} prove={prove}", version.protocol_version);
            let status = capture(&fixture, Envelope::V1, fixture.hash, None, prove)
                .await
                .expect_err(&context);
            assert_eq!(status.code(), Code::Unknown, "{context}: {status}");
            assert!(
                status.message().starts_with(
                    "unsupported version for query: identity_by_non_unique_public_key_hash,"
                ) && status.message().ends_with("given 1"),
                "{context}: {status}"
            );
        }
    }
}

/// Unproved answers do not depend on the request version: both carry the serialized
/// identity in result field 1.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_answer_unproved_v0_and_v1_with_the_same_identity_bytes() {
    for version in versions() {
        let fixture = Fixture::new(version);
        let v1_served = Envelope::V1
            .served_versions()
            .iter()
            .any(|served| served.protocol_version == version.protocol_version);
        for case in cases(&fixture) {
            let context = format!("PV{} {}", version.protocol_version, case.name);
            let v0 = capture(&fixture, Envelope::V0, case.hash, case.after, false)
                .await
                .expect("unproved V0")
                .v0
                .expect("V0 answer");
            assert_eq!(v0.proof, None, "{context}");
            let bytes = v0.identity.expect("identity result in field 1").identity;
            assert_eq!(
                bytes.as_deref().map(|bytes| {
                    Identity::deserialize_from_bytes_untrusted_no_limit(bytes)
                        .expect("identity payload")
                }),
                case.expected.cloned(),
                "{context}"
            );
            if v1_served {
                let v1 = capture(&fixture, Envelope::V1, case.hash, case.after, false)
                    .await
                    .expect("unproved V1")
                    .v1
                    .expect("V1 answer");
                assert_eq!(v1.proof, None, "{context}");
                assert_eq!(
                    v1.identity.expect("identity result in field 1").identity,
                    bytes,
                    "{context}"
                );
            }
        }
    }
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
    }
}

mod signed {
    use super::*;
    use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::{
        GetIdentityByNonUniquePublicKeyHashRequestV0, GetIdentityByNonUniquePublicKeyHashRequestV1,
    };
    use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::{
        get_identity_by_non_unique_public_key_hash_response_v0::{
            IdentityProvedResponse, Result as V0Result,
        },
        get_identity_by_non_unique_public_key_hash_response_v1::Result as V1Result,
        Version,
    };
    use dapi_grpc::platform::v0::{
        GetIdentityByNonUniquePublicKeyHashRequest, GetIdentityByNonUniquePublicKeyHashResponse,
    };
    use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
    use dpp::data_contract::TokenConfiguration;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::prelude::{CoreBlockHeight, DataContract};
    use drive_proof_verifier::{ContextProvider, ContextProviderError, Error, FromProof};
    use tenderdash_abci::{
        proto::types::{CanonicalVote, SignedMsgType, StateId},
        signatures::{Hashable, Signable},
    };

    type Verified = Result<(Option<Identity>, ResponseMetadata, Proof), Box<Error>>;

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
        envelope: Envelope,
        hash: [u8; 20],
        after: Option<[u8; 32]>,
        prove: bool,
    ) -> GetIdentityByNonUniquePublicKeyHashRequest {
        let (public_key_hash, start_after) = (hash.to_vec(), after.map(|id| id.to_vec()));
        match envelope {
            Envelope::V0 => GetIdentityByNonUniquePublicKeyHashRequestV0 {
                public_key_hash,
                start_after,
                prove,
            }
            .into(),
            Envelope::V1 => GetIdentityByNonUniquePublicKeyHashRequestV1 {
                public_key_hash,
                start_after,
                prove,
            }
            .into(),
        }
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

    /// Signs a proved answer over `root`, as a quorum would, and decodes its wire bytes
    /// with the generated client type. A V0 answer's signature travels with its index proof.
    fn signed(
        mut wire: CapturedResponse,
        root: [u8; 32],
    ) -> GetIdentityByNonUniquePublicKeyHashResponse {
        let (proof, metadata) = match (&mut wire.v0, &mut wire.v1) {
            (Some(v0), None) => (
                v0.proof
                    .as_mut()
                    .and_then(|proved| proved.index.as_mut())
                    .expect("V0 index proof"),
                v0.metadata.as_ref().expect("V0 metadata"),
            ),
            (None, Some(v1)) => (
                v1.proof.as_mut().expect("V1 proof"),
                v1.metadata.as_ref().expect("V1 metadata"),
            ),
            _ => panic!("an answer has exactly one version"),
        };
        sign(proof, metadata, root);
        GetIdentityByNonUniquePublicKeyHashResponse::decode(wire.encode_to_vec().as_slice())
            .expect("generated client type decodes the wire bytes")
    }

    fn two_proof_answer(
        index: Vec<u8>,
        identity: Option<Vec<u8>>,
        metadata: ResponseMetadata,
    ) -> CapturedResponse {
        CapturedResponse {
            v0: Some(CapturedResponseV0 {
                proof: Some(CapturedTwoProofs {
                    index: Some(Proof {
                        grovedb_proof: index,
                        ..Proof::default()
                    }),
                    identity,
                }),
                metadata: Some(metadata),
                ..CapturedResponseV0::default()
            }),
            v1: None,
        }
    }

    fn single_proof_answer(proof: Vec<u8>, metadata: ResponseMetadata) -> CapturedResponse {
        CapturedResponse {
            v0: None,
            v1: Some(CapturedResponseV1 {
                proof: Some(Proof {
                    grovedb_proof: proof,
                    ..Proof::default()
                }),
                metadata: Some(metadata),
                identity: None,
            }),
        }
    }

    /// Metadata for answers this test assembles from Drive proofs rather than captures.
    fn fixed_metadata(version: &PlatformVersion) -> ResponseMetadata {
        ResponseMetadata {
            height: 100,
            core_chain_locked_height: 1,
            epoch: 0,
            time_ms: 1_000_000,
            protocol_version: version.protocol_version,
            chain_id: "test-chain".into(),
        }
    }

    async fn signed_capture(
        fixture: &Fixture,
        envelope: Envelope,
        hash: [u8; 20],
        after: Option<[u8; 32]>,
    ) -> GetIdentityByNonUniquePublicKeyHashResponse {
        let wire = capture(fixture, envelope, hash, after, true)
            .await
            .unwrap_or_else(|status| panic!("{envelope:?} refused: {status}"));
        signed(wire, fixture.root())
    }

    fn verify(
        envelope: Envelope,
        response: GetIdentityByNonUniquePublicKeyHashResponse,
        hash: [u8; 20],
        after: Option<[u8; 32]>,
        verifier: &PlatformVersion,
    ) -> Verified {
        <Identity as FromProof<GetIdentityByNonUniquePublicKeyHashRequest>>::maybe_from_proof_with_metadata(
            request(envelope, hash, after, true), response, Network::Testnet, verifier, &provider(),
        ).map_err(Box::new)
    }

    /// The proof that carries the quorum signature: a V0 answer's index proof, or the one
    /// proof of a V1 answer.
    fn signed_proof(response: &mut GetIdentityByNonUniquePublicKeyHashResponse) -> &mut Proof {
        match &mut response.version {
            Some(Version::V0(v0)) => match &mut v0.result {
                Some(V0Result::Proof(proved)) => proved
                    .grovedb_identity_public_key_hash_proof
                    .as_mut()
                    .expect("index proof"),
                _ => panic!("proved V0 answer"),
            },
            Some(Version::V1(v1)) => match &mut v1.result {
                Some(V1Result::Proof(proof)) => proof,
                _ => panic!("proved V1 answer"),
            },
            None => panic!("versioned answer"),
        }
    }

    fn metadata(
        response: &mut GetIdentityByNonUniquePublicKeyHashResponse,
    ) -> &mut ResponseMetadata {
        match &mut response.version {
            Some(Version::V0(v0)) => v0.metadata.as_mut(),
            Some(Version::V1(v1)) => v1.metadata.as_mut(),
            None => None,
        }
        .expect("answer metadata")
    }

    fn two_proofs(
        response: &mut GetIdentityByNonUniquePublicKeyHashResponse,
    ) -> &mut IdentityProvedResponse {
        let Some(Version::V0(v0)) = &mut response.version else {
            panic!("V0 answer")
        };
        let Some(V0Result::Proof(proved)) = &mut v0.result else {
            panic!("proved V0 answer")
        };
        proved
    }

    /// Changes one bit in the middle of the first occurrence of `needle`. The proof keeps
    /// its structure, so only the authentication of those bytes can refuse it.
    fn flip_inside(bytes: &mut [u8], needle: &[u8], context: &str) {
        let at = bytes
            .windows(needle.len())
            .position(|window| window == needle)
            .unwrap_or_else(|| panic!("{context}: the proof holds the bytes to change"));
        bytes[at + needle.len() / 2] ^= 1;
    }

    /// The reason a refused answer must fail for.
    #[derive(Clone, Copy, Debug)]
    enum Refusal {
        /// The quorum signature does not cover the root and metadata the client rebuilt.
        Signature,
        /// The signature bytes do not decode as a signature.
        MalformedSignature,
        /// The answer is in the shape of the other request version.
        OtherVersion,
        /// The signed metadata names a protocol version no table describes.
        UnknownVersion,
        /// A Drive check refuses the proof; its message contains the text.
        Drive(&'static str),
        /// GroveDB refuses the proof for the rebuilt query; its message contains the text.
        Grove(&'static str),
    }

    #[track_caller]
    fn assert_refused(result: Verified, refusal: Refusal, context: &str) {
        let error = match result {
            Ok((identity, ..)) => panic!(
                "{context}: accepted with identity {:?}",
                identity.map(|identity| identity.id())
            ),
            Err(error) => error,
        };
        let refused_as_expected = match (&*error, refusal) {
            (Error::InvalidSignature { .. }, Refusal::Signature) => true,
            (Error::SignatureVerificationError { error }, Refusal::MalformedSignature) => {
                error.contains("Could not verify signature digest")
            }
            (Error::ResponseDecodeError { error }, Refusal::OtherVersion) => {
                error.contains("does not match request version")
            }
            (Error::ResponseDecodeError { error }, Refusal::UnknownVersion) => {
                error.contains("unsupported proof query protocol version")
            }
            (Error::DriveError { error }, Refusal::Drive(text)) => error.contains(text),
            (Error::GroveDBError { error, .. }, Refusal::Grove(text)) => error.contains(text),
            _ => false,
        };
        assert!(
            refused_as_expected,
            "{context}: expected {refusal:?}, got {error:?}"
        );
    }

    /// V0 answers made at every protocol version and V1 answers made where V1 is served
    /// verify for every lookup, with the verifier of an SDK at protocol 13 and at the
    /// latest version. The returned proof is the one carrying the signature.
    #[tokio::test]
    async fn should_verify_signed_answers_of_both_versions_for_every_lookup() {
        for envelope in [Envelope::V0, Envelope::V1] {
            for version in envelope.served_versions() {
                let fixture = Fixture::new(version);
                for case in cases(&fixture) {
                    let mut answer =
                        signed_capture(&fixture, envelope, case.hash, case.after).await;
                    let sent_proof = signed_proof(&mut answer).clone();
                    let sent_metadata = metadata(&mut answer).clone();
                    assert_eq!(sent_metadata.protocol_version, version.protocol_version);
                    for verifier in [
                        PlatformVersion::get(13).expect("PV13"),
                        PlatformVersion::latest(),
                    ] {
                        let context = format!(
                            "{envelope:?} PV{} {}, verifier PV{}",
                            version.protocol_version, case.name, verifier.protocol_version
                        );
                        let (identity, returned_metadata, returned_proof) =
                            verify(envelope, answer.clone(), case.hash, case.after, verifier)
                                .unwrap_or_else(|error| panic!("{context}: {error}"));
                        assert_eq!(identity.as_ref(), case.expected, "{context}");
                        assert_eq!(returned_metadata, sent_metadata, "{context}");
                        assert_eq!(returned_proof, sent_proof, "{context}");
                        assert_eq!(returned_proof.signature.len(), 96, "{context}");
                    }
                }
            }
        }
    }

    /// The request version decides the shape a client verifies, and the signature does not
    /// cover the envelope, so an answer in the other version's shape is refused rather than
    /// read as the shape it claims.
    #[tokio::test]
    async fn should_refuse_an_answer_in_the_other_request_version() {
        let fixture = Fixture::new(PlatformVersion::latest());
        for case in cases(&fixture) {
            let v0 = signed_capture(&fixture, Envelope::V0, case.hash, case.after).await;
            let v1 = signed_capture(&fixture, Envelope::V1, case.hash, case.after).await;
            for (envelope, answer) in [(Envelope::V0, &v0), (Envelope::V1, &v1)] {
                let (identity, ..) = verify(
                    envelope,
                    answer.clone(),
                    case.hash,
                    case.after,
                    fixture.version,
                )
                .unwrap_or_else(|error| panic!("{envelope:?} {} control: {error}", case.name));
                assert_eq!(identity.as_ref(), case.expected);
            }
            for (requested, answer) in [(Envelope::V1, v0), (Envelope::V0, v1)] {
                assert_refused(
                    verify(requested, answer, case.hash, case.after, fixture.version),
                    Refusal::OtherVersion,
                    &format!("{requested:?} request, {}", case.name),
                );
            }
        }
    }

    #[tokio::test]
    async fn should_reject_unbound_roots_and_mutated_signed_metadata() {
        let latest = PlatformVersion::latest();
        for (envelope, version) in [
            (Envelope::V0, PlatformVersion::get(13).expect("PV13")),
            (Envelope::V0, latest),
            (Envelope::V1, latest),
        ] {
            let fixture = Fixture::new(version);
            let context = format!("{envelope:?} PV{}", version.protocol_version);
            let wire = capture(&fixture, envelope, fixture.hash, None, true)
                .await
                .expect("served");
            let valid = signed(wire.clone(), fixture.root());
            let (identity, ..) = verify(envelope, valid.clone(), fixture.hash, None, latest)
                .unwrap_or_else(|error| panic!("{context}: untampered control: {error}"));
            assert_eq!(identity.as_ref(), Some(&fixture.identities[0]));
            let mut wrong_root = fixture.root();
            wrong_root[0] ^= 1;
            assert_refused(
                verify(
                    envelope,
                    signed(wire, wrong_root),
                    fixture.hash,
                    None,
                    latest,
                ),
                Refusal::Signature,
                &format!("{context}: signed over another root"),
            );
            for field in 0..9 {
                let mut invalid = valid.clone();
                let refusal = if field == 8 {
                    // This changed byte leaves no valid curve point, so it fails decoding.
                    Refusal::MalformedSignature
                } else {
                    Refusal::Signature
                };
                let metadata = metadata(&mut invalid);
                match field {
                    // Another known version that builds the same queries, so only the
                    // signature refuses it.
                    0 => {
                        metadata.protocol_version = if version.protocol_version == 13 {
                            12
                        } else {
                            13
                        }
                    }
                    1 => metadata.height += 1,
                    2 => metadata.time_ms += 1,
                    3 => metadata.core_chain_locked_height += 1,
                    4 => metadata.chain_id.push('x'),
                    5 => signed_proof(&mut invalid).round += 1,
                    6 => signed_proof(&mut invalid).block_id_hash[0] ^= 1,
                    7 => signed_proof(&mut invalid).quorum_hash[0] ^= 1,
                    _ => signed_proof(&mut invalid).signature[0] ^= 1,
                }
                assert_refused(
                    verify(envelope, invalid, fixture.hash, None, latest),
                    refusal,
                    &format!("{context}: signed field {field} changed"),
                );
            }
            // Version 0 names no table. Newer and reserved mock versions are read with the
            // newest rules, never a mock table, and stay bound by the signature.
            for (protocol_version, refusal) in [
                (0, Refusal::UnknownVersion),
                (latest.protocol_version + 1, Refusal::Signature),
                (0x1000_0000, Refusal::Signature),
                (0x1000_0001, Refusal::Signature),
            ] {
                let mut invalid = valid.clone();
                metadata(&mut invalid).protocol_version = protocol_version;
                assert_refused(
                    verify(envelope, invalid, fixture.hash, None, latest),
                    refusal,
                    &format!("{context}: signed version {protocol_version}"),
                );
            }
        }
    }

    /// Changing, dropping or substituting either proof of a V0 answer cannot make it select
    /// another identity, skip one, or carry an identity its index proof does not name.
    #[tokio::test]
    async fn should_reject_two_proof_answers_with_changed_dropped_or_swapped_proofs() {
        for version in versions() {
            let fixture = Fixture::new(version);
            let (first, second) = (fixture.id(0), fixture.id(1));
            let wire = capture(&fixture, Envelope::V0, fixture.hash, None, true)
                .await
                .expect("V0 served");
            let metadata = wire
                .v0
                .as_ref()
                .and_then(|v0| v0.metadata.clone())
                .expect("metadata");
            let control = signed(wire, fixture.root());
            let check = |answer, refusal, case: &str| {
                assert_refused(
                    verify(Envelope::V0, answer, fixture.hash, None, version),
                    refusal,
                    &format!("PV{} {case}", version.protocol_version),
                )
            };
            let (identity, ..) = verify(Envelope::V0, control.clone(), fixture.hash, None, version)
                .expect("untampered control");
            assert_eq!(identity.as_ref(), Some(&fixture.identities[0]));

            let index_proof = |after, limit| {
                let mut query =
                    Drive::identity_id_by_non_unique_public_key_hash_query(fixture.hash, after);
                query.query.limit = limit;
                fixture.prove(&query)
            };
            let identity_proof = |id: &[u8; 32]| {
                fixture.prove(
                    &Drive::full_identity_query(id, &version.drive.grove_version)
                        .expect("full identity query"),
                )
            };

            let mut changed_index = control.clone();
            flip_inside(
                &mut signed_proof(&mut changed_index).grovedb_proof,
                &first,
                "index proof",
            );
            // Each GroveDB proof layer is checked against the hash its parent layer commits
            // to, so changed bytes are refused inside the proof that holds them.
            check(
                changed_index,
                Refusal::Grove("mismatch in lower layer hash"),
                "changed identity ID in the index proof",
            );

            let first_key_data = fixture.identities[0].public_keys()[&0]
                .data()
                .as_slice()
                .to_vec();
            let mut changed_identity = control.clone();
            flip_inside(
                two_proofs(&mut changed_identity)
                    .identity_proof_bytes
                    .as_mut()
                    .expect("identity proof"),
                &first_key_data,
                "identity proof",
            );
            check(
                changed_identity,
                Refusal::Grove("mismatch in lower layer hash"),
                "changed key data in the identity proof",
            );

            let mut dropped = control.clone();
            two_proofs(&mut dropped).identity_proof_bytes = None;
            check(
                dropped,
                Refusal::Drive("references an identity without its proof"),
                "identity proof dropped",
            );

            let mut swapped = control.clone();
            two_proofs(&mut swapped).identity_proof_bytes = Some(identity_proof(&second));
            check(
                swapped,
                Refusal::Grove("Cannot verify lower bound of queried range"),
                "another identity's proof",
            );

            // An index proof of the same query at another state.
            let other = Fixture::new(version);
            other
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
            let mut unbound_index =
                Drive::identity_id_by_non_unique_public_key_hash_query(fixture.hash, None);
            unbound_index.query.limit = Some(1);
            let unbound_index = other.prove(&unbound_index);

            for (case, index, identity, refusal) in [
                (
                    "index proof skips the first identity",
                    index_proof(Some(first), Some(1)),
                    Some(identity_proof(&second)),
                    Refusal::Grove("Proof is missing data for query"),
                ),
                (
                    "index proof of two entries",
                    index_proof(None, Some(2)),
                    Some(identity_proof(&first)),
                    Refusal::Grove("more data than limit"),
                ),
                (
                    "identity proof in place of the index proof",
                    identity_proof(&first),
                    Some(identity_proof(&first)),
                    Refusal::Grove("Cannot verify lower bound of queried range"),
                ),
                (
                    "index proof of another state",
                    unbound_index,
                    Some(identity_proof(&first)),
                    Refusal::Drive("index and identity proof roots differ"),
                ),
            ] {
                check(
                    signed(
                        two_proof_answer(index, identity, metadata.clone()),
                        fixture.root(),
                    ),
                    refusal,
                    case,
                );
            }

            // An absence answer that also carries an identity proof.
            let after = Some(fixture.id(2));
            let mut absence = signed_capture(&fixture, Envelope::V0, fixture.hash, after).await;
            let (identity, ..) =
                verify(Envelope::V0, absence.clone(), fixture.hash, after, version)
                    .expect("untampered absence");
            assert_eq!(identity, None);
            two_proofs(&mut absence).identity_proof_bytes = Some(identity_proof(&first));
            assert_refused(
                verify(Envelope::V0, absence, fixture.hash, after, version),
                Refusal::Drive("must bind its first eligible identity or prove absence"),
                &format!(
                    "PV{} absence with an identity proof",
                    version.protocol_version
                ),
            );
        }
    }

    /// A V1 answer with changed identity bytes, or one that proves a different selection
    /// or an incomplete identity at the authentic root, is refused.
    #[tokio::test]
    async fn should_reject_signed_skipped_incomplete_and_unrelated_single_proofs() {
        for version in versions() {
            let fixture = Fixture::new(version);
            let context = |case: &str| format!("PV{} {case}", version.protocol_version);
            let first = fixture.id(0);
            let later = fixture.id(1);
            let metadata = fixed_metadata(version);
            let control = signed(
                single_proof_answer(
                    fixture
                        .platform
                        .drive
                        .prove_full_identity_by_non_unique_public_key_hash(
                            fixture.hash,
                            None,
                            None,
                            version,
                        )
                        .expect("single proof"),
                    metadata.clone(),
                ),
                fixture.root(),
            );
            let (identity, ..) = verify(Envelope::V1, control.clone(), fixture.hash, None, version)
                .expect("untampered control");
            assert_eq!(identity.as_ref(), Some(&fixture.identities[0]));
            let mut changed = control.clone();
            flip_inside(
                &mut signed_proof(&mut changed).grovedb_proof,
                fixture.identities[0].public_keys()[&0].data().as_slice(),
                "single proof",
            );
            // GroveDB checks each proof layer against the hash its parent layer commits to.
            assert_refused(
                verify(Envelope::V1, changed, fixture.hash, None, version),
                Refusal::Grove("mismatch in lower layer hash"),
                &context("changed key data"),
            );

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
            let (scalar_root, scalar_rows) =
                GroveDb::verify_query(&scalar_blob, &original, &version.drive.grove_version)
                    .expect("strict query result for scalar diagnostic");
            assert_eq!(scalar_root, fixture.root());
            assert!(!scalar_rows
                .iter()
                .any(|(path, key, _)| path.is_empty() && key == &vec![0xff]));
            // Non-queried Merk authentication nodes are not additional query results.
            // They cannot change the selected identity or bypass the signed root check.
            let (identity, returned_metadata, returned_proof) = verify(
                Envelope::V1,
                signed(
                    single_proof_answer(scalar_blob.clone(), metadata.clone()),
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
            for (case, query, refusal) in [
                (
                    "later prefix",
                    bounded_query(&fixture, later, None),
                    Refusal::Grove("more data than limit"),
                ),
                (
                    "later membership",
                    membership,
                    Refusal::Grove("Cannot verify lower bound of queried range"),
                ),
                (
                    "identity only",
                    Drive::full_identity_query(&first, &version.drive.grove_version)
                        .expect("full first identity"),
                    Refusal::Grove("Cannot verify lower bound of queried range"),
                ),
                ("index only", index, Refusal::Grove("more data than limit")),
                (
                    "extra identity",
                    mismatched,
                    Refusal::Grove("extra lower layer"),
                ),
            ] {
                let blob = fixture
                    .platform
                    .drive
                    .grove
                    .prove_query(&query, None, &version.drive.grove_version)
                    .unwrap()
                    .expect("malicious request-shaped proof at authentic root");
                assert_refused(
                    verify(
                        Envelope::V1,
                        signed(single_proof_answer(blob, metadata.clone()), fixture.root()),
                        fixture.hash,
                        None,
                        version,
                    ),
                    refusal,
                    &context(case),
                );
            }
        }
    }

    /// Malformed committed index entries, an index entry without its identity, and
    /// truncated or unsupported envelopes are refused by both Drive verifiers.
    #[test]
    fn should_reject_malformed_index_values_keys_and_missing_identity() {
        for version in versions() {
            let pv = version.protocol_version;
            for case in 0..2 {
                let fixture = Fixture::new(version);
                let first = fixture.id(0);
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
                let mut first_entry = index.clone();
                first_entry.query.limit = Some(1);
                index.query.query.set_subquery(Query::new_single_query_item(
                    QueryItem::RangeToInclusive(..=key),
                ));
                let full = Drive::full_identity_query(&first, &version.drive.grove_version)
                    .expect("unrelated first identity");
                let query = PathQuery::merge(vec![&index, &full], &version.drive.grove_version)
                    .expect("proof of malformed index entry");
                let single = Drive::verify_full_identity_by_non_unique_public_key_hash(
                    &fixture.prove(&query),
                    fixture.hash,
                    None,
                    version,
                    version,
                )
                .expect_err("malformed index cannot produce a valid lookup");
                let two = Drive::verify_full_identity_by_non_unique_public_key_hash_two_proofs(
                    &fixture.prove(&first_entry),
                    Some(&fixture.prove(&full)),
                    fixture.hash,
                    None,
                    version,
                    version,
                )
                .expect_err("malformed index cannot produce a valid lookup");
                match case {
                    0 => {
                        assert!(
                            matches!(&single, DriveError::Proof(ProofError::CorruptedProof(message)) if message.contains("zero or one empty item")),
                            "PV{pv} non-empty index value, single proof: {single:?}"
                        );
                        assert!(
                            matches!(&two, DriveError::Proof(ProofError::CorruptedProof(message)) if message.contains("must bind its first eligible identity")),
                            "PV{pv} non-empty index value, two proofs: {two:?}"
                        );
                    }
                    _ => {
                        for (shape, error) in [("single proof", &single), ("two proofs", &two)] {
                            assert!(
                                matches!(
                                    error,
                                    DriveError::Proof(ProofError::IncorrectValueSize(
                                        "non-unique index ID must be 32 bytes"
                                    ))
                                ),
                                "PV{pv} 31-byte index key, {shape}: {error:?}"
                            );
                        }
                    }
                }
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
            let two = fixture
                .platform
                .drive
                .prove_full_identity_by_non_unique_public_key_hash_two_proofs(
                    fixture.hash,
                    None,
                    None,
                    version,
                )
                .expect("two-proof producer proves the indexed identity absence");
            for (shape, error) in [
                (
                    "single proof",
                    Drive::verify_full_identity_by_non_unique_public_key_hash(
                        &blob,
                        fixture.hash,
                        None,
                        version,
                        version,
                    )
                    .expect_err("a dangling index is not an absent hash lookup"),
                ),
                (
                    "two proofs",
                    Drive::verify_full_identity_by_non_unique_public_key_hash_two_proofs(
                        &two.identity_id_public_key_hash_proof,
                        two.identity_proof.as_deref(),
                        fixture.hash,
                        None,
                        version,
                        version,
                    )
                    .expect_err("a dangling index is not an absent hash lookup"),
                ),
            ] {
                assert!(
                    matches!(
                        error,
                        DriveError::Proof(ProofError::IncompleteProof(
                            "non-unique index references a missing full identity"
                        ))
                    ),
                    "PV{pv} dangling indexed identity, {shape}: {error:?}"
                );
            }

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
            let pair = fixture
                .platform
                .drive
                .prove_full_identity_by_non_unique_public_key_hash_two_proofs(
                    fixture.hash,
                    None,
                    None,
                    version,
                )
                .expect("two proofs");
            let half = |bytes: &[u8]| bytes[..bytes.len() / 2].to_vec();
            for (case, single, index) in [
                ("empty", vec![], vec![]),
                (
                    "truncated",
                    half(&wire),
                    half(&pair.identity_id_public_key_hash_proof),
                ),
                ("envelope 0", vec![0], vec![0]),
                ("envelope 1 without a proof", vec![1], vec![1]),
            ] {
                let single = Drive::verify_full_identity_by_non_unique_public_key_hash(
                    &single,
                    fixture.hash,
                    None,
                    version,
                    version,
                )
                .expect_err("a broken envelope cannot become absence");
                let two = Drive::verify_full_identity_by_non_unique_public_key_hash_two_proofs(
                    &index,
                    pair.identity_proof.as_deref(),
                    fixture.hash,
                    None,
                    version,
                    version,
                )
                .expect_err("a broken envelope cannot become absence");
                for (shape, error) in [("single proof", single), ("two proofs", two)] {
                    let refused_as_expected = match case {
                        "empty" => matches!(
                            error,
                            DriveError::Proof(ProofError::InvalidGroveDBProofEnvelope { .. })
                        ),
                        "envelope 0" => matches!(
                            error,
                            DriveError::Proof(ProofError::UnsupportedGroveDBProofEnvelopeVersion {
                                version: 0,
                                ..
                            })
                        ),
                        // The envelope version is supported, but GroveDB cannot decode the
                        // proof behind it.
                        _ => matches!(
                            &error,
                            DriveError::GroveDB(grove) if matches!(
                                grove.as_ref(),
                                GroveError::CorruptedData(message)
                                    if message.starts_with("unable to decode proof")
                            )
                        ),
                    };
                    assert!(
                        refused_as_expected,
                        "PV{pv} {case} envelope, {shape}: {error:?}"
                    );
                }
            }
        }
    }

    /// Writes the vectors of `rs-sdk`'s `test_fetch_identity_by_non_unique_public_keys`
    /// and of wasm-drive-verify, to `NONUNIQUE_VECTOR_OUTPUT` or a scratch directory.
    ///
    /// The SDK pages through three identities. A fresh SDK has verified no answer yet, so it
    /// asks for the first page in V0; once it has verified that protocol 14 answer it asks
    /// for the rest in V1. wasm-drive-verify reads single proofs only, so its vectors take
    /// every page from a V1 answer, and they must equal the committed fixture. The vectors
    /// are made at protocol 14, the first version that serves V1, so that later branches
    /// rewrite the same files.
    #[tokio::test]
    async fn should_export_deterministic_sdk_vectors() {
        let temporary = TempDir::new().expect("scratch export");
        let destination = std::env::var_os("NONUNIQUE_VECTOR_OUTPUT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| temporary.path().to_path_buf());
        std::fs::create_dir_all(&destination).expect("scratch vector directory");
        let fixture = Fixture::new(PlatformVersion::get(14).expect("protocol 14"));
        let mut after = None;
        let mut direct_vectors = Vec::new();
        for page in 0..4 {
            let envelope = if page == 0 {
                Envelope::V0
            } else {
                Envelope::V1
            };
            let answer = signed_capture(&fixture, envelope, fixture.hash, after).await;
            let (identity, _, _) = verify(
                envelope,
                answer.clone(),
                fixture.hash,
                after,
                fixture.version,
            )
            .expect("verify vector before export");
            let (single_identity, _, single_proof) = verify(
                Envelope::V1,
                signed_capture(&fixture, Envelope::V1, fixture.hash, after).await,
                fixture.hash,
                after,
                fixture.version,
            )
            .expect("verify single proof before export");
            assert_eq!(single_identity, identity, "page {page}");
            direct_vectors.push(serde_json::json!({
                "proof": hex::encode(single_proof.grovedb_proof),
                "root": hex::encode(fixture.root()), "hash": hex::encode(fixture.hash),
                "after": after.map(hex::encode), "identity_id": identity.as_ref().map(|id| hex::encode(id.id().to_buffer())),
                "balance": identity.as_ref().map(|id| id.balance()), "revision": identity.as_ref().map(|id| id.revision()),
                "key_count": identity.as_ref().map(|id| id.public_keys().len()),
            }));
            let req = serde_json::json!({"GetIdentityByNonUniquePublicKeyHash": request(envelope, fixture.hash, after, true)});
            let resp = serde_json::json!({"GetIdentityByNonUniquePublicKeyHash": answer});
            let request_bytes = serde_json::to_vec_pretty(&req).expect("request JSON");
            let response_bytes = serde_json::to_vec_pretty(&resp).expect("response JSON");
            let mut bytes = request_bytes;
            bytes.extend_from_slice(b"\n\0\n");
            bytes.extend(
                serde_json::to_vec(&serde_json::json!({"Ok": response_bytes}))
                    .expect("mock result"),
            );
            std::fs::write(
                destination.join(format!("msg_IdentityRequest_{page}.json")),
                bytes,
            )
            .expect("write scratch vector");
            after = identity.map(|identity| identity.id().to_buffer());
        }
        assert_eq!(after, None, "the last page finds no identity");
        std::fs::write(
            destination.join(format!("quorum_pubkey-106-{}.json", hex::encode([2; 32]))),
            hex::encode(provider().0),
        )
        .expect("quorum key vector");
        let direct = serde_json::to_vec_pretty(&direct_vectors).expect("direct vectors");
        std::fs::write(destination.join("nonunique-fixtures.json"), &direct)
            .expect("write direct WASM vectors");
        let committed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../wasm-drive-verify/tests/fixtures/nonunique_single_proof.json");
        // Compare parsed JSON so formatting, such as a final newline, does not matter.
        let committed_vectors: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&committed).expect("committed WASM vectors"))
                .expect("committed WASM vectors are JSON");
        assert!(
            committed_vectors == serde_json::Value::from(direct_vectors),
            "single proofs changed: replace {} with nonunique-fixtures.json from {}",
            committed.display(),
            destination.display()
        );
    }
}
