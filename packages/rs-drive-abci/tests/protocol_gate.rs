//! Signed answers the SDK uses to learn the network's protocol version before it sends a
//! request that only newer nodes serve, and answers to such requests signed by nodes still
//! at the previous protocol version.
mod signed_response;

use dapi_grpc::platform::v0::get_data_contracts_by_range_request::GetDataContractsByRangeRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_latest_versions_request::GetDataContractsLatestVersionsRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_latest_versions_response::{
    get_data_contracts_latest_versions_response_v0, GetDataContractsLatestVersionsResponseV0,
};
use dapi_grpc::platform::v0::get_data_contracts_response::{
    get_data_contracts_response_v0, GetDataContractsResponseV0,
};
use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
use dapi_grpc::platform::v0::get_epochs_info_response::{
    get_epochs_info_response_v0, GetEpochsInfoResponseV0,
};
use dapi_grpc::platform::v0::{
    get_data_contracts_by_range_request, get_data_contracts_latest_versions_request,
    get_data_contracts_latest_versions_response, get_data_contracts_response,
    get_epochs_info_request, get_epochs_info_response, GetDataContractsByRangeRequest,
    GetDataContractsLatestVersionsRequest, GetDataContractsLatestVersionsResponse,
    GetDataContractsResponse, GetEpochsInfoRequest, GetEpochsInfoResponse, ResponseMetadata,
};
use dapi_grpc::Message;
use dpp::block::epoch::Epoch;
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
use drive::drive::Drive;
use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use drive::util::batch::GroveDbOpBatch;
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use drive_proof_verifier::types::data_contracts_by_range::DataContractsByRange;
use drive_proof_verifier::types::data_contracts_latest_versions::DataContractsLatestVersions;
use drive_proof_verifier::types::ExtendedEpochInfos;
use drive_proof_verifier::{ContextProvider, FromProof};
use signed_response::{signed, Provider, NEWER_PROTOCOL_VERSION};
use std::path::PathBuf;

/// The contract whose latest version the recorded answer proves absent.
const CONTRACT_ID: [u8; 32] = [0x42; 32];

/// The epoch the epoch-2 states are in.
const CURRENT_EPOCH: u16 = 2;

/// The epoch length of the epoch-2 states.
const EPOCH_LENGTH_MS: u64 = 3_600_000;

/// The genesis time of an epoch-2 state far enough ahead that the hold an SDK derives from it
/// has not ended while its tests run.
const GENESIS_AHEAD_MS: u64 = 4_000_000_000_000;

/// The genesis time of an epoch-2 state whose epoch 2 ended before its answers' block time.
const GENESIS_ENDED_MS: u64 = 1_700_000_000_000 - 10 * EPOCH_LENGTH_MS;

/// When epochs 0 to 2 of the epoch-2 state with genesis at `genesis_ms` started: each first
/// block a second after its epoch's start on the grid.
fn epoch_starts_ms(genesis_ms: u64) -> [(u16, u64); 3] {
    [
        (0, genesis_ms),
        (1, genesis_ms + EPOCH_LENGTH_MS + 1000),
        (2, genesis_ms + 2 * EPOCH_LENGTH_MS + 1000),
    ]
}

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../rs-drive-proof-verifier/tests/vectors/protocol-gate")
}

/// A node's state at `protocol_version`, as initialized at genesis.
struct Node {
    drive: Drive,
    version: &'static PlatformVersion,
}

impl Node {
    fn new(protocol_version: u32) -> Self {
        let version = PlatformVersion::get(protocol_version).unwrap();
        Self {
            drive: setup_drive_with_initial_state_structure(Some(version)),
            version,
        }
    }

    /// A node's state at `protocol_version` in epoch [`CURRENT_EPOCH`], its epochs started
    /// at [`epoch_starts_ms`] of `genesis_ms`.
    fn in_epoch_2(protocol_version: u32, genesis_ms: u64) -> Self {
        let node = Self::new(protocol_version);
        let mut batch = GroveDbOpBatch::new();
        for (index, start_ms) in epoch_starts_ms(genesis_ms) {
            Epoch::new(index).unwrap().add_init_current_operations(
                1000,
                u64::from(index) * 100,
                u32::from(index) + 1,
                start_ms,
                protocol_version,
                &mut batch,
            );
        }
        node.drive
            .grove_apply_batch(batch, false, None, &node.version.drive)
            .unwrap();
        node
    }

    /// The genesis epoch this state proves, with the current-epoch hint in the metadata.
    fn epoch_probe_in_epoch_2(&self) -> GetEpochsInfoResponse {
        let mut answer = self.epochs_answer(0, 1);
        set_epoch_hint(&mut answer);
        answer
    }

