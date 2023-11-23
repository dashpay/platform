use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

use tonic_build::Builder;

fn main() {
    generate().expect("failed to compile protobuf definitions");

    println!("cargo:rerun-if-changed=./protos");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_SERDE");
}

struct MappingConfig {
    protobuf_file: PathBuf,
    out_dir: PathBuf,
    builder: Builder,
    proto_includes: Vec<PathBuf>,
}
/// Generate Rust definitions from Protobuf definitions
pub fn generate() -> Result<(), std::io::Error> {
    let core = MappingConfig::new(
        PathBuf::from("protos/core/v0/core.proto"),
        PathBuf::from("src/core/proto"),
    );
    core.generate().unwrap();

    let mut platform = MappingConfig::new(
        PathBuf::from("protos/platform/v0/platform.proto"),
        PathBuf::from("src/platform/proto"),
    );

    // Derive features for versioned messages
    //
    // "GetConsensusParamsRequest" is excluded as this message does not support proofs
    const VERSIONED_REQUESTS: [&str; 15] = [
        "GetDataContractHistoryRequest",
        "GetDataContractRequest",
        "GetDataContractsRequest",
        "GetDocumentsRequest",
        "GetIdentitiesByPublicKeyHashesRequest",
        "GetIdentitiesRequest",
        "GetIdentityBalanceAndRevisionRequest",
        "GetIdentityBalanceRequest",
        "GetIdentityByPublicKeyHashRequest",
        "GetIdentityKeysRequest",
        "GetIdentityRequest",
        "GetProofsRequest",
        "WaitForStateTransitionResultRequest",
        "GetProtocolVersionUpgradeStateRequest",
        "GetProtocolVersionUpgradeVoteStatusRequest",
    ];

    //  "GetConsensusParamsResponse" is excluded as this message does not support proofs
    const VERSIONED_RESPONSES: [&str; 16] = [
        "GetDataContractHistoryResponse",
        "GetDataContractResponse",
        "GetDataContractsResponse",
        "GetDocumentsResponse",
        "GetIdentitiesByPublicKeyHashesResponse",
        "GetIdentitiesResponse",
        "GetIdentityBalanceAndRevisionResponse",
        "GetIdentityBalanceResponse",
        "GetIdentityByPublicKeyHashResponse",
        "GetIdentityKeysResponse",
        "GetIdentityResponse",
        "GetProofsResponse",
        "WaitForStateTransitionResultResponse",
        "GetEpochsInfoResponse",
        "GetProtocolVersionUpgradeStateResponse",
        "GetProtocolVersionUpgradeVoteStatusResponse",
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

    platform.generate().unwrap();

    Ok(())
}

impl MappingConfig {
    fn new(protobuf_file: PathBuf, out_dir: PathBuf) -> Self {
        let protobuf_file = abs_path(&protobuf_file);
        let out_dir = abs_path(&out_dir);

        let builder = tonic_build::configure()
            .build_server(false)
            .out_dir(out_dir.clone())
            .protoc_arg("--experimental_allow_proto3_optional");

        #[cfg(feature = "client")]
        let builder = builder.build_client(true).build_transport(true);
        #[cfg(not(feature = "client"))]
        let builder = builder.build_client(false).build_transport(false);

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
