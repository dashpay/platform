use std::{
    collections::HashSet,
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
    process::exit,
};

use tonic_build::Builder;

const SERDE_WITH_BYTES: &str = r#"#[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]"#;
const SERDE_WITH_BASE64: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::deserialization::vec_base64string"))]"#;
const SERDE_WITH_STRING: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::deserialization::from_to_string"))]"#;

fn main() {
    let core = MappingConfig::new(
        PathBuf::from("protos/core/v0/core.proto"),
        PathBuf::from("src/core"),
    );

    configure_core(core)
        .generate()
        .expect("generate core proto");

    let platform = MappingConfig::new(
        PathBuf::from("protos/platform/v0/platform.proto"),
        PathBuf::from("src/platform"),
    );

    configure_platform(platform)
        .generate()
        .expect("generate platform proto");

    println!("cargo:rerun-if-changed=./protos");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_SERDE");
}

struct MappingConfig {
    protobuf_file: PathBuf,
    out_dir: PathBuf,
    builder: Builder,
    proto_includes: Vec<PathBuf>,
}

fn configure_platform(mut platform: MappingConfig) -> MappingConfig {
    // Derive features for versioned messages
    //
    // "GetConsensusParamsRequest" is excluded as this message does not support proofs
    const VERSIONED_REQUESTS: [&str; 30] = [
        "GetDataContractHistoryRequest",
        "GetDataContractRequest",
        "GetDataContractsRequest",
        "GetDocumentsRequest",
        "GetIdentitiesByPublicKeyHashesRequest",
        "GetIdentitiesRequest",
        "GetIdentitiesBalancesRequest",
        "GetIdentityNonceRequest",
        "GetIdentityContractNonceRequest",
        "GetIdentityBalanceAndRevisionRequest",
        "GetIdentityBalanceRequest",
        "GetIdentityByPublicKeyHashRequest",
        "GetIdentityKeysRequest",
        "GetIdentityRequest",
        "GetProofsRequest",
        "WaitForStateTransitionResultRequest",
        "GetProtocolVersionUpgradeStateRequest",
        "GetProtocolVersionUpgradeVoteStatusRequest",
        "GetPathElementsRequest",
        "GetIdentitiesContractKeysRequest",
        "GetPrefundedSpecializedBalanceRequest",
        "GetContestedResourcesRequest",
        "GetContestedResourceVoteStateRequest",
        "GetContestedResourceVotersForIdentityRequest",
        "GetContestedResourceIdentityVotesRequest",
        "GetVotePollsByEndDateRequest",
        "GetTotalCreditsInPlatformRequest",
        "GetEvonodesProposedEpochBlocksByIdsRequest",
        "GetEvonodesProposedEpochBlocksByRangeRequest",
        "GetStatusRequest",
    ];

    // The following responses are excluded as they don't support proofs:
    // - "GetConsensusParamsResponse"
    // - "GetStatusResponse"
    //
    //  "GetEvonodesProposedEpochBlocksResponse" is used for 2 Requests
    const VERSIONED_RESPONSES: [&str; 29] = [
        "GetDataContractHistoryResponse",
        "GetDataContractResponse",
        "GetDataContractsResponse",
        "GetDocumentsResponse",
        "GetIdentitiesByPublicKeyHashesResponse",
        "GetIdentitiesResponse",
        "GetIdentitiesBalancesResponse",
        "GetIdentityBalanceAndRevisionResponse",
        "GetIdentityBalanceResponse",
        "GetIdentityNonceResponse",
        "GetIdentityContractNonceResponse",
        "GetIdentityByPublicKeyHashResponse",
        "GetIdentityKeysResponse",
        "GetIdentityResponse",
        "GetProofsResponse",
        "WaitForStateTransitionResultResponse",
        "GetEpochsInfoResponse",
        "GetProtocolVersionUpgradeStateResponse",
        "GetProtocolVersionUpgradeVoteStatusResponse",
        "GetPathElementsResponse",
        "GetIdentitiesContractKeysResponse",
        "GetPrefundedSpecializedBalanceResponse",
        "GetContestedResourcesResponse",
        "GetContestedResourceVoteStateResponse",
        "GetContestedResourceVotersForIdentityResponse",
        "GetContestedResourceIdentityVotesResponse",
        "GetVotePollsByEndDateResponse",
        "GetTotalCreditsInPlatformResponse",
        "GetEvonodesProposedEpochBlocksResponse",
    ];

    check_unique(&VERSIONED_REQUESTS).expect("VERSIONED_REQUESTS");
    check_unique(&VERSIONED_RESPONSES).expect("VERSIONED_RESPONSES");

    // Derive VersionedGrpcMessage on requests
    for msg in VERSIONED_REQUESTS {
        platform = platform
            .message_attribute(
                msg,
                r#"#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]"#,
            )
            .message_attribute(msg, r#"#[grpc_versions(0)]"#);
    }

    // Derive VersionedGrpcMessage and VersionedGrpcResponse on responses
    for msg in VERSIONED_RESPONSES {
        platform = platform
            .message_attribute(
                msg,
                r#"#[derive(::dapi_grpc_macros::VersionedGrpcMessage,::dapi_grpc_macros::VersionedGrpcResponse)]"#,
            )
            .message_attribute(msg, r#"#[grpc_versions(0)]"#);
    }

    // All messages can be mocked.
    let platform = platform.message_attribute(".", r#"#[derive( ::dapi_grpc_macros::Mockable)]"#);

    let platform = platform
        .type_attribute(
            ".",
            r#"#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]"#,
        )
        .type_attribute(
            ".",
            r#"#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]"#,
        )
        .field_attribute("id", SERDE_WITH_BYTES)
        .field_attribute("identity_id", SERDE_WITH_BYTES)
        .field_attribute("ids", SERDE_WITH_BASE64)
        .field_attribute("ResponseMetadata.height", SERDE_WITH_STRING)
        .field_attribute("ResponseMetadata.time_ms", SERDE_WITH_STRING)
        .field_attribute("start_at_ms", SERDE_WITH_STRING)
        .field_attribute("public_key_hash", SERDE_WITH_BYTES)
        .field_attribute("public_key_hashes", SERDE_WITH_BASE64)
        // Get documents fields
        .field_attribute("data_contract_id", SERDE_WITH_BYTES)
        .field_attribute("where", SERDE_WITH_BYTES)
        .field_attribute("order_by", SERDE_WITH_BYTES)
        // Proof fields
        .field_attribute("Proof.grovedb_proof", SERDE_WITH_BYTES)
        .field_attribute("Proof.quorum_hash", SERDE_WITH_BYTES)
        .field_attribute("Proof.signature", SERDE_WITH_BYTES)
        .field_attribute("Proof.block_id_hash", SERDE_WITH_BYTES);

    #[allow(clippy::let_and_return)]
    platform
}

/// Check for duplicate messages in the list.
fn check_unique(messages: &[&'static str]) -> Result<(), String> {
    let mut hashset: HashSet<&'static str> = HashSet::new();
    let mut duplicates = String::new();

    for value in messages {
        if !hashset.insert(*value) {
            duplicates.push_str(value);
            duplicates.push_str(", ");
        }
    }

    if duplicates.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Duplicate messages found: {}",
            duplicates.trim_end_matches(", ")
        ))
    }
}

fn configure_core(core: MappingConfig) -> MappingConfig {
    // All messages can be mocked.
    let core = core.message_attribute(".", r#"#[derive(::dapi_grpc_macros::Mockable)]"#);

    // Serde support
    let core = core.type_attribute(
        ".",
        r#"#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]"#,
    );

    #[allow(clippy::let_and_return)]
    core
}

impl MappingConfig {
    /// Create a new MappingConfig instance.
    ///
    /// ## Arguments
    ///
    /// * `protobuf_file` - Path to the protobuf file to use as input.
    /// * `out_dir` - Output directory where subdirectories for generated files will be created.
    ///
    /// Depending on the features, either `client`, `server` or `client_server` subdirectory
    /// will be created inside `out_dir`.
    fn new(protobuf_file: PathBuf, out_dir: PathBuf) -> Self {
        let protobuf_file = abs_path(&protobuf_file);

        let build_server = cfg!(feature = "server");
        let build_client = cfg!(feature = "client");

        // Depending on the features, we need to build the server, client or both.
        // We save these artifacts in separate directories to avoid overwriting the generated files
        // when another crate requires different features.
        let out_dir_suffix = match (build_server, build_client) {
            (true, true) => "client_server",
            (true, false) => "server",
            (false, true) => "client",
            (false, false) => {
                println!("WARNING: At least one of the features 'server' or 'client' must be enabled; dapi-grpc will not generate any files.");
                exit(0)
            }
        };

        let out_dir = abs_path(&out_dir.join(out_dir_suffix));

        let builder = tonic_build::configure()
            .build_server(build_server)
            .build_client(build_client)
            .build_transport(build_server || build_client)
            .out_dir(out_dir.clone())
            .protoc_arg("--experimental_allow_proto3_optional");

        Self {
            protobuf_file,
            out_dir,
            builder,
            proto_includes: vec![abs_path(&PathBuf::from("protos"))],
        }
    }

    #[allow(unused)]
    fn type_attribute(mut self, path: &str, attribute: &str) -> Self {
        self.builder = self.builder.type_attribute(path, attribute);
        self
    }

    #[allow(unused)]
    fn field_attribute(mut self, path: &str, attribute: &str) -> Self {
        self.builder = self.builder.field_attribute(path, attribute);
        self
    }

    #[allow(unused)]
    fn enum_attribute(mut self, path: &str, attribute: &str) -> Self {
        self.builder = self.builder.enum_attribute(path, attribute);
        self
    }

    #[allow(unused)]
    fn message_attribute(mut self, path: &str, attribute: &str) -> Self {
        self.builder = self.builder.message_attribute(path, attribute);
        self
    }

    /// Run single generation process.
    fn generate(self) -> Result<(), std::io::Error> {
        // Remove old compiled files; ignore errors
        if self.out_dir.exists() {
            remove_dir_all(&self.out_dir)?;
        }
        create_dir_all(&self.out_dir)?;

        self.builder
            .compile(&[self.protobuf_file], &self.proto_includes)
    }
}

fn abs_path(path: &PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path.to_owned();
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}