    /// Epochs 2 and 3 of this state: the proof shows 2 is the newest that started.
    fn current_epoch(&self) -> GetEpochsInfoResponse {
        let mut answer = self.epochs_answer(CURRENT_EPOCH, 2);
        set_epoch_hint(&mut answer);
        answer
    }

    fn epochs_answer(&self, start: u16, count: u16) -> GetEpochsInfoResponse {
        let proof = self
            .drive
            .prove_epochs_infos(start, count, true, None, self.version)
            .unwrap();
        let metadata = self.metadata();
        GetEpochsInfoResponse {
            version: Some(get_epochs_info_response::Version::V0(
                GetEpochsInfoResponseV0 {
                    result: Some(get_epochs_info_response_v0::Result::Proof(signed(
                        proof,
                        &metadata,
                        self.root(),
                    ))),
                    metadata: Some(metadata),
                },
            )),
        }
    }

    fn root(&self) -> [u8; 32] {
        self.drive
            .grove
            .root_hash(None, &self.version.drive.grove_version)
            .unwrap()
            .unwrap()
    }

    fn metadata(&self) -> ResponseMetadata {
        self.metadata_signed_as(self.version.protocol_version)
    }

    fn metadata_signed_as(&self, protocol_version: u32) -> ResponseMetadata {
        ResponseMetadata {
            height: 10,
            core_chain_locked_height: 1,
            epoch: 0,
            time_ms: 1_700_000_000_000,
            protocol_version,
            chain_id: "dash-testnet-51".into(),
        }
    }

    fn epoch_probe(&self) -> GetEpochsInfoResponse {
        self.epoch_probe_signed_as(self.version.protocol_version)
    }

    /// The genesis epoch this state proves, signed as `protocol_version`.
    fn epoch_probe_signed_as(&self, protocol_version: u32) -> GetEpochsInfoResponse {
        let proof = self
            .drive
            .prove_epochs_infos(0, 1, true, None, self.version)
            .unwrap();
        let metadata = self.metadata_signed_as(protocol_version);
        GetEpochsInfoResponse {
            version: Some(get_epochs_info_response::Version::V0(
                GetEpochsInfoResponseV0 {
                    result: Some(get_epochs_info_response_v0::Result::Proof(signed(
                        proof,
                        &metadata,
                        self.root(),
                    ))),
                    metadata: Some(metadata),
                },
            )),
        }
    }

    fn contracts_by_range(&self) -> GetDataContractsResponse {
        let proof = self
            .drive
            .prove_contracts_by_range(None, 10, true, None, self.version)
            .unwrap();
        let metadata = self.metadata();
        GetDataContractsResponse {
            version: Some(get_data_contracts_response::Version::V0(
                GetDataContractsResponseV0 {
                    result: Some(get_data_contracts_response_v0::Result::Proof(signed(
                        proof,
                        &metadata,
                        self.root(),
                    ))),
                    metadata: Some(metadata),
                },
            )),
        }
    }

    /// The multi-contract proof a protocol 13 node answers a latest-versions request with.
    fn latest_versions(&self) -> GetDataContractsLatestVersionsResponse {
        let proof = self
            .drive
            .prove_contracts(&[CONTRACT_ID], None, self.version)
            .unwrap();
        let metadata = self.metadata();
        GetDataContractsLatestVersionsResponse {
            version: Some(get_data_contracts_latest_versions_response::Version::V0(
                GetDataContractsLatestVersionsResponseV0 {
                    result: Some(
                        get_data_contracts_latest_versions_response_v0::Result::Proof(signed(
                            proof,
                            &metadata,
                            self.root(),
                        )),
                    ),
                    metadata: Some(metadata),
                },
            )),
        }
    }
}

/// The epoch index in response metadata is not covered by the quorum signature, so it is set
/// after signing, as a node reports it.
fn set_epoch_hint(answer: &mut GetEpochsInfoResponse) {
    let Some(get_epochs_info_response::Version::V0(v0)) = answer.version.as_mut() else {
        panic!("v0 answer")
    };
    v0.metadata.as_mut().unwrap().epoch = u32::from(CURRENT_EPOCH);
}

/// The request that proves the current epoch of an epoch-2 state.
fn current_epoch_request() -> GetEpochsInfoRequest {
    GetEpochsInfoRequest {
        version: Some(get_epochs_info_request::Version::V0(
            GetEpochsInfoRequestV0 {
                start_epoch: Some(u32::from(CURRENT_EPOCH)),
                count: 2,
                ascending: true,
                prove: true,
            },
        )),
    }
}

