use super::{Sdk, SdkBuilder};
use crate::mock::{MockDashPlatformSdk, MockResponse};
use crate::platform::trunk_branch_sync::{
    run_full_tree_scan, BranchQueryParams, KeyLeafTracker, TrunkBranchSyncOps, TrunkQueryResponse,
};
use crate::Error;
use dapi_grpc::platform::v0::{
    GetDataContractRequest, GetEpochsInfoRequest, GetEpochsInfoResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::{ContextProvider, ContextProviderError, QuorumKeyFuture};
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::dashcore::Network;
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use drive::grovedb::{GroveBranchQueryResult, GroveTrunkQueryResult, LeafInfo, MerkProofNode};
use drive_proof_verifier::FromProof;
use rs_dapi_client::{AddressList, DumpData, RequestSettings};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

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
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
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

// Changed metadata is used only by verifier doubles and signature-rejection
// tests; a response with changed metadata is not an authenticated fixture.
fn recorded_response(signed: u32) -> (GetEpochsInfoRequest, GetEpochsInfoResponse) {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "tests/vectors/test_epoch_fetch/msg_GetEpochsInfoRequest_b2b426ac4a52cb4cb08904c63386caf3663c40a12d3b03827006d66058e439ac.json",
    );
    let (request, response) = DumpData::<GetEpochsInfoRequest>::load(file)
        .expect("recorded epoch response")
        .deserialize();
    let mut response = response.expect("successful recorded response").inner;
    let Some(dapi_grpc::platform::v0::get_epochs_info_response::Version::V0(v0)) =
        response.version.as_mut()
    else {
        panic!("recorded version");
    };
    v0.metadata
        .as_mut()
        .expect("recorded metadata")
        .protocol_version = signed;
    (request, response)
}

async fn observed_table(stored: u32, signed: u32, pinned: bool, mock: bool) -> (u32, u32) {
    let builder = if mock {
        SdkBuilder::new_mock()
    } else {
        SdkBuilder::new(AddressList::new())
    }
    .with_network(Network::Regtest)
    .with_context_provider(NoKeys)
    .with_time_tolerance(None)
    .with_height_tolerance(Some(1))
    .with_trusted_initial_height(2);
    let version = PlatformVersion::get(stored).expect("known stored table");
    let builder = if pinned {
        builder.with_version(version)
    } else {
        builder.with_initial_version(version)
    };
    let sdk = builder.build().expect("sdk");
    let (request, response) = recorded_response(signed);
    let (observed, _, _) = if mock {
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
        _: &dpp::prelude::Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<std::sync::Arc<dpp::prelude::DataContract>>, ContextProviderError> {
        Ok(None)
    }
    fn get_token_configuration(
        &self,
        _: &dpp::prelude::Identifier,
    ) -> Result<Option<dpp::data_contract::TokenConfiguration>, ContextProviderError> {
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
    fn get_platform_activation_height(
        &self,
    ) -> Result<dpp::prelude::CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

#[tokio::test]
async fn should_verify_with_the_signed_newer_version_not_the_stored_one() {
    assert_eq!(observed_table(13, 14, false, false).await, (14, 14));
}

#[tokio::test]
async fn should_verify_mock_responses_with_the_signed_newer_version() {
    assert_eq!(observed_table(13, 14, false, true).await, (14, 14));
}

#[tokio::test]
async fn should_never_verify_below_the_stored_version() {
    assert_eq!(observed_table(14, 13, false, false).await, (14, 14));
    assert_eq!(observed_table(14, 0, false, false).await, (14, 14));
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
        observed_table(13, 99, false, false).await,
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

impl std::io::Write for WarningBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("warnings").extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn should_verify_with_the_signed_newer_version_when_pinned() {
    assert_eq!(observed_table(13, 14, true, false).await, (14, 13));
}

#[tokio::test]
async fn should_read_a_recorded_pv14_contract_first_with_an_sdk_seeded_at_13() {
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
        .expect("first contract read must parse and verify with the signed table");
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
        sdk.verify_response_metadata(
            "get_addresses_trunk_state",
            &ResponseMetadata {
                protocol_version: 14,
                ..Default::default()
            },
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
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: Sized + 'a,
    {
        let response = response.into();
        provider.get_data_contract(&dpp::prelude::Identifier::new([0; 32]), platform_version)?;
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
            .map_err(|error| drive_proof_verifier::Error::QuorumKeyUnavailable {
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
        _: &dpp::prelude::Identifier,
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
        _: &dpp::prelude::Identifier,
    ) -> Result<Option<dpp::data_contract::TokenConfiguration>, ContextProviderError> {
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
    fn get_platform_activation_height(
        &self,
    ) -> Result<dpp::prelude::CoreBlockHeight, ContextProviderError> {
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
    let (request, response) = recorded_response(9);
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
        matches!(
            error,
            Error::Proof(drive_proof_verifier::Error::InvalidSignature { .. })
        ),
        "{error:?}"
    );
    assert_eq!(sdk.protocol_version_number(), 13);
}
