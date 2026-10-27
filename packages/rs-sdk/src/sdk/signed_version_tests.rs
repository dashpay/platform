use super::{verifier_version, Sdk, SdkBuilder, SdkInstance};
use crate::error::StaleNodeError;
use crate::mock::{MockDashPlatformSdk, MockResponse};
use crate::platform::trunk_branch_sync::{
    run_full_tree_scan, BranchQueryParams, KeyLeafTracker, TrunkBranchSyncOps, TrunkQueryResponse,
};
use crate::platform::DocumentQuery;
use crate::sync::retry;
use crate::Error;
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v0::Result as DocumentsResult, GetDocumentsResponseV0,
    Version as DocumentsResponseVersion,
};
use dapi_grpc::platform::v0::get_epochs_info_response::Version as EpochResponseVersion;
use dapi_grpc::platform::v0::wait_for_state_transition_result_response::Version as WaitResponseVersion;
use dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, GetDataContractRequest, GetDocumentsResponse,
    GetEpochsInfoRequest, GetEpochsInfoResponse, Proof, ResponseMetadata,
    WaitForStateTransitionResultResponse,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dapi_grpc::Message;
use dash_context_provider::{ContextProvider, ContextProviderError, QuorumKeyFuture};
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted;
use dpp::state_transition::proof_result::StateTransitionProofOutcome;
use drive::grovedb::{GroveBranchQueryResult, GroveTrunkQueryResult, LeafInfo, MerkProofNode};
use drive_proof_verifier::types::Documents;
use drive_proof_verifier::{Error as ProofError, FromProof};
use platform_version::version::PlatformVersion;
use rs_dapi_client::{
    Address, AddressList, CanRetry, DumpData, ExecutionError, ExecutionResponse, IntoInner,
    RequestSettings,
};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// A verifier double that exposes the table supplied by the SDK without
/// interpreting proof bytes. Signature acceptance is tested separately using
/// recorded, signed responses.
#[derive(Debug)]
struct ObservedTable(u32);

impl MockResponse for ObservedTable {
    fn mock_serialize(&self, _: &MockDashPlatformSdk) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
    }

    fn mock_deserialize(_: &MockDashPlatformSdk, buf: &[u8]) -> Self {
        Self(u32::from_be_bytes(buf.try_into().expect("table number")))
    }
}

impl FromProof<GetEpochsInfoRequest> for ObservedTable {
    type Request = GetEpochsInfoRequest;
    type Response = GetEpochsInfoResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _: I,
        response: O,
        _: Network,
        platform_version: &PlatformVersion,
        _: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), ProofError>
    where
        Self: Sized + 'a,
    {
        let response = response.into();
        Ok((
            Some(Self(platform_version.protocol_version)),
            response.metadata()?.clone(),
            response.proof()?.clone(),
        ))
    }
}

fn recorded_epoch_response() -> (GetEpochsInfoRequest, GetEpochsInfoResponse) {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "tests/vectors/test_epoch_fetch/msg_GetEpochsInfoRequest_b2b426ac4a52cb4cb08904c63386caf3663c40a12d3b03827006d66058e439ac.json",
    );
    let (request, response) = DumpData::<GetEpochsInfoRequest>::load(file)
        .expect("recorded epoch response")
        .deserialize();
    (
        request,
        response.expect("successful recorded response").inner,
    )
}

// Changed metadata is used only by verifier doubles and signature-rejection
// tests; a response with changed metadata is not an authenticated fixture.
fn recorded_response(signed: u32) -> (GetEpochsInfoRequest, GetEpochsInfoResponse) {
    let (request, mut response) = recorded_epoch_response();
    let Some(EpochResponseVersion::V0(v0)) = response.version.as_mut() else {
        panic!("recorded version");
    };
    v0.metadata
        .as_mut()
        .expect("recorded metadata")
        .protocol_version = signed;
    (request, response)
}

#[derive(Clone, Copy)]
enum ProofPath {
    Network,
    Mock,
}

#[derive(Clone, Copy)]
enum VersionPolicy {
    Track,
    Pin,
}

async fn observed_table(
    stored: u32,
    signed: u32,
    policy: VersionPolicy,
    path: ProofPath,
) -> (u32, u32) {
    let builder = match path {
        ProofPath::Mock => SdkBuilder::new_mock(),
        ProofPath::Network => SdkBuilder::new(AddressList::new()),
    }
    .with_network(Network::Regtest)
    .with_context_provider(NoKeys)
    .with_time_tolerance(None)
    .with_height_tolerance(Some(1))
    .with_trusted_initial_height(2);
    let version = PlatformVersion::get(stored).expect("known stored table");
    let builder = match policy {
        VersionPolicy::Pin => builder.with_version(version),
        VersionPolicy::Track => builder.with_initial_version(version),
    };
    let sdk = builder.build().expect("sdk");
    let (request, response) = recorded_response(signed);
    let (observed, _, _) = if matches!(path, ProofPath::Mock) {
        sdk.parse_proof_with_metadata_and_proof::<GetEpochsInfoRequest, ObservedTable>(
            request,
            response,
            "get_epochs_info",
        )
        .await
        .expect("mock verifier double")
    } else {
        sdk.verify_fetching_quorum_key::<GetEpochsInfoRequest, ObservedTable>(
            request,
            response,
            &NoKeys,
            "get_epochs_info",
        )
        .await
        .expect("verifier double")
    };
    (
        observed.expect("observed table").0,
        sdk.protocol_version_number(),
    )
}

struct NoKeys;