/// The request the SDK sends to learn the network's protocol version: the genesis epoch.
fn epoch_probe_request() -> GetEpochsInfoRequest {
    GetEpochsInfoRequest {
        version: Some(get_epochs_info_request::Version::V0(
            GetEpochsInfoRequestV0 {
                start_epoch: Some(0),
                count: 1,
                ascending: true,
                prove: true,
            },
        )),
    }
}

fn contracts_by_range_request() -> GetDataContractsByRangeRequest {
    GetDataContractsByRangeRequest {
        version: Some(get_data_contracts_by_range_request::Version::V0(
            GetDataContractsByRangeRequestV0 {
                start: None,
                limit: Some(10),
                ids_only: true,
                prove: true,
            },
        )),
    }
}

fn latest_versions_request() -> GetDataContractsLatestVersionsRequest {
    GetDataContractsLatestVersionsRequest {
        version: Some(get_data_contracts_latest_versions_request::Version::V0(
            GetDataContractsLatestVersionsRequestV0 {
                ids: vec![CONTRACT_ID.to_vec()],
                include_contracts: false,
                prove: true,
            },
        )),
    }
}

fn rules(protocol_version: u32) -> &'static PlatformVersion {
    PlatformVersion::get(protocol_version).unwrap()
}

/// The signed version the SDK learns from verifying `response` under `rules_version`.
fn verified_epoch_probe(response: GetEpochsInfoResponse, rules_version: u32) -> u32 {
    let (_, metadata, _) =
        <ExtendedEpochInfo as FromProof<GetEpochsInfoRequest>>::maybe_from_proof_with_metadata(
            epoch_probe_request(),
            response,
            Network::Testnet,
            rules(rules_version),
            &Provider,
        )
        .unwrap();
    metadata.protocol_version
}

/// The started epochs, with their start times, an answer to [`current_epoch_request`]
/// proves.
fn verified_epoch_starts(response: GetEpochsInfoResponse) -> Vec<(u16, u64)> {
    let (epochs, _, _) =
        <ExtendedEpochInfos as FromProof<GetEpochsInfoRequest>>::maybe_from_proof_with_metadata(
            current_epoch_request(),
            response,
            Network::Testnet,
            rules(13),
            &Provider,
        )
        .unwrap();
    epochs
        .unwrap()
        .into_iter()
        .filter_map(|(index, epoch)| epoch.map(|epoch| (index, epoch.first_block_time())))
        .collect()
}

/// The genesis epoch answer verifies at every protocol version an SDK may hold, so any
/// SDK can learn the version the answering node signed it at.
#[test]
fn should_verify_the_epoch_probe_signed_at_13_and_14_under_both_rules() {
    for signed in [13, 14] {
        let answer = Node::new(signed).epoch_probe();
        for rules_version in [13, 14] {
            assert_eq!(
                verified_epoch_probe(answer.clone(), rules_version),
                signed,
                "signed at {signed}, verified under {rules_version}"
            );
        }
    }
}

/// A protocol 13 node serves the contract range and latest-versions requests, and its
/// answers verify under the rules of an SDK already at 14: only the signed version shows
/// the node has not activated protocol 14.
#[test]
fn should_verify_protocol_13_answers_to_requests_from_protocol_14_under_its_rules() {
    let node = Node::new(13);
    let (_, metadata, _) =
        <DataContractsByRange as FromProof<GetDataContractsByRangeRequest>>::maybe_from_proof_with_metadata(
            contracts_by_range_request(),
            node.contracts_by_range(),
            Network::Testnet,
            rules(14),
            &Provider,
        )
        .unwrap();
    assert_eq!(metadata.protocol_version, 13);
    let (versions, metadata, _) = <DataContractsLatestVersions as FromProof<
        GetDataContractsLatestVersionsRequest,
    >>::maybe_from_proof_with_metadata(
        latest_versions_request(),
        node.latest_versions(),
        Network::Testnet,
        rules(14),
        &Provider,
    )
    .unwrap();
    assert_eq!(metadata.protocol_version, 13);
    assert!(versions.is_some());
}

