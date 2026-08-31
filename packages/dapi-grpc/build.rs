use std::{
    collections::HashSet,
    env,
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
};

use tonic_prost_build::Builder;

const SERDE_WITH_BYTES: &str = r#"#[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]"#;
const SERDE_WITH_BASE64: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::deserialization::vec_base64string"))]"#;
const SERDE_WITH_STRING: &str =
    r#"#[cfg_attr(feature = "serde", serde(with = "crate::deserialization::from_to_string"))]"#;

fn main() {
    let output_base = resolve_output_base().unwrap_or_else(|e| {
        eprintln!("[error] => resolve output base failed: {e}");
        std::process::exit(1);
    });
    println!(
        "cargo:rustc-env=DAPI_GRPC_OUT_DIR={}",
        output_base.display()
    );

    #[cfg(feature = "server")]
    generate_code(ImplType::Server, &output_base);
    #[cfg(feature = "client")]
    generate_code(ImplType::Client, &output_base);

    if std::env::var("CARGO_CFG_TARGET_ARCH")
        .unwrap_or_default()
        .eq("wasm32")
    {
        generate_code(ImplType::Wasm, &output_base);
    }
}

fn generate_code(typ: ImplType, output_base: &Path) {
    let core = MappingConfig::new(
        PathBuf::from("protos/core/v0/core.proto"),
        output_base.join("core"),
        &typ,
    );

    configure_core(core)
        .generate()
        .expect("generate core proto");

    let platform = MappingConfig::new(
        PathBuf::from("protos/platform/v0/platform.proto"),
        output_base.join("platform"),
        &typ,
    );

    configure_platform(platform)
        .generate()
        .expect("generate platform proto");

    let drive = MappingConfig::new(
        PathBuf::from("protos/drive/v0/drive.proto"),
        output_base.join("drive"),
        &typ,
    );

    configure_drive(drive)
        .generate()
        .expect("generate platform proto");

    println!("cargo:rerun-if-changed=./protos");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_SERDE");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_TRANSPORT");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");
    println!("cargo:rerun-if-env-changed=DAPI_GRPC_OUT_DIR");
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
    const VERSIONED_REQUESTS: [&str; 56] = [
        "GetDataContractHistoryRequest",
        "GetDataContractRequest",
        "GetDataContractsRequest",
        "GetDocumentHistoryRequest",
        "GetDocumentsRequest",
        "GetIdentitiesByPublicKeyHashesRequest",
        "GetIdentitiesRequest",
        "GetIdentitiesBalancesRequest",
        "GetIdentityNonceRequest",
        "GetIdentityContractNonceRequest",
        "GetIdentityBalanceAndRevisionRequest",
        "GetIdentityBalanceRequest",
        "GetIdentityByNonUniquePublicKeyHashRequest",
        "GetIdentityByPublicKeyHashRequest",
        "GetIdentityKeysRequest",
        "GetIdentityRequest",
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
        "GetIdentityTokenBalancesRequest",
        "GetIdentitiesTokenBalancesRequest",
        "GetTokenPerpetualDistributionLastClaimRequest",
        "GetIdentityTokenInfosRequest",
        "GetIdentitiesTokenInfosRequest",
        "GetTokenDirectPurchasePricesRequest",
        "GetTokenContractInfoRequest",
        "GetTokenStatusesRequest",
        "GetTokenPreProgrammedDistributionsRequest",
        "GetTokenTotalSupplyRequest",
        "GetGroupInfoRequest",
        "GetGroupInfosRequest",
        "GetGroupActionsRequest",
        "GetGroupActionSignersRequest",
        "GetFinalizedEpochInfosRequest",
        "GetAddressInfoRequest",
        "GetAddressesInfosRequest",
        "GetRecentAddressBalanceChangesRequest",
        "GetRecentCompactedAddressBalanceChangesRequest",
        "GetShieldedEncryptedNotesRequest",
        "GetShieldedAnchorsRequest",
        "GetMostRecentShieldedAnchorRequest",
        "GetShieldedPoolStateRequest",
        "GetShieldedNotesCountRequest",
        "GetShieldedNullifiersRequest",
    ];

    const PROOF_ONLY_VERSIONED_REQUESTS: [&str; 1] = ["GetAddressesTrunkStateRequest"];

    const MERK_PROOF_VERSIONED_REQUESTS: [&str; 1] = ["GetAddressesBranchStateRequest"];

    // The following responses are excluded as they don't support proofs:
    // - "GetConsensusParamsResponse"
    // - "GetStatusResponse"
    //
    // The following responses are excluded as they need custom proof handling:
    // - "GetIdentityByNonUniquePublicKeyHashResponse"
    //
    //  "GetEvonodesProposedEpochBlocksResponse" is used for 2 Requests
    const VERSIONED_RESPONSES: [&str; 54] = [
        "GetDataContractHistoryResponse",
        "GetDataContractResponse",
        "GetDataContractsResponse",
        "GetDocumentHistoryResponse",
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
        "GetIdentityTokenBalancesResponse",
        "GetIdentitiesTokenBalancesResponse",
        "GetTokenPerpetualDistributionLastClaimResponse",
        "GetIdentityTokenInfosResponse",
        "GetIdentitiesTokenInfosResponse",
        "GetTokenDirectPurchasePricesResponse",
        "GetTokenContractInfoResponse",
        "GetTokenStatusesResponse",
        "GetTokenPreProgrammedDistributionsResponse",
        "GetTokenTotalSupplyResponse",
        "GetGroupInfoResponse",
        "GetGroupInfosResponse",
        "GetGroupActionsResponse",
        "GetGroupActionSignersResponse",
        "GetFinalizedEpochInfosResponse",
        "GetAddressInfoResponse",
        "GetAddressesInfosResponse",
        "GetRecentAddressBalanceChangesResponse",
        "GetRecentCompactedAddressBalanceChangesResponse",
        "GetShieldedEncryptedNotesResponse",
        "GetShieldedAnchorsResponse",
        "GetMostRecentShieldedAnchorResponse",
        "GetShieldedPoolStateResponse",
        "GetShieldedNotesCountResponse",
        "GetShieldedNullifiersResponse",
    ];

    const PROOF_ONLY_VERSIONED_RESPONSES: [&str; 1] = ["GetAddressesTrunkStateResponse"];

    const MERK_PROOF_VERSIONED_RESPONSES: [&str; 1] = ["GetAddressesBranchStateResponse"];

    check_unique(&VERSIONED_REQUESTS).expect("VERSIONED_REQUESTS");
    check_unique(&VERSIONED_RESPONSES).expect("VERSIONED_RESPONSES");
    check_unique(&PROOF_ONLY_VERSIONED_REQUESTS).expect("PROOF_ONLY_VERSIONED_REQUESTS");
    check_unique(&PROOF_ONLY_VERSIONED_RESPONSES).expect("PROOF_ONLY_VERSIONED_RESPONSES");
    check_unique(&MERK_PROOF_VERSIONED_REQUESTS).expect("MERK_PROOF_VERSIONED_REQUESTS");
    check_unique(&MERK_PROOF_VERSIONED_RESPONSES).expect("MERK_PROOF_VERSIONED_RESPONSES");

    // Messages whose latest version is v1 — the macro needs to know
    // to generate match arms for both V0 and V1. Listed separately
    // so the default `grpc_versions(0)` loop below skips them.
    //
    // Adding a message here is the proto-side companion of:
    //   - Adding a `GetXxxRequestV1` / `GetXxxResponseV1` to the
    //     oneof in `platform.proto`.
    //   - Bumping the matching `FeatureVersionBounds.max_version`
    //     to 1 in `rs-platform-version`.
    //   - Implementing the v1 dispatch arm in `drive-abci`.
    const VERSIONED_AT_V1_REQUESTS: [&str; 1] = ["GetDocumentsRequest"];
    const VERSIONED_AT_V1_RESPONSES: [&str; 1] = ["GetDocumentsResponse"];

    // Derive VersionedGrpcMessage on requests
    for msg in VERSIONED_REQUESTS {
        if VERSIONED_AT_V1_REQUESTS.contains(&msg) {
            continue;
        }
        platform = platform
            .message_attribute(
                msg,
                r#"#[derive(::dash_platform_macros::VersionedGrpcMessage)]"#,
            )
            .message_attribute(msg, r#"#[grpc_versions(0)]"#);
    }
    for msg in VERSIONED_AT_V1_REQUESTS {
        platform = platform
            .message_attribute(
                msg,
                r#"#[derive(::dash_platform_macros::VersionedGrpcMessage)]"#,
            )
            .message_attribute(msg, r#"#[grpc_versions(1)]"#);
    }

    // Derive ProofOnlyVersionedGrpcMessage on requests
    for msg in PROOF_ONLY_VERSIONED_REQUESTS {
        platform = platform
            .message_attribute(
                msg,
                r#"#[derive(::dash_platform_macros::ProofOnlyVersionedGrpcMessage)]"#,
            )
            .message_attribute(msg, r#"#[grpc_versions(0)]"#);
    }

    // Derive VersionedGrpcMessage and VersionedGrpcResponse on responses
    for msg in VERSIONED_RESPONSES {
        if VERSIONED_AT_V1_RESPONSES.contains(&msg) {
            continue;
        }
        platform = platform
            .message_attribute(
                msg,
                r#"#[derive(::dash_platform_macros::VersionedGrpcMessage,::dash_platform_macros::VersionedGrpcResponse)]"#,
            )
            .message_attribute(msg, r#"#[grpc_versions(0)]"#);
    }
    for msg in VERSIONED_AT_V1_RESPONSES {
        platform = platform
            .message_attribute(
                msg,
                r#"#[derive(::dash_platform_macros::VersionedGrpcMessage,::dash_platform_macros::VersionedGrpcResponse)]"#,
            )
            .message_attribute(msg, r#"#[grpc_versions(1)]"#);
    }

    // Derive VersionedGrpcMessage and ProofOnlyVersionedGrpcResponse on responses
    for msg in PROOF_ONLY_VERSIONED_RESPONSES {
        platform = platform
            .message_attribute(
                msg,
                r#"#[derive(::dash_platform_macros::VersionedGrpcMessage,::dash_platform_macros::ProofOnlyVersionedGrpcResponse)]"#,
            )
            .message_attribute(msg, r#"#[grpc_versions(0)]"#);
    }

    // Derive VersionedGrpcMessage on merk proof requests
    for msg in MERK_PROOF_VERSIONED_REQUESTS {
        platform = platform
            .message_attribute(
                msg,
                r#"#[derive(::dash_platform_macros::VersionedGrpcMessage)]"#,
            )
            .message_attribute(msg, r#"#[grpc_versions(0)]"#);
    }

    // Derive VersionedGrpcMessage and MerkProofVersionedGrpcResponse on responses
    for msg in MERK_PROOF_VERSIONED_RESPONSES {
        platform = platform
            .message_attribute(
                msg,
                r#"#[derive(::dash_platform_macros::VersionedGrpcMessage,::dash_platform_macros::MerkProofVersionedGrpcResponse)]"#,
            )
            .message_attribute(msg, r#"#[grpc_versions(0)]"#);
    }

    // All messages can be mocked.
    let platform =
        platform.message_attribute(".", r#"#[derive( ::dash_platform_macros::Mockable)]"#);

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
        .field_attribute("nullifiers", SERDE_WITH_BASE64)
        // Get documents fields
        .field_attribute("data_contract_id", SERDE_WITH_BYTES)
        // V0 still ships CBOR for `where` / `order_by`; V1 ships
        // typed `repeated WhereClause` / `repeated OrderClause`
        // and doesn't need the `bytes`-shaped serde shim.
        .field_attribute("GetDocumentsRequestV0.where", SERDE_WITH_BYTES)
        .field_attribute("GetDocumentsRequestV0.order_by", SERDE_WITH_BYTES)
        // Proof fields
        .field_attribute("Proof.grovedb_proof", SERDE_WITH_BYTES)
        .field_attribute("Proof.quorum_hash", SERDE_WITH_BYTES)
        .field_attribute("Proof.signature", SERDE_WITH_BYTES)
        .field_attribute("Proof.block_id_hash", SERDE_WITH_BYTES);

    #[allow(clippy::let_and_return)]
    platform
}

fn configure_drive(drive: MappingConfig) -> MappingConfig {
    drive
        .message_attribute(".", r#"#[derive( ::dash_platform_macros::Mockable)]"#)
        .type_attribute(
            ".",
            r#"#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]"#,
        )
        .type_attribute(
            ".",
            r#"#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]"#,
        )
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
    let core = core.message_attribute(".", r#"#[derive(::dash_platform_macros::Mockable)]"#);

    // Serde support
    let core = core.type_attribute(
        ".",
        r#"#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]"#,
    );

    #[allow(clippy::let_and_return)]
    core
}

#[allow(unused)]
enum ImplType {
    Server,
    Client,
    Wasm,
}

impl ImplType {
    // Configure the builder based on the implementation type.
    pub fn configure(&self, builder: Builder) -> Builder {
        // The `transport` cargo feature controls whether generated clients get
        // the `connect()` convenience impls over tonic's own channel. Without
        // it, clients are still generated but stay generic over the caller's
        // transport. Never enabled for wasm32, where tonic transport does not
        // build. Note: cfg!(target_arch) in a build script reflects the HOST,
        // so the target must be read from CARGO_CFG_TARGET_ARCH.
        let transport = std::env::var("CARGO_FEATURE_TRANSPORT").is_ok()
            && std::env::var("CARGO_CFG_TARGET_ARCH").map(|arch| arch != "wasm32") == Ok(true);
        match self {
            Self::Server => builder
                .build_client(true)
                .build_server(true)
                .build_transport(transport),
            Self::Client => builder
                .build_client(true)
                .build_server(false)
                .build_transport(transport),
            Self::Wasm => builder
                .build_client(true)
                .build_server(false)
                .build_transport(false),
        }
    }

    /// Get the directory name for the implementation type.
    fn dirname(&self) -> String {
        match self {
            Self::Server => "server",
            Self::Client => "client",
            Self::Wasm => "wasm",
        }
        .to_string()
    }
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
    fn new(protobuf_file: PathBuf, out_dir: PathBuf, typ: &ImplType) -> Self {
        let protobuf_file = abs_path(&protobuf_file);

        // Depending on the features, we need to build the server, client or both.
        // We save these artifacts in separate directories to avoid overwriting the generated files
        // when another crate requires different features.
        let out_dir_suffix = typ.dirname();

        let out_dir = abs_path(&out_dir.join(out_dir_suffix));

        let builder = typ
            .configure(tonic_prost_build::configure())
            .out_dir(out_dir.clone())
            // Emit the FileDescriptorSet alongside the generated code so
            // consumers can enumerate the served rpcs at test time (e.g.
            // rs-dapi asserts its metrics allowlist covers every method).
            .file_descriptor_set_path(out_dir.join("descriptor.bin"))
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
    fn includes(mut self, includes: &[PathBuf]) -> Self {
        for include in includes {
            self.proto_includes.push(abs_path(include));
        }
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
            .compile_protos(&[self.protobuf_file], &self.proto_includes)
    }
}

fn abs_path(path: &PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path.to_owned();
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}

/// Resolve output base directory for generated files.
fn resolve_output_base() -> Result<PathBuf, String> {
    env::var("DAPI_GRPC_OUT_DIR")
        .map(PathBuf::from)
        .or_else(|_| env::var("OUT_DIR").map(|out_dir| PathBuf::from(out_dir).join("dapi_grpc")))
        .map_err(|_| {
            "OUT_DIR should be provided by Cargo; set DAPI_GRPC_OUT_DIR to override it".to_string()
        })
}