impl ContextProvider for NoKeys {
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
        Err(ContextProviderError::InvalidQuorum(
            "unused by verifier double".into(),
        ))
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

#[tokio::test]
async fn should_verify_with_the_signed_newer_version_not_the_stored_one() {
    assert_eq!(
        observed_table(13, 14, VersionPolicy::Track, ProofPath::Network).await,
        (14, 14)
    );
}

#[test_case::test_case(0x10000000; "mock_version_zero")]
#[test_case::test_case(0x10000001; "mock_version_one")]
fn should_use_newest_known_table_for_unauthenticated_mock_versions(signed: u32) {
    let stored = PlatformVersion::get(13).expect("known stored table");
    assert!(std::ptr::eq(
        verifier_version(stored, signed),
        PlatformVersion::latest()
    ));
}

#[tokio::test]
async fn should_verify_mock_responses_with_the_signed_newer_version() {
    assert_eq!(
        observed_table(13, 14, VersionPolicy::Track, ProofPath::Mock).await,
        (14, 14)
    );
}

#[tokio::test]
async fn should_never_verify_below_the_stored_version() {
    assert_eq!(
        observed_table(14, 13, VersionPolicy::Track, ProofPath::Network).await,
        (14, 14)
    );
    let latest = PlatformVersion::latest().protocol_version;
    assert_eq!(
        observed_table(latest, 0, VersionPolicy::Track, ProofPath::Network).await,
        (latest, latest)
    );
}

#[tokio::test]
async fn should_verify_with_the_newest_known_table_when_the_network_is_newer() {
    let warning = Arc::new(Mutex::new(Vec::new()));
    let captured = warning.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .with_writer(move || WarningBuffer(captured.clone()))
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    assert_eq!(
        observed_table(
            13,
            PlatformVersion::latest().protocol_version + 1,
            VersionPolicy::Track,
            ProofPath::Network
        )
        .await,
        (PlatformVersion::latest().protocol_version, 13)
    );
    let warning =
        String::from_utf8(warning.lock().expect("warnings").clone()).expect("UTF8 warning");
    assert!(
        warning.contains("using newest known verifier table"),
        "{warning}"
    );
}

struct WarningBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for WarningBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("warnings").extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn should_verify_with_the_signed_newer_version_when_pinned() {
    assert_eq!(
        observed_table(13, 14, VersionPolicy::Pin, ProofPath::Network).await,
        (14, 13)
    );
}

#[tokio::test]
async fn should_read_a_recorded_pv14_contract_first_with_an_sdk_seeded_at_13() {
    // This contract's grammar is supported by PV13. The real-vector smoke
    // test checks authentication and first-read learning; verifier doubles
    // observe the table passed to proof parsing.
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors/contested_resource_vote_states_ok");
    let (request, response) = DumpData::<GetDataContractRequest>::load(directory.join(
        "msg_GetDataContractRequest_e87a2e6acef76975c30eb7272da71733fb6ad13495beb7ca1b6a6c4ceb30e0f7.json",
    )).expect("recorded PV14 contract").deserialize();
    let sdk = SdkBuilder::new_mock()
        .with_initial_version(PlatformVersion::get(13).expect("seed table"))
        .with_dump_dir(&directory)
        .with_height_tolerance(None)
        .build()
        .expect("sdk");
    let response = response.expect("successful recorded response").inner;
    assert_eq!(response.metadata().expect("metadata").protocol_version, 14);
    let (contract, _, _) = sdk
        .parse_proof_with_metadata_and_proof::<GetDataContractRequest, DataContract>(
            request,
            response,
            "get_data_contract",
        )
        .await
        .expect("recorded first contract read succeeds");
    assert!(contract.is_some());
    assert_eq!(sdk.protocol_version_number(), 14);
}

struct ScanTable;

impl TrunkBranchSyncOps for ScanTable {
    type Context<'a> = Arc<Mutex<Vec<u32>>>;
    type BranchQueryConfig = Arc<Mutex<Vec<u32>>>;

    async fn execute_trunk_query(
        sdk: &Sdk,
        _: RequestSettings,
        _: &mut Self::Context<'_>,
    ) -> Result<TrunkQueryResponse, Error> {
        sdk.accept_verified_metadata(
            "get_addresses_trunk_state",
            &ResponseMetadata {
                protocol_version: 14,
                ..Default::default()
            },
            None,
        )?;
        Ok(TrunkQueryResponse {
            trunk: GroveTrunkQueryResult {
                elements: Default::default(),
                leaf_keys: Default::default(),
                chunk_depths: vec![],
                max_tree_depth: 1,
                tree: MerkProofNode::Hash([0; 32]).into(),
            },
            height: 1,
            block_time_ms: 1,
            protocol_version: 14,
        })
    }

    async fn process_trunk_result(
        _: &GroveTrunkQueryResult,
        _: &mut Self::Context<'_>,
        tracker: &mut KeyLeafTracker,
    ) -> Result<(), Error> {
        tracker.add_key(
            vec![1],
            vec![1],
            LeafInfo {
                hash: [0; 32],
                count: Some(1),
            },
        );
        Ok(())
    }

    fn branch_query_config(context: &Self::Context<'_>) -> Self::BranchQueryConfig {
        context.clone()
    }

    async fn execute_single_branch_query(
        _: &Sdk,
        config: &Self::BranchQueryConfig,
        _: BranchQueryParams,
        _: RequestSettings,
        platform_version: &PlatformVersion,
    ) -> Result<GroveBranchQueryResult, Error> {
        config
            .lock()
            .expect("observations")
            .push(platform_version.protocol_version);
        Ok(GroveBranchQueryResult {
            elements: Default::default(),
            leaf_keys: Default::default(),
            branch_root_hash: [0; 32],
            tree: MerkProofNode::Hash([0; 32]).into(),
        })
    }