/// Writes the signed answers that `rs-sdk`'s protocol gate tests replay. Rewrites the
/// committed vectors, so running the suite with `--ignored` skips it unless
/// `DASH_SDK_EXPORT_PROTOCOL_GATE_VECTORS=1` is set.
#[test]
#[ignore = "requires DASH_SDK_EXPORT_PROTOCOL_GATE_VECTORS=1; regenerates deterministic signed SDK fixtures"]
fn should_export_protocol_gate_sdk_fixtures() {
    if std::env::var("DASH_SDK_EXPORT_PROTOCOL_GATE_VECTORS").as_deref() != Ok("1") {
        eprintln!("skipped: set DASH_SDK_EXPORT_PROTOCOL_GATE_VECTORS=1 to rewrite the vectors");
        return;
    }
    let directory = directory();
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("quorum-key.bin"),
        Provider.get_quorum_public_key(106, [2; 32], 0).unwrap(),
    )
    .unwrap();
    // The versions on either side of protocol 14, fixed so that re-running on a later
    // branch rewrites the same files the SDK tests read.
    for signed in [13, 14] {
        let answer = Node::new(signed).epoch_probe();
        assert_eq!(verified_epoch_probe(answer.clone(), signed), signed);
        std::fs::write(
            directory.join(format!("epoch-probe-pv{signed}.bin")),
            answer.encode_to_vec(),
        )
        .unwrap();
    }
    // A version newer than any build: the SDK reads it with the newest rules it knows.
    let newer = Node::new(14).epoch_probe_signed_as(NEWER_PROTOCOL_VERSION);
    assert_eq!(
        verified_epoch_probe(newer.clone(), 14),
        NEWER_PROTOCOL_VERSION
    );
    std::fs::write(
        directory.join("epoch-probe-newer.bin"),
        newer.encode_to_vec(),
    )
    .unwrap();
    for (suffix, genesis_ms) in [("", GENESIS_AHEAD_MS), ("-ended", GENESIS_ENDED_MS)] {
        let in_epoch_2 = Node::in_epoch_2(13, genesis_ms);
        let probe = in_epoch_2.epoch_probe_in_epoch_2();
        assert_eq!(verified_epoch_probe(probe.clone(), 13), 13);
        std::fs::write(
            directory.join(format!("epoch-probe-pv13-epoch-2{suffix}.bin")),
            probe.encode_to_vec(),
        )
        .unwrap();
        let current = in_epoch_2.current_epoch();
        assert_eq!(
            verified_epoch_starts(current.clone()),
            epoch_starts_ms(genesis_ms)[2..].to_vec()
        );
        std::fs::write(
            directory.join(format!("current-epoch-pv13-epoch-2{suffix}.bin")),
            current.encode_to_vec(),
        )
        .unwrap();
    }
    let node = Node::new(13);
    std::fs::write(
        directory.join("contracts-by-range-pv13.bin"),
        node.contracts_by_range().encode_to_vec(),
    )
    .unwrap();
    std::fs::write(
        directory.join("latest-versions-pv13.bin"),
        node.latest_versions().encode_to_vec(),
    )
    .unwrap();
}

/// The recorded answers still verify, so the SDK tests replaying them test the gate and
/// not a stale fixture.
#[test]
fn should_keep_the_recorded_protocol_gate_vectors_verifiable() {
    let read = |name: &str| std::fs::read(directory().join(name)).unwrap();
    for signed in [13, 14] {
        let answer =
            GetEpochsInfoResponse::decode(read(&format!("epoch-probe-pv{signed}.bin")).as_slice())
                .unwrap();
        for rules_version in [13, 14] {
            assert_eq!(verified_epoch_probe(answer.clone(), rules_version), signed);
        }
    }
    let newer = GetEpochsInfoResponse::decode(read("epoch-probe-newer.bin").as_slice()).unwrap();
    assert_eq!(verified_epoch_probe(newer, 14), NEWER_PROTOCOL_VERSION);
    for (suffix, genesis_ms) in [("", GENESIS_AHEAD_MS), ("-ended", GENESIS_ENDED_MS)] {
        let probe = GetEpochsInfoResponse::decode(
            read(&format!("epoch-probe-pv13-epoch-2{suffix}.bin")).as_slice(),
        )
        .unwrap();
        assert_eq!(verified_epoch_probe(probe, 13), 13);
        let current = GetEpochsInfoResponse::decode(
            read(&format!("current-epoch-pv13-epoch-2{suffix}.bin")).as_slice(),
        )
        .unwrap();
        assert_eq!(
            verified_epoch_starts(current),
            epoch_starts_ms(genesis_ms)[2..].to_vec()
        );
    }
    let by_range =
        GetDataContractsResponse::decode(read("contracts-by-range-pv13.bin").as_slice()).unwrap();
    <DataContractsByRange as FromProof<GetDataContractsByRangeRequest>>::maybe_from_proof(
        contracts_by_range_request(),
        by_range,
        Network::Testnet,
        rules(14),
        &Provider,
    )
    .unwrap();
    let latest =
        GetDataContractsLatestVersionsResponse::decode(read("latest-versions-pv13.bin").as_slice())
            .unwrap();
    <DataContractsLatestVersions as FromProof<GetDataContractsLatestVersionsRequest>>::maybe_from_proof(
        latest_versions_request(),
        latest,
        Network::Testnet,
        rules(14),
        &Provider,
    )
    .unwrap();
}
