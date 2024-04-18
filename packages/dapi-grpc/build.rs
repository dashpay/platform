use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

use tonic_build::Builder;

fn main() {
    let core = MappingConfig::new(
        PathBuf::from("protos/core/v0/core.proto"),
        PathBuf::from("src/core/proto"),
    );

    configure_core(core)
        .generate()
        .expect("generate core proto");

    let platform = MappingConfig::new(
        PathBuf::from("protos/platform/v0/platform.proto"),
        PathBuf::from("src/platform/proto"),
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
    const VERSIONED_REQUESTS: [&str; 18] = [
        "GetDataContractHistoryRequest",
        "GetDataContractRequest",
        "GetDataContractsRequest",
        "GetDocumentsRequest",
        "GetIdentitiesByPublicKeyHashesRequest",
        "GetIdentitiesRequest",
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
    ];

    //  "GetConsensusParamsResponse" is excluded as this message does not support proofs
    const VERSIONED_RESPONSES: [&str; 19] = [
        "GetDataContractHistoryResponse",
        "GetDataContractResponse",
        "GetDataContractsResponse",
        "GetDocumentsResponse",
        "GetIdentitiesByPublicKeyHashesResponse",
        "GetIdentitiesResponse",
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
    ];

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
    platform = platform.message_attribute(".", r#"#[derive( ::dapi_grpc_macros::Mockable)]"#);

    #[cfg(feature = "serde")]
    let platform = platform
        .type_attribute(
            ".",
            r#"#[derive(::serde::Serialize, ::serde::Deserialize)]"#,
        )
        .type_attribute(".", r#"#[serde(rename_all = "snake_case")]"#)
        .field_attribute("id", r#"#[serde(with = "serde_bytes")]"#)
        .field_attribute("identity_id", r#"#[serde(with = "serde_bytes")]"#)
        .field_attribute(
            "ids",
            r#"#[serde(with = "crate::deserialization::vec_base64string")]"#,
        )
        .field_attribute(
            "ResponseMetadata.height",
            r#"#[serde(with = "crate::deserialization::from_to_string")]"#,
        )
        .field_attribute(
            "ResponseMetadata.time_ms",
            r#"#[serde(with = "crate::deserialization::from_to_string")]"#,
        )
        .field_attribute(
            "start_at_ms",
            r#"#[serde(with = "crate::deserialization::from_to_string")]"#,
        )
        .field_attribute("public_key_hash", r#"#[serde(with = "serde_bytes")]"#)
        .field_attribute(
            "public_key_hashes",
            r#"#[serde(with = "crate::deserialization::vec_base64string")]"#,
        )
        // Get documents fields
        .field_attribute("data_contract_id", r#"#[serde(with = "serde_bytes")]"#)
        .field_attribute("where", r#"#[serde(with = "serde_bytes")]"#)
        .field_attribute("order_by", r#"#[serde(with = "serde_bytes")]"#)
        // Proof fields
        .field_attribute("Proof.grovedb_proof", r#"#[serde(with = "serde_bytes")]"#)
        .field_attribute("Proof.quorum_hash", r#"#[serde(with = "serde_bytes")]"#)
        .field_attribute("Proof.signature", r#"#[serde(with = "serde_bytes")]"#)
        .field_attribute("Proof.block_id_hash", r#"#[serde(with = "serde_bytes")]"#);

    platform
}

fn configure_core(mut core: MappingConfig) -> MappingConfig {
    // All messages can be mocked.
    core = core.message_attribute(".", r#"#[derive( ::dapi_grpc_macros::Mockable)]"#);

    // Serde support
    #[cfg(feature = "serde")]
    let core = core.type_attribute(
        ".",
        r#"#[derive(::serde::Serialize, ::serde::Deserialize)]"#,
    );

    core
}

impl MappingConfig {
    fn new(protobuf_file: PathBuf, out_dir: PathBuf) -> Self {
        let protobuf_file = abs_path(&protobuf_file);
        let out_dir = abs_path(&out_dir);

        let build_server = cfg!(feature = "server");
        let build_client = cfg!(feature = "client");

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