    async fn process_branch_result(
        _: &GroveBranchQueryResult,
        _: &[u8],
        _: &mut Self::Context<'_>,
        tracker: &mut KeyLeafTracker,
    ) -> Result<(), Error> {
        tracker.key_found(&[1]);
        Ok(())
    }
    fn depth_limits(_: &PlatformVersion) -> (u8, u8) {
        (1, 2)
    }
    fn on_branch_query(_: &mut Self::Context<'_>) {}
    fn on_branch_failure(_: &mut Self::Context<'_>) {
        panic!("branch failed");
    }
    fn on_elements_seen(_: &mut Self::Context<'_>, _: usize) {}
    fn on_iteration(_: &mut Self::Context<'_>, _: usize) {}
    fn set_checkpoint_height(_: &mut Self::Context<'_>, _: u64) {}
}

#[tokio::test]
async fn should_verify_branches_with_the_newer_trunk_table() {
    for pinned in [false, true] {
        let builder = SdkBuilder::new_mock().with_height_tolerance(None);
        let version = PlatformVersion::get(13).expect("seed");
        let builder = if pinned {
            builder.with_version(version)
        } else {
            builder.with_initial_version(version)
        };
        let sdk = builder.build().expect("sdk");
        let mut observed = Arc::new(Mutex::new(vec![]));
        run_full_tree_scan::<ScanTable>(&sdk, 1, 2, 1, RequestSettings::default(), &mut observed)
            .await
            .expect("scan");
        assert_eq!(*observed.lock().expect("observations"), vec![14]);
        assert_eq!(sdk.protocol_version_number(), if pinned { 13 } else { 14 });
    }
}

struct RetryTable;

impl FromProof<GetEpochsInfoRequest> for RetryTable {
    type Request = GetEpochsInfoRequest;
    type Response = GetEpochsInfoResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _: I,
        response: O,
        _: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), ProofError>
    where
        Self: Sized + 'a,
    {
        let response = response.into();
        provider.get_data_contract(&Identifier::new([0; 32]), platform_version)?;
        let proof = response.proof()?.clone();
        let metadata = response.metadata()?.clone();
        let quorum_hash = proof
            .quorum_hash
            .as_slice()
            .try_into()
            .expect("recorded quorum hash");
        provider
            .get_quorum_public_key(
                proof.quorum_type,
                quorum_hash,
                metadata.core_chain_locked_height,
            )
            .map_err(|error| ProofError::QuorumKeyUnavailable {
                quorum_type: proof.quorum_type,
                quorum_hash,
                core_chain_locked_height: metadata.core_chain_locked_height,
                error,
            })?;
        Ok((None, metadata, proof))
    }
}

struct RetryProbe {
    observations: Mutex<Vec<u32>>,
    cached: AtomicBool,
    stored: Arc<AtomicU32>,
}

impl ContextProvider for RetryProbe {
    fn get_data_contract(
        &self,
        _: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.observations
            .lock()
            .expect("observations")
            .push(platform_version.protocol_version);
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
        if self.cached.load(Ordering::Relaxed) {
            Ok([1; 48])
        } else {
            Err(ContextProviderError::InvalidQuorum("not cached".into()))
        }
    }
    fn fetch_quorum_public_key(&self, _: u32, _: [u8; 32], _: u32) -> Option<QuorumKeyFuture> {
        self.cached.store(true, Ordering::Relaxed);
        self.stored.store(14, Ordering::Relaxed);
        Some(Box::pin(async { Ok(Some([1; 48])) }))
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

#[tokio::test]
async fn should_keep_the_selected_table_while_fetching_a_quorum_key() {
    let sdk = SdkBuilder::new_mock()
        .with_initial_version(PlatformVersion::get(12).expect("seed"))
        .with_height_tolerance(None)
        .build()
        .expect("sdk");
    let provider = RetryProbe {
        observations: Mutex::new(vec![]),
        cached: AtomicBool::new(false),
        stored: sdk.protocol_version.clone(),
    };
    let (request, response) = recorded_response(13);
    sdk.verify_fetching_quorum_key::<GetEpochsInfoRequest, RetryTable>(
        request,
        response,
        &provider,
        "get_epochs_info",
    )
    .await
    .expect("key retry");
    assert_eq!(
        *provider.observations.lock().expect("observations"),
        vec![13, 13]
    );
    assert_eq!(sdk.protocol_version_number(), 14);
}

#[tokio::test]
async fn should_reject_metadata_tampering_with_the_signed_version() {
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/test_epoch_fetch");
    let sdk = SdkBuilder::new_mock()
        .with_dump_dir(&directory)
        .with_initial_version(PlatformVersion::get(13).expect("seed"))
        .with_height_tolerance(None)
        .build()
        .expect("sdk");
    let (request, response) = recorded_epoch_response();
    assert_eq!(
        response
            .metadata()
            .expect("recorded metadata")
            .protocol_version,
        9
    );
    sdk.parse_proof_with_metadata_and_proof::<GetEpochsInfoRequest, ExtendedEpochInfo>(
        request,
        response,
        "get_epochs_info",
    )
    .await
    .expect("unaltered signature verifies");
    let (request, response) = recorded_response(14);
    let error = sdk
        .parse_proof_with_metadata_and_proof::<GetEpochsInfoRequest, ExtendedEpochInfo>(
            request,
            response,
            "get_epochs_info",
        )
        .await
        .expect_err("altering the signed app_version invalidates the signature");
    assert!(
        matches!(error, Error::Proof(ProofError::InvalidSignature { .. })),
        "{error:?}"
    );
    assert_eq!(sdk.protocol_version_number(), 13);
}

#[tokio::test]
async fn should_not_warn_about_unauthenticated_protocol_versions() {
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/test_epoch_fetch");
    let sdk = SdkBuilder::new_mock()
        .with_dump_dir(&directory)
        .with_initial_version(PlatformVersion::get(13).expect("seed"))
        .with_height_tolerance(None)
        .build()
        .expect("sdk");
    let warning = Arc::new(Mutex::new(Vec::new()));
    let captured = warning.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .with_writer(move || WarningBuffer(captured.clone()))
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let (request, response) = recorded_response(PlatformVersion::latest().protocol_version + 1);
    let error = sdk
        .parse_proof_with_metadata_and_proof::<GetEpochsInfoRequest, ExtendedEpochInfo>(
            request,
            response,
            "get_epochs_info",
        )
        .await
        .expect_err("tampered metadata must fail authentication");
    assert!(
        matches!(error, Error::Proof(ProofError::InvalidSignature { .. })),
        "{error:?}"
    );
    let warning =
        String::from_utf8(warning.lock().expect("warnings").clone()).expect("UTF8 warning");
    assert!(
        !warning.contains("using newest known verifier table"),
        "{warning}"
    );
    assert_eq!(sdk.protocol_version_number(), 13);
}

fn document_outcome_vector(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../rs-drive-proof-verifier/tests/vectors/document-outcome")
            .join(name),
    )
    .expect("signed document-outcome fixture")
}

struct DocumentOutcomeProvider {
    contract: Arc<DataContract>,
}

impl DocumentOutcomeProvider {
    fn new(version: u32) -> Self {
        Self::for_vectors(
            &format!("pv{version}"),
            PlatformVersion::get(version).unwrap(),
        )
    }
    /// The contract of the `prefix` vectors, read with `platform_version`.
    fn for_vectors(prefix: &str, platform_version: &PlatformVersion) -> Self {
        Self {
            contract: Arc::new(
                DataContract::versioned_deserialize_trusted(
                    &document_outcome_vector(&format!("{prefix}-contract.bin")),
                    true,
                    platform_version,
                )
                .expect("locally generated contract fixture"),
            ),
        }
    }
}

impl ContextProvider for DocumentOutcomeProvider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok((*id == self.contract.id()).then(|| self.contract.clone()))
    }
    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
    fn get_quorum_public_key(
        &self,
        kind: u32,
        quorum: [u8; 32],
        _: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        assert_eq!((kind, quorum), (106, [2; 32]));
        Ok(std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../rs-drive-proof-verifier/tests/vectors/document-outcome/quorum-key.bin"),
        )
        .unwrap()
        .try_into()
        .unwrap())
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

fn document_outcome_response(
    version: u32,
) -> (
    BroadcastStateTransitionRequest,
    WaitForStateTransitionResultResponse,
) {
    document_outcome_vectors(&format!("pv{version}"))
}

fn document_outcome_vectors(
    prefix: &str,
) -> (
    BroadcastStateTransitionRequest,
    WaitForStateTransitionResultResponse,
) {
    (
        BroadcastStateTransitionRequest::decode(
            document_outcome_vector(&format!("{prefix}-request.bin")).as_slice(),
        )
        .unwrap(),
        WaitForStateTransitionResultResponse::decode(
            document_outcome_vector(&format!("{prefix}-response.bin")).as_slice(),
        )
        .unwrap(),
    )
}

/// Each signed producer's outcome verifies in its own layout, from an SDK above, at or below
/// the producer, pinned or not. A pin keeps the stored version even when a newer producer
/// raised the rules for one answer (`should_verify_with_the_signed_newer_version_when_pinned`
/// shows which table that answer used); an unpinned SDK learns the newer producer's version.
#[tokio::test]
async fn should_verify_signed_document_outcome_layouts_without_lowering_security_or_pins() {
    let latest = PlatformVersion::latest().protocol_version;
    for pinned in [false, true] {
        for (stored, producer) in [(latest, 13), (latest, 14), (13, 14)] {
            let provider = DocumentOutcomeProvider::new(producer);
            let builder = SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .with_time_tolerance(None)
                .with_height_tolerance(Some(1))
                .with_trusted_initial_height(10);
            let stored_version = PlatformVersion::get(stored).unwrap();
            let sdk = if pinned {
                builder.with_version(stored_version)
            } else {
                builder.with_initial_version(stored_version)
            }
            .build()
            .unwrap();
            let (request, response) = document_outcome_response(producer);
            let (outcome, _, _) = sdk.verify_fetching_quorum_key::<BroadcastStateTransitionRequest, StateTransitionProofOutcome>(request, response, &provider, "wait_for_state_transition_result").await.expect("signed producer layout with retained current security");
            let outcome = outcome.unwrap();
            assert!(outcome.is_execution_proved());
            assert_eq!(outcome.owner_balance().is_some(), producer >= 14);
            let expected = if pinned { stored } else { stored.max(producer) };
            assert_eq!(
                sdk.protocol_version_number(),
                expected,
                "stored {stored}, producer {producer}, pinned {pinned}"
            );
        }
    }
}

#[tokio::test]
async fn should_not_learn_document_outcome_versions_from_stale_or_tampered_responses() {
    for stale in [false, true] {
        let provider = DocumentOutcomeProvider::new(13);
        // Seeded below both the signed version and the tampered one, so learning either
        // would show.
        let sdk = SdkBuilder::new_mock()
            .with_network(Network::Testnet)
            .with_initial_version(PlatformVersion::get(11).unwrap())
            .with_time_tolerance(None)
            .with_height_tolerance(Some(1))
            .with_trusted_initial_height(if stale { 100 } else { 10 })
            .build()
            .unwrap();
        let (request, mut response) = document_outcome_response(13);
        if !stale {
            let Some(WaitResponseVersion::V0(v0)) = &mut response.version else {
                panic!("v0")
            };
            v0.metadata.as_mut().unwrap().protocol_version = 12;
        }
        let error = sdk.verify_fetching_quorum_key::<BroadcastStateTransitionRequest, StateTransitionProofOutcome>(request, response, &provider, "wait_for_state_transition_result").await.unwrap_err();
        if stale {
            assert!(matches!(error, Error::StaleNode(_)), "{error}");
        } else {
            assert!(
                matches!(error, Error::Proof(ProofError::InvalidSignature { .. })),
                "{error}"
            );
        }
        assert_eq!(sdk.protocol_version_number(), 11);
    }
}

#[tokio::test]
async fn should_learn_document_outcome_versions_monotonically_from_signed_producers() {
    let sdk = SdkBuilder::new_mock()
        .with_network(Network::Testnet)
        .with_initial_version(PlatformVersion::get(12).unwrap())
        .with_time_tolerance(None)
        .with_height_tolerance(Some(1))
        .with_trusted_initial_height(10)
        .build()
        .unwrap();
    let older_provider = DocumentOutcomeProvider::new(13);
    let newer_provider = DocumentOutcomeProvider::new(14);
    let (older_request, older_response) = document_outcome_response(13);
    let (newer_request, newer_response) = document_outcome_response(14);
    // Newer first: a later, older outcome must still verify but never lower the version.
    sdk.verify_fetching_quorum_key::<BroadcastStateTransitionRequest, StateTransitionProofOutcome>(
        newer_request,
        newer_response,
        &newer_provider,
        "wait_for_state_transition_result",
    )
    .await
    .unwrap();
    assert_eq!(sdk.protocol_version_number(), 14);
    sdk.verify_fetching_quorum_key::<BroadcastStateTransitionRequest, StateTransitionProofOutcome>(
        older_request,
        older_response,
        &older_provider,
        "wait_for_state_transition_result",
    )
    .await
    .unwrap();
    assert_eq!(sdk.protocol_version_number(), 14);
}

/// After a newer protocol activates, this SDK still verifies document outcomes with the
/// newest known rules, which keep requiring the owner balance, and does not learn the
/// unknown version.
#[tokio::test]
async fn should_verify_document_outcomes_signed_by_an_unknown_newer_version() {
    let latest = PlatformVersion::latest();
    let sdk = SdkBuilder::new_mock()
        .with_network(Network::Testnet)
        .with_initial_version(latest)
        .with_time_tolerance(None)
        .with_height_tolerance(Some(1))
        .with_trusted_initial_height(10)
        .build()
        .unwrap();
    let provider = DocumentOutcomeProvider::for_vectors("newer", latest);
    let (request, response) = document_outcome_vectors("newer");
    let (outcome, metadata, _) = sdk
        .verify_fetching_quorum_key::<BroadcastStateTransitionRequest, StateTransitionProofOutcome>(
            request,
            response,
            &provider,
            "wait_for_state_transition_result",
        )
        .await
        .expect("newest known rules verify the newer producer's outcome");
    assert!(
        metadata.protocol_version > latest.protocol_version,
        "the vector must stay signed by a version this build does not know"
    );
    let outcome = outcome.unwrap();
    assert!(outcome.is_execution_proved());
    assert!(outcome.owner_balance().is_some());
    assert_eq!(sdk.protocol_version_number(), latest.protocol_version);
}

/// An SDK that verifies document outcomes at protocol 14 (the fixture contract's version)
/// and starts at protocol `version`, pinned or seeded.
fn document_sdk(version: u32, pinned: bool, time_tolerance_ms: Option<u64>) -> Sdk {
    let builder = SdkBuilder::new_mock()
        .with_network(Network::Testnet)
        .with_context_provider(DocumentOutcomeProvider::new(14))
        .with_time_tolerance(time_tolerance_ms)
        .with_height_tolerance(Some(1))
        .with_trusted_initial_height(10);
    let version = PlatformVersion::get(version).unwrap();
    if pinned {
        builder.with_version(version)
    } else {
        builder.with_initial_version(version)
    }
    .build()
    .unwrap()
}

/// Verify the document outcome signed at `version`, as an SDK does after a write.
async fn verify_document_outcome_signed_at(sdk: &Sdk, version: u32) {
    let (request, response) = document_outcome_response(version);
    sdk.verify_fetching_quorum_key::<BroadcastStateTransitionRequest, StateTransitionProofOutcome>(
        request,
        response,
        &DocumentOutcomeProvider::new(version),
        "wait_for_state_transition_result",
    )
    .await
    .expect("signed document outcome verifies");
}

/// The query of the document query tests: every `preorder` document, limit 10.
fn preorder_query() -> DocumentQuery {
    DocumentQuery::new(DocumentOutcomeProvider::new(14).contract, "preorder")
        .unwrap()
        .with_limit(10)
}

/// A node's answer to [`preorder_query`] at protocol 13, sent at `time_ms` and `height`. The
/// verifier refuses it before reading its proof, so the proof bytes are arbitrary.
fn older_document_answer(time_ms: u64, height: u64) -> GetDocumentsResponse {
    GetDocumentsResponse {
        version: Some(DocumentsResponseVersion::V0(GetDocumentsResponseV0 {
            result: Some(DocumentsResult::Proof(Proof {
                grovedb_proof: vec![1; 32],
                ..Default::default()
            })),
            metadata: Some(ResponseMetadata {
                height,
                time_ms,
                protocol_version: 13,
                ..Default::default()
            }),
        })),
    }
}

/// A node's answer to [`preorder_query`] at protocol 14, signed by the test quorum over a
/// state holding one document, at height 10.
fn signed_document_answer() -> GetDocumentsResponse {
    GetDocumentsResponse::decode(document_outcome_vector("pv14-documents-response.bin").as_slice())
        .unwrap()
}

/// A time tolerance that keeps both the recorded [`signed_document_answer`] and a current
/// answer fresh.
fn tolerance_admitting_the_signed_answer() -> u64 {
    let signed_at = signed_document_answer().metadata().unwrap().time_ms;
    now_ms() - signed_at + 24 * 60 * 60 * 1000
}

/// Ask `sdk` to verify `response` to [`preorder_query`] on `path`.
async fn verify_document_answer(
    sdk: &Sdk,
    path: ProofPath,
    response: GetDocumentsResponse,
) -> Result<Option<Documents>, Error> {
    let query = preorder_query();
    match path {
        ProofPath::Network => {
            sdk.verify_fetching_quorum_key::<DocumentQuery, Documents>(
                query,
                response,
                &DocumentOutcomeProvider::new(14),
                "get_documents",
            )
            .await
        }
        ProofPath::Mock => {
            sdk.parse_proof_with_metadata_and_proof::<DocumentQuery, Documents>(
                query,
                response,
                "get_documents",
            )
            .await
        }
    }
    .map(|(documents, _, _)| documents)
}

/// Ask `sdk` to verify a document query answer signed at protocol 13 at `time_ms`. Its
/// height is above every height the SDK has seen, so learning from it would show.
async fn verify_older_document_answer(sdk: &Sdk, path: ProofPath, time_ms: u64) -> Error {
    verify_document_answer(sdk, path, older_document_answer(time_ms, 20))
        .await
        .expect_err("a document answer signed below the verifier's version is not verified")
}

/// Fetch [`preorder_query`] as `Fetch` does, through the SDK's retry loop, from one node per
/// answer: each attempt asks the node the SDK's address list selects, as the DAPI client
/// does, and that node sends its answer. Returns the result, the nodes asked in order, and
/// each node's ban count afterwards.
///
/// With two nodes, the rotation holds one node at a time and starts at the first: the
/// second is asked only once the first leaves the rotation, by a ban or without one.
async fn fetch_documents_from(
    sdk: &mut Sdk,
    path: ProofPath,
    answers: Vec<GetDocumentsResponse>,
) -> (Result<Option<Documents>, Error>, Vec<usize>, Vec<usize>) {
    let nodes: Vec<Address> = (0..answers.len())
        .map(|node| format!("http://127.0.0.1:{}", 3000 + node).parse().unwrap())
        .collect();
    let SdkInstance::Mock { address_list, .. } = &mut sdk.inner else {
        panic!("mock SDK")
    };
    *address_list = if nodes.len() == 2 {
        AddressList::new().with_active_set_size(1)
    } else {
        AddressList::new()
    };
    address_list.add(nodes[0].clone());
    if nodes.len() == 2 {
        assert_eq!(address_list.get_live_address().as_ref(), Some(&nodes[0]));
    }
    for node in &nodes[1..] {
        address_list.add(node.clone());
    }
    let sdk = &*sdk;
    let asked = Mutex::new(Vec::new());
    let settings = RequestSettings {
        retries: Some(answers.len() - 1),
        ban_failed_address: Some(true),
        ..Default::default()
    };
    let result = retry(sdk.address_list(), settings, |_| {
        let address = sdk.address_list().get_live_address().expect("a live node");
        let node = nodes.iter().position(|node| *node == address).unwrap();
        asked.lock().unwrap().push(node);
        let answer = answers[node].clone();
        async move {
            verify_document_answer(sdk, path, answer)
                .await
                .map(|documents| ExecutionResponse {
                    inner: documents,
                    address: address.clone(),
                    retries: 0,
                })
                .map_err(|inner| ExecutionError {
                    inner,
                    address: Some(address),
                    retries: 0,
                })
        }
    })
    .await
    .into_inner();
    let ban_info = sdk.address_list().ban_info();
    let bans = nodes
        .iter()
        .map(|node| {
            ban_info
                .iter()
                .find(|ban| ban.uri == node.to_string())
                .unwrap()
                .ban_count
        })
        .collect();
    (result, asked.into_inner().unwrap(), bans)
}

fn now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis() as u64
}

/// The stored version, the highest verified version and the height mark.
fn learned_state(sdk: &Sdk) -> (u32, u32, u64) {
    (
        sdk.protocol_version_number(),
        sdk.verified_protocol_version.load(Ordering::Relaxed),
        sdk.metadata_last_seen_height.load(Ordering::Relaxed),
    )
}

/// Once a verified answer has shown the network running the SDK's version, a node that
/// answers a document query at an older version is behind: the error is retryable, so the
/// request moves to another node and this one is banned, and nothing is learned from it.
#[tokio::test]
async fn should_retry_document_answers_signed_below_a_version_the_network_has_shown() {
    for path in [ProofPath::Network, ProofPath::Mock] {
        let sdk = document_sdk(13, false, None);
        verify_document_outcome_signed_at(&sdk, 14).await;
        let learned = learned_state(&sdk);
        assert_eq!(learned, (14, 14, 10));
        let error = verify_older_document_answer(&sdk, path, now_ms()).await;
        assert!(
            matches!(
                error,
                Error::Proof(ProofError::ProofProtocolVersionMismatch {
                    signed: 13,
                    verifier: 14
                })
            ),
            "{error:?}"
        );
        assert!(error.can_retry());
        assert!(!error.spares_responder());
        assert!(matches!(
            error.into_exhausted_error(),
            Error::Proof(ProofError::ProofProtocolVersionMismatch { .. })
        ));
        assert_eq!(learned_state(&sdk), learned);
    }
}

/// An SDK pinned, seeded or persisted ahead of its network has no verified answer at its
/// own version, and its network's nodes all answer below it. Banning them would empty the
/// pool, so such an answer moves the request to another node without banning this one, and
/// a request that runs out of attempts on it fails as a configuration error. Nothing is
/// learned from the answer.
#[tokio::test]
async fn should_spare_nodes_answering_below_a_version_the_network_has_not_shown() {
    for path in [ProofPath::Network, ProofPath::Mock] {
        for pinned in [false, true] {
            // Fresh, or having verified only answers signed below the SDK's version.
            for network in [None, Some(13)] {
                let sdk = document_sdk(14, pinned, None);
                if let Some(network) = network {
                    verify_document_outcome_signed_at(&sdk, network).await;
                }
                let learned = learned_state(&sdk);
                assert_eq!(learned, (14, network.unwrap_or(0), 10));
                let error = verify_older_document_answer(&sdk, path, now_ms()).await;
                assert!(
                    matches!(
                        error,
                        Error::StaleNode(StaleNodeError::ProtocolVersion {
                            expected_protocol_version: 14,
                            received_protocol_version: 13,
                        })
                    ),
                    "{error:?}"
                );
                assert!(error.can_retry());
                assert!(error.spares_responder());
                assert!(matches!(error.into_exhausted_error(), Error::Config(_)));
                assert_eq!(learned_state(&sdk), learned);
            }
        }
    }
}

/// An answer whose metadata names no protocol version is malformed: no node on any version
/// sends one, so it says nothing about an SDK set ahead of its network. Its node is banned
/// as for any unusable proof, even before a verified answer has shown the SDK's version.
#[tokio::test]
async fn should_ban_nodes_answering_without_a_protocol_version() {
    for path in [ProofPath::Network, ProofPath::Mock] {
        let mut sdk = document_sdk(14, false, Some(tolerance_admitting_the_signed_answer()));
        let mut unnamed = older_document_answer(now_ms(), 20);
        let Some(DocumentsResponseVersion::V0(v0)) = unnamed.version.as_mut() else {
            panic!("v0 answer")
        };
        v0.metadata.as_mut().unwrap().protocol_version = 0;
        let error = verify_document_answer(&sdk, path, unnamed.clone())
            .await
            .expect_err("an answer without a protocol version is not verified");
        assert!(
            matches!(
                error,
                Error::Proof(ProofError::ProofProtocolVersionMismatch {
                    signed: 0,
                    verifier: 14
                })
            ),
            "{error:?}"
        );
        assert!(!error.spares_responder());
        let (result, asked, bans) =
            fetch_documents_from(&mut sdk, path, vec![unnamed, signed_document_answer()]).await;
        assert_eq!(result.unwrap().unwrap().len(), 1);
        assert_eq!(asked, [0, 1]);
        assert_eq!(
            bans,
            [1, 0],
            "the node without a protocol version is banned"
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum Freshness {
    Fresh,
    StaleTime,
    StaleHeight,
}

/// An answer outside the height or time tolerance comes from a stale node, whatever the
/// SDK's version: it is retried on another node, which bans this one, even before any
/// verified answer has shown the network running the SDK's version. Its height is compared
/// with the SDK's height mark but never raises it. Answers from nodes keeping up with the
/// network are fresh, so an SDK set ahead of its network still bans none.
#[tokio::test]
async fn should_retry_stale_document_answers_signed_below_a_version_the_network_has_not_shown() {
    const TOLERANCE_MS: u64 = 60_000;
    for path in [ProofPath::Network, ProofPath::Mock] {
        for freshness in [
            Freshness::Fresh,
            Freshness::StaleTime,
            Freshness::StaleHeight,
        ] {
            let sdk = document_sdk(14, false, Some(TOLERANCE_MS));
            let learned = learned_state(&sdk);
            assert_eq!(learned, (14, 0, 10));
            let answer = match freshness {
                Freshness::Fresh => older_document_answer(now_ms(), 20),
                Freshness::StaleTime => older_document_answer(now_ms() - 10 * TOLERANCE_MS, 20),
                Freshness::StaleHeight => older_document_answer(now_ms(), 5),
            };
            let error = verify_document_answer(&sdk, path, answer)
                .await
                .expect_err("a document answer signed below the verifier's version");
            match freshness {
                Freshness::Fresh => assert!(error.spares_responder(), "{error:?}"),
                Freshness::StaleTime => assert!(
                    matches!(error, Error::StaleNode(StaleNodeError::Time { .. })),
                    "{error:?}"
                ),
                Freshness::StaleHeight => assert!(
                    matches!(
                        error,
                        Error::StaleNode(StaleNodeError::Height {
                            expected_height: 10,
                            received_height: 5,
                            tolerance_blocks: 1,
                        })
                    ),
                    "{error:?}"
                ),
            }
            assert!(error.can_retry(), "{freshness:?}");
            let spared = error.spares_responder();
            assert_eq!(
                matches!(error.into_exhausted_error(), Error::Config(_)),
                spared,
                "only a spared answer ends as a configuration error; {freshness:?}"
            );
            assert_eq!(learned_state(&sdk), learned, "{freshness:?}");
        }
    }
}

/// Before any verified answer has shown the network running the SDK's version, a node that
/// answers at an older version may be behind, or the SDK ahead: the request moves to the
/// next node without banning it. The next node answers at the SDK's version and is
/// verified. The refused answer's higher height never raised the height mark, which would
/// have made the verified answer stale.
#[tokio::test]
async fn should_fetch_documents_from_the_next_node_when_one_answers_below_an_unshown_version() {
    for path in [ProofPath::Network, ProofPath::Mock] {
        for pinned in [false, true] {
            let mut sdk = document_sdk(14, pinned, Some(tolerance_admitting_the_signed_answer()));
            assert_eq!(learned_state(&sdk), (14, 0, 10));
            let (result, asked, bans) = fetch_documents_from(
                &mut sdk,
                path,
                vec![
                    older_document_answer(now_ms(), 20),
                    signed_document_answer(),
                ],
            )
            .await;
            let documents = result
                .expect("the node answering at the SDK's version is verified")
                .expect("the proved page");
            assert_eq!(documents.len(), 1, "pinned={pinned}");
            assert_eq!(asked, [0, 1], "pinned={pinned}");
            assert_eq!(bans, [0, 0], "no node is banned; pinned={pinned}");
            assert_eq!(learned_state(&sdk), (14, 14, 10), "pinned={pinned}");
        }
    }
}

/// When every node answers below the SDK's version and no verified answer has shown the
/// network running it, the SDK is ahead of its network: the request asks every node it may,
/// bans none, and fails with a configuration error. Nothing is learned.
#[tokio::test]
async fn should_fail_with_a_configuration_error_when_no_node_answers_at_the_sdk_version() {
    for path in [ProofPath::Network, ProofPath::Mock] {
        for pinned in [false, true] {
            let mut sdk = document_sdk(14, pinned, None);
            let learned = learned_state(&sdk);
            let (result, mut asked, bans) =
                fetch_documents_from(&mut sdk, path, vec![older_document_answer(now_ms(), 20); 4])
                    .await;
            let error = result.expect_err("no node answers at the SDK's version");
            let Error::Config(message) = &error else {
                panic!("{error:?}")
            };
            for named in [
                "protocol version 13",
                "protocol version 14",
                "with_initial_version",
            ] {
                assert!(message.contains(named), "{message}");
            }
            assert!(!error.can_retry());
            assert_eq!(asked.len(), 4, "pinned={pinned}");
            asked.sort();
            assert_eq!(
                asked,
                [0, 1, 2, 3],
                "every node is asked once; pinned={pinned}"
            );
            assert_eq!(bans, [0; 4], "no node is banned; pinned={pinned}");
            assert_eq!(learned_state(&sdk), learned, "pinned={pinned}");
        }
    }
}

/// Once a verified answer has shown the network running the SDK's version, a node answering
/// below it is behind: the request moves to the next node, and the node behind is banned.
#[tokio::test]
async fn should_ban_a_node_answering_below_a_version_the_network_has_shown() {
    for path in [ProofPath::Network, ProofPath::Mock] {
        // Seeded below and raised by the verified answer, or pinned at that version.
        for (version, pinned) in [(13, false), (14, true)] {
            let mut sdk = document_sdk(
                version,
                pinned,
                Some(tolerance_admitting_the_signed_answer()),
            );
            verify_document_outcome_signed_at(&sdk, 14).await;
            assert_eq!(learned_state(&sdk), (14, 14, 10));
            let (result, asked, bans) = fetch_documents_from(
                &mut sdk,
                path,
                vec![
                    older_document_answer(now_ms(), 20),
                    signed_document_answer(),
                ],
            )
            .await;
            assert_eq!(result.unwrap().unwrap().len(), 1, "pinned={pinned}");
            assert_eq!(asked, [0, 1], "pinned={pinned}");
            assert_eq!(bans, [1, 0], "the node behind is banned; pinned={pinned}");
            assert_eq!(learned_state(&sdk), (14, 14, 10), "pinned={pinned}");
        }
    }
}

/// Once a verified answer has shown the network running the SDK's version, nodes answering
/// below it are behind, not a sign of an SDK set ahead of its network: each one asked is
/// banned, and the query fails with the retryable proof error, not a configuration error.
#[tokio::test]
async fn should_ban_every_node_answering_below_a_version_the_network_has_shown() {
    for path in [ProofPath::Network, ProofPath::Mock] {
        let mut sdk = document_sdk(13, false, None);
        verify_document_outcome_signed_at(&sdk, 14).await;
        let learned = learned_state(&sdk);
        let (result, mut asked, bans) =
            fetch_documents_from(&mut sdk, path, vec![older_document_answer(now_ms(), 20); 4])
                .await;
        let error = result.expect_err("no node answers at the SDK's version");
        assert!(
            matches!(
                error,
                Error::Proof(ProofError::ProofProtocolVersionMismatch { .. })
            ),
            "{error:?}"
        );
        assert!(error.can_retry());
        asked.sort();
        assert_eq!(asked, [0, 1, 2, 3], "every node is asked once");
        assert_eq!(bans, [1; 4], "every node behind is banned");
        assert_eq!(learned_state(&sdk), learned);
    }
}

/// A node whose answer is outside the height or time tolerance is stale whatever its
/// version, so it is banned even before any verified answer has shown the network running
/// the SDK's version, and the request moves to the next node.
#[tokio::test]
async fn should_ban_a_stale_node_answering_below_an_unshown_version() {
    for path in [ProofPath::Network, ProofPath::Mock] {
        for freshness in [Freshness::StaleTime, Freshness::StaleHeight] {
            let tolerance = tolerance_admitting_the_signed_answer();
            let mut sdk = document_sdk(14, false, Some(tolerance));
            let stale = match freshness {
                Freshness::StaleTime => {
                    older_document_answer(now_ms() - tolerance - 24 * 60 * 60 * 1000, 20)
                }
                Freshness::StaleHeight => older_document_answer(now_ms(), 5),
                Freshness::Fresh => unreachable!("only stale answers are asked here"),
            };
            let (result, asked, bans) =
                fetch_documents_from(&mut sdk, path, vec![stale, signed_document_answer()]).await;
            assert_eq!(result.unwrap().unwrap().len(), 1, "{freshness:?}");
            assert_eq!(asked, [0, 1], "{freshness:?}");
            assert_eq!(bans, [1, 0], "the stale node is banned; {freshness:?}");
            assert_eq!(learned_state(&sdk), (14, 14, 10), "{freshness:?}");
        }
    }
}

/// The verified version is recorded before the stored version is raised, so whatever sees a
/// raised stored version also sees the verified answer that raised it. The observer runs
/// as the stored version is raised.
#[tokio::test]
async fn should_record_the_verified_version_before_raising_the_stored_one() {
    let verified: Arc<OnceLock<Arc<AtomicU32>>> = Arc::new(OnceLock::new());
    let observed = Arc::new(Mutex::new(Vec::new()));
    let (verified_slot, record) = (verified.clone(), observed.clone());
    let sdk = SdkBuilder::new_mock()
        .with_network(Network::Testnet)
        .with_initial_version(PlatformVersion::get(13).unwrap())
        .with_time_tolerance(None)
        .with_height_tolerance(Some(1))
        .with_trusted_initial_height(10)
        .with_protocol_version_observer(Arc::new(move |stored| {
            let verified = verified_slot
                .get()
                .map(|verified| verified.load(Ordering::Relaxed));
            record.lock().unwrap().push((stored, verified));
        }))
        .build()
        .unwrap();
    verified
        .set(Arc::clone(&sdk.verified_protocol_version))
        .unwrap();
    verify_document_outcome_signed_at(&sdk, 14).await;
    assert_eq!(*observed.lock().unwrap(), vec![(14, Some(14))]);
}
