//! Protobuf refusal must leave real Drive state and successful data/proof queries intact.

use bytes::{BufMut, Bytes};
use dapi_grpc::platform::v0::get_data_contracts_request::GetDataContractsRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_response::{
    get_data_contracts_response_v0::Result as ContractsResult, Version,
};
use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::platform::v0::platform_server::PlatformServer;
use dapi_grpc::platform::v0::{GetDataContractsRequest, GetDataContractsResponse};
use dapi_grpc::tonic::client::Grpc;
use dapi_grpc::tonic::codec::{BufferSettings, Codec, EncodeBuf, Encoder};
use dapi_grpc::tonic::codegen::http::{uri::PathAndQuery, Uri};
use dapi_grpc::tonic::transport::{Channel, Endpoint, Server};
use dapi_grpc::tonic::{Code, Request, Response, Status};
use dapi_grpc::tonic_prost::{ProstCodec, ProstDecoder};
use dapi_grpc::Message;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::platform_value::Identifier;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive_abci::config::PlatformConfig;
use drive_abci::platform_types::platform::Platform;
use drive_abci::query::QueryService;
use drive_abci::rpc::core::DefaultCoreRPC;
use drive_abci::test::helpers::setup::TestPlatformBuilder;
use hyper_util::rt::TokioIo;
use rs_dapi_client::CanRetry;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;

struct Node {
    platform: Arc<Platform<DefaultCoreRPC>>,
    contract: DataContract,
    channel: Channel,
    server: JoinHandle<()>,
    _directory: TempDir,
}

impl Drop for Node {
    fn drop(&mut self) {
        self.server.abort();
    }
}

impl Node {
    async fn new() -> Self {
        let version = PlatformVersion::latest();
        let mut config = PlatformConfig::default_for_network(Network::Testnet);
        // The fixture never calls Core.
        config.core.consensus_rpc.host = "http://127.0.0.1".to_owned();
        config.core.consensus_rpc.port = 1;
        let temporary = TestPlatformBuilder::new()
            .with_config(config)
            .build_with_default_rpc();
        let mut state = temporary.platform.state.load_full().as_ref().clone();
        state.current_protocol_version_in_consensus = version.protocol_version;
        temporary.platform.state.store(Arc::new(state));
        temporary
            .platform
            .drive
            .create_initial_state_structure(None, version)
            .expect("initial Drive state");
        let contract =
            get_data_contract_fixture(Some(Identifier::new([7; 32])), 0, version.protocol_version)
                .data_contract_owned();
        temporary
            .platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo {
                    time_ms: 1000,
                    ..Default::default()
                },
                true,
                None,
                None,
                version,
            )
            .expect("store deterministic contract");
        let platform = Arc::new(temporary.platform);
        let service = QueryService::new(Arc::clone(&platform));
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(async move {
            let incoming = tokio_stream::iter([Ok::<_, std::io::Error>(server_io)])
                .chain(tokio_stream::pending());
            Server::builder()
                .add_service(PlatformServer::new(service))
                .serve_with_incoming(incoming)
                .await
                .expect("serve actual Drive query service");
        });
        let mut io = Some(client_io);
        let channel = Endpoint::from_static("http://drive.in-memory")
            .timeout(Duration::from_secs(5))
            .connect_with_connector_lazy(tower::service_fn(move |_: Uri| {
                let io = io.take();
                async move {
                    io.map(TokioIo::new)
                        .ok_or_else(|| std::io::Error::other("single-use query channel"))
                }
            }));
        Self {
            platform,
            contract,
            channel,
            server,
            _directory: temporary.tempdir,
        }
    }

    fn root(&self) -> [u8; 32] {
        self.platform
            .drive
            .grove
            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
            .value
            .expect("stored root")
    }

    async fn raw_request(
        &self,
        payload: &[u8],
    ) -> Result<Response<GetDataContractsResponse>, Status> {
        let mut client = Grpc::new(self.channel.clone());
        client.ready().await.expect("ready query channel");
        client
            .unary(
                Request::new(Bytes::copy_from_slice(payload)),
                PathAndQuery::from_static("/org.dash.platform.dapi.v0.Platform/getDataContracts"),
                PayloadCodec::<GetDataContractsResponse>(PhantomData),
            )
            .await
    }

    async fn assert_original_query(&self) {
        let mut ids = [self.contract.id().to_buffer(), [1; 32]];
        ids.sort();
        assert_ne!(ids[0], ids[1]);
        let root = self.root();
        let mut client = PlatformClient::new(self.channel.clone());
        for prove in [false, true] {
            let response = client
                .get_data_contracts(GetDataContractsRequest::from(GetDataContractsRequestV0 {
                    ids: ids.iter().map(|id| id.to_vec()).collect(),
                    prove,
                }))
                .await
                .expect("valid original query")
                .into_inner();
            let Some(Version::V0(response)) = response.version else {
                panic!("expected V0 response");
            };
            if prove {
                let Some(ContractsResult::Proof(proof)) = response.result else {
                    panic!("expected original-query proof");
                };
                let (proved_root, contracts) = Drive::verify_contracts(
                    &proof.grovedb_proof,
                    false,
                    &ids,
                    PlatformVersion::latest(),
                )
                .expect("verify original query");
                assert_eq!(
                    proved_root, root,
                    "wire proof retains the exact stored root"
                );
                assert_eq!(contracts.len(), 2);
                assert_eq!(
                    contracts.get(&self.contract.id().to_buffer()),
                    Some(&Some(self.contract.clone()))
                );
                assert_eq!(contracts.get(&[1; 32]), Some(&None));
            } else {
                let Some(ContractsResult::DataContracts(contracts)) = response.result else {
                    panic!("expected raw contract entries");
                };
                assert_eq!(contracts.data_contract_entries.len(), 2);
                let stored = contracts
                    .data_contract_entries
                    .iter()
                    .find(|entry| entry.identifier == self.contract.id().to_buffer())
                    .expect("present entry");
                assert_eq!(
                    stored.data_contract.as_ref().expect("stored serialization"),
                    &self
                        .contract
                        .serialize_to_bytes_with_platform_version(PlatformVersion::latest())
                        .expect("canonical contract serialization"),
                    "valid raw response preserves the exact fixture contract",
                );
                let absent = contracts
                    .data_contract_entries
                    .iter()
                    .find(|entry| entry.identifier == [1; 32])
                    .expect("absence entry");
                assert!(absent.data_contract.is_none());
            }
            assert_eq!(self.root(), root, "query leaves state unchanged");
        }
    }
}

struct PayloadCodec<T>(PhantomData<T>);
struct PayloadEncoder;

impl Encoder for PayloadEncoder {
    type Item = Bytes;
    type Error = Status;

    fn encode(&mut self, bytes: Bytes, buffer: &mut EncodeBuf<'_>) -> Result<(), Status> {
        buffer.put_slice(&bytes);
        Ok(())
    }
}

impl<T: Message + Default + Send + 'static> Codec for PayloadCodec<T> {
    type Encode = Bytes;
    type Decode = T;
    type Encoder = PayloadEncoder;
    type Decoder = ProstDecoder<T>;

    fn encoder(&mut self) -> PayloadEncoder {
        PayloadEncoder
    }
    fn decoder(&mut self) -> ProstDecoder<T> {
        ProstCodec::<T, T>::raw_decoder(BufferSettings::default())
    }
}

#[tokio::test]
async fn should_refuse_malformed_protobuf_without_changing_drive_state_or_query_proofs() {
    let node = Node::new().await;
    let root = node.root();
    let mut wrong = Vec::new();
    for payload in [&[0][..], &[0x80], &[0x0a, 0x02, 0x08], &[0x08, 0x01]] {
        let status = node
            .raw_request(payload)
            .await
            .expect_err("malformed caller request");
        assert!(status
            .message()
            .starts_with("failed to decode Protobuf message: "));
        if status.code() != Code::InvalidArgument {
            wrong.push(format!(
                "{payload:02x?}: {:?}: {}",
                status.code(),
                status.message()
            ));
        }
        assert_eq!(
            node.root(),
            root,
            "an undecodable request never changes state"
        );
    }
    node.assert_original_query().await;
    assert!(
        wrong.is_empty(),
        "Drive caller protobuf statuses: {wrong:#?}"
    );
}

#[tokio::test]
async fn should_preserve_valid_data_proofs_unknown_fields_and_missing_versions() {
    let node = Node::new().await;
    node.assert_original_query().await;
    let request = GetDataContractsRequest::from(GetDataContractsRequestV0 {
        ids: vec![vec![1; 32]],
        prove: false,
    });
    let mut bytes = request.encode_to_vec();
    bytes.extend_from_slice(&[0xf8, 0x07, 0x01]);
    let with_unknown = node
        .raw_request(&bytes)
        .await
        .expect("valid unknown field")
        .into_inner();
    let ordinary = PlatformClient::new(node.channel.clone())
        .get_data_contracts(request)
        .await
        .expect("ordinary query")
        .into_inner();
    assert_eq!(
        with_unknown, ordinary,
        "unknown fields preserve query semantics"
    );
    let missing = node
        .raw_request(&[])
        .await
        .expect_err("valid message with no version");
    assert_eq!(missing.code(), Code::Unknown);
    assert_eq!(
        missing.message(),
        "decoding error: could not decode data contracts query"
    );
    assert!(
        missing.can_retry(),
        "a newer node may support this request version"
    );
    let unknown_version = node
        .raw_request(&[0x12, 0])
        .await
        .expect_err("unrecognized version field");
    assert_eq!(unknown_version.code(), missing.code());
    assert_eq!(unknown_version.message(), missing.message());
    assert!(unknown_version.can_retry());
}
