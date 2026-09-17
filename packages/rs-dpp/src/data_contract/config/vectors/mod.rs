//! Canonical contract configuration vectors.
//!
//! `contract_config_vectors.json` is the one corpus every client mirror of
//! [`DataContractConfig`] replays. The cases are generated from the Rust
//! types by the tests in this module and pinned in the repository; each
//! mirror decodes the same file, must observe the same typed values
//! (`expect`), and must render the same canonical envelope (`canonical`).
//!
//! The surfaces that mirror the configuration wire shape are listed in
//! [`MIRROR_SURFACES`]. A change to the configuration types (a new
//! generation selected by a protocol version, a field added to or renamed in
//! a shipped generation) fails the guards below until the corpus carries a
//! case for it, and the failure message names every mirror that has to move
//! with it.
//!
//! Regenerate the corpus with
//!
//! ```text
//! DASH_WRITE_CONTRACT_CONFIG_VECTORS=1 cargo test -p dpp data_contract::config::vectors
//! ```
//!
//! and extend every mirror in the same change, or stack the mirror change on
//! it: the mobile CI jobs replay this file.

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::v0::{DataContractConfigGettersV0, DataContractConfigV0};
use crate::data_contract::config::v1::{DataContractConfigGettersV1, DataContractConfigV1};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::data_contract::DataContract;
use crate::version::{PlatformVersion, TryIntoPlatformVersioned, ALL_VERSIONS};
use platform_value::{platform_value, Identifier, Value};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value as JsonValue};
use std::collections::{BTreeMap, BTreeSet};

/// The pinned corpus, read at compile time.
const CORPUS: &str = include_str!("contract_config_vectors.json");

/// Where the writer puts a regenerated corpus.
const CORPUS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/data_contract/config/vectors/contract_config_vectors.json"
);

/// Set this variable to rewrite the corpus instead of asserting against it.
const WRITE_ENV: &str = "DASH_WRITE_CONTRACT_CONFIG_VECTORS";

/// Every surface that models the configuration wire shape.
const MIRROR_SURFACES: &str = "\
  - packages/rs-dpp/src/data_contract/config/vectors/contract_config_vectors.json (this corpus)
  - packages/rs-sdk-ffi/src/data_contract/json.rs (the FFI JSON emitters and their vector test)
  - the DataContractConfig TypeScript union in packages/wasm-dpp2/src/data_contract/model.rs
    and packages/wasm-dpp2/tests/unit/DataContractConfigVectors.spec.ts
  - DataContractConfig in packages/swift-sdk/Sources/SwiftDashSDK/DPP/DPPDataContract.swift
    and the parser projection in packages/swift-sdk/Sources/SwiftDashSDK/Core/Utils/DataContractParser.swift
  - packages/kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/contracts/DataContractConfig.kt
    and the entity projection in the example app's ContractDownloader.kt
  - docs/sdk/sdk-parity-manifest.json (capability contract.config_mirror)";

/// The protocol version the V1-envelope cases are generated at. The V1
/// contract envelope and the V1 configuration have been the defaults since
/// protocol version 9; pinning one version keeps the corpus stable across
/// protocol bumps, while the generation guard below walks every version.
const V1_ENVELOPE_PROTOCOL_VERSION: u32 = 14;

/// The protocol version the historic V0-envelope case is generated at and
/// replayed by every mirror: the last version whose tables still select the
/// V0 contract envelope.
const V0_ENVELOPE_PROTOCOL_VERSION: u32 = 8;

const CONTRACT_ID: [u8; 32] = [0x11; 32];
const OWNER_ID: [u8; 32] = [0x22; 32];

#[derive(Serialize, Deserialize, Debug)]
struct Corpus {
    description: String,
    cases: Vec<Case>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Case {
    name: String,
    /// The protocol version the case was generated at; every mirror decodes
    /// and re-encodes the case at this version.
    platform_version: u32,
    /// The contract a mirror decodes. Equals `canonical` except for the
    /// unknown-key case, which carries an extra configuration key.
    contract: JsonValue,
    /// The contract as Rust re-serializes it at `platform_version`.
    canonical: JsonValue,
    /// The configuration a mirror must observe after decoding `contract`,
    /// keyed by the wire names, the format tag included. A key present here
    /// that a mirror does not model fails that mirror's test.
    expect: JsonValue,
}

/// One generated case: the configuration under test and the version whose
/// tables choose the contract envelope it is written in.
struct CaseSpec {
    name: &'static str,
    platform_version: u32,
    config: DataContractConfig,
    /// Keys added to the `config` block of `contract` only; the canonical
    /// rendering must drop them.
    unknown_config_keys: Vec<(&'static str, JsonValue)>,
}

fn v1_all_set() -> DataContractConfigV1 {
    DataContractConfigV1 {
        can_be_deleted: true,
        readonly: true,
        keeps_history: true,
        documents_keep_history_contract_default: true,
        documents_mutable_contract_default: false,
        documents_can_be_deleted_contract_default: false,
        requires_identity_encryption_bounded_key: Some(StorageKeyRequirements::Unique),
        requires_identity_decryption_bounded_key: Some(
            StorageKeyRequirements::MultipleReferenceToLatest,
        ),
        sized_integer_types: false,
    }
}

fn specs() -> Vec<CaseSpec> {
    vec![
        CaseSpec {
            name: "v1_defaults",
            platform_version: V1_ENVELOPE_PROTOCOL_VERSION,
            config: DataContractConfigV1::default().into(),
            unknown_config_keys: vec![],
        },
        CaseSpec {
            name: "v1_all_set",
            platform_version: V1_ENVELOPE_PROTOCOL_VERSION,
            config: v1_all_set().into(),
            unknown_config_keys: vec![],
        },
        CaseSpec {
            name: "v0_defaults_in_v1_envelope",
            platform_version: V1_ENVELOPE_PROTOCOL_VERSION,
            config: DataContractConfigV0::default().into(),
            unknown_config_keys: vec![],
        },
        CaseSpec {
            name: "v0_key_requirements",
            platform_version: V1_ENVELOPE_PROTOCOL_VERSION,
            config: DataContractConfigV0 {
                requires_identity_encryption_bounded_key: Some(StorageKeyRequirements::Multiple),
                requires_identity_decryption_bounded_key: Some(StorageKeyRequirements::Unique),
                ..Default::default()
            }
            .into(),
            unknown_config_keys: vec![],
        },
        CaseSpec {
            name: "v0_envelope",
            platform_version: V0_ENVELOPE_PROTOCOL_VERSION,
            config: DataContractConfigV0::default().into(),
            unknown_config_keys: vec![],
        },
        CaseSpec {
            name: "v1_unknown_field_is_ignored",
            platform_version: V1_ENVELOPE_PROTOCOL_VERSION,
            config: v1_all_set().into(),
            unknown_config_keys: vec![("futurePolicy", json!({ "kind": 1 }))],
        },
    ]
}

/// One minimal document type, valid under every document meta-schema.
fn note_document_schemas() -> BTreeMap<String, Value> {
    BTreeMap::from([(
        "note".to_string(),
        platform_value!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "position": 0u32
                }
            },
            "additionalProperties": false
        }),
    )])
}

fn platform_version(protocol_version: u32) -> &'static PlatformVersion {
    PlatformVersion::get(protocol_version)
        .unwrap_or_else(|e| panic!("protocol version {protocol_version} is unknown: {e}"))
}

/// The contract envelope for a case, in the format the case's protocol
/// version selects as its default.
fn envelope(spec: &CaseSpec) -> DataContractInSerializationFormat {
    let format_version = platform_version(spec.platform_version)
        .dpp
        .contract_versions
        .contract_serialization_version
        .default_current_version;
    match format_version {
        0 => DataContractInSerializationFormatV0 {
            id: Identifier::new(CONTRACT_ID),
            config: spec.config,
            version: 1,
            owner_id: Identifier::new(OWNER_ID),
            schema_defs: None,
            document_schemas: note_document_schemas(),
        }
        .into(),
        1 => DataContractInSerializationFormatV1 {
            id: Identifier::new(CONTRACT_ID),
            config: spec.config,
            version: 1,
            owner_id: Identifier::new(OWNER_ID),
            schema_defs: None,
            document_schemas: note_document_schemas(),
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: Default::default(),
            tokens: Default::default(),
            keywords: Default::default(),
            description: None,
        }
        .into(),
        version => panic!(
            "{}",
            mirror_notice(&format!(
                "contract serialization format {version} has no generator in this corpus"
            ))
        ),
    }
}

/// The typed expectation for a decoded configuration, read through the enum
/// getters and keyed by the wire names, so every mirror compares its own
/// re-encoded configuration against it without a translation table.
fn expected_config(config: &DataContractConfig) -> JsonValue {
    let mut expect = Map::new();
    expect.insert(
        "$formatVersion".to_string(),
        json!(config.version().to_string()),
    );
    expect.insert("canBeDeleted".to_string(), json!(config.can_be_deleted()));
    expect.insert("readonly".to_string(), json!(config.readonly()));
    expect.insert("keepsHistory".to_string(), json!(config.keeps_history()));
    expect.insert(
        "documentsKeepHistoryContractDefault".to_string(),
        json!(config.documents_keep_history_contract_default()),
    );
    expect.insert(
        "documentsMutableContractDefault".to_string(),
        json!(config.documents_mutable_contract_default()),
    );
    expect.insert(
        "documentsCanBeDeletedContractDefault".to_string(),
        json!(config.documents_can_be_deleted_contract_default()),
    );
    expect.insert(
        "requiresIdentityEncryptionBoundedKey".to_string(),
        json!(config
            .requires_identity_encryption_bounded_key()
            .map(|requirement| requirement as u8)),
    );
    expect.insert(
        "requiresIdentityDecryptionBoundedKey".to_string(),
        json!(config
            .requires_identity_decryption_bounded_key()
            .map(|requirement| requirement as u8)),
    );
    if matches!(config, DataContractConfig::V1(_)) {
        // The V0 wire carries no `sizedIntegerTypes`; the enum getter reports
        // `false` for it, which is a Rust convenience, not a field.
        expect.insert(
            "sizedIntegerTypes".to_string(),
            json!(config.sized_integer_types()),
        );
    }
    JsonValue::Object(expect)
}

fn generate_case(spec: CaseSpec) -> Case {
    let canonical = serde_json::to_value(envelope(&spec)).expect("envelope serializes");
    let mut contract = canonical.clone();
    let config_block = contract["config"]
        .as_object_mut()
        .expect("the envelope carries a config object");
    for (key, value) in spec.unknown_config_keys {
        config_block.insert(key.to_string(), value);
    }
    Case {
        name: spec.name.to_string(),
        platform_version: spec.platform_version,
        contract,
        canonical,
        expect: expected_config(&spec.config),
    }
}

fn generate_corpus() -> Corpus {
    Corpus {
        description: format!(
            "Canonical contract configuration vectors generated by rs-dpp \
             (packages/rs-dpp/src/data_contract/config/vectors). Every mirror decodes \
             `contract` at `platformVersion`, must observe `expect`, and must re-serialize \
             to `canonical`. Regenerate with {WRITE_ENV}=1 cargo test -p dpp \
             data_contract::config::vectors"
        ),
        cases: specs().into_iter().map(generate_case).collect(),
    }
}

fn pinned_corpus() -> Corpus {
    serde_json::from_str(CORPUS).expect("the pinned corpus parses")
}

/// Renders a contract the way every emitter does: through the serialization
/// format the supplied version selects, then serde.
fn render(contract: &DataContract, platform_version: &PlatformVersion) -> JsonValue {
    let format: DataContractInSerializationFormat = contract
        .try_into_platform_versioned(platform_version)
        .expect("contract converts to its serialization format");
    serde_json::to_value(format).expect("serialization format serializes")
}

fn keys(value: &JsonValue) -> BTreeSet<String> {
    value
        .as_object()
        .expect("a JSON object")
        .keys()
        .cloned()
        .collect()
}

fn mirror_notice(what: &str) -> String {
    format!(
        "{what}\n\
         The contract configuration wire shape is mirrored outside rs-dpp; every mirror \
         must move with it:\n{MIRROR_SURFACES}\n\
         Regenerate the corpus with {WRITE_ENV}=1 cargo test -p dpp \
         data_contract::config::vectors, add a case for the new generation or field, and \
         extend every mirror in the same change (or stack the mirror change on it): the \
         mobile CI jobs replay the corpus."
    )
}

#[test]
fn should_round_trip_every_vector_through_rs_dpp() {
    for case in pinned_corpus().cases {
        let platform_version = platform_version(case.platform_version);
        let contract = DataContract::from_json(case.contract.clone(), true, platform_version)
            .unwrap_or_else(|e| panic!("{}: the vector does not decode: {e}", case.name));

        let config = contract.config();
        assert_eq!(
            expected_config(config),
            case.expect,
            "{}: the decoded configuration differs from expect",
            case.name
        );
        assert_eq!(
            config.version().to_string(),
            case.expect["$formatVersion"],
            "{}: the configuration generation differs from the expected tag",
            case.name
        );
        match case.expect.get("sizedIntegerTypes") {
            Some(JsonValue::Bool(expected)) => assert_eq!(
                config.sized_integer_types(),
                *expected,
                "{}: sizedIntegerTypes differs",
                case.name
            ),
            None => assert!(
                !config.sized_integer_types(),
                "{}: a configuration without sizedIntegerTypes reports it as false",
                case.name
            ),
            Some(other) => panic!("{}: sizedIntegerTypes is not a boolean: {other}", case.name),
        }

        assert_eq!(
            render(&contract, platform_version),
            case.canonical,
            "{}: the re-serialized contract differs from canonical",
            case.name
        );
        assert_eq!(
            case.expect, case.canonical["config"],
            "{}: expect and the canonical config block disagree",
            case.name
        );
    }
}

#[test]
fn should_keep_the_corpus_equal_to_the_generators_output() {
    let mut generated =
        serde_json::to_string_pretty(&generate_corpus()).expect("the corpus serializes");
    generated.push('\n');

    if std::env::var_os(WRITE_ENV).is_some() {
        std::fs::write(CORPUS_PATH, &generated).expect("the corpus is written");
        return;
    }

    assert!(
        CORPUS == generated,
        "{}",
        mirror_notice(
            "the pinned corpus differs from the generator's output; regenerate it and review \
             the diff"
        )
    );
}

#[test]
fn should_drop_unknown_config_keys_when_re_serializing() {
    let corpus = pinned_corpus();
    let case = corpus
        .cases
        .iter()
        .find(|case| keys(&case.contract["config"]) != keys(&case.canonical["config"]))
        .expect("the corpus carries a case whose contract config has a key canonical lacks");

    let extra: BTreeSet<String> = keys(&case.contract["config"])
        .difference(&keys(&case.canonical["config"]))
        .cloned()
        .collect();
    assert!(!extra.is_empty(), "{}: no extra key", case.name);
    for key in &extra {
        assert!(
            case.expect.get(key).is_none(),
            "{}: the unknown key {key} must not be expected of a mirror",
            case.name
        );
    }
}

#[test]
fn should_cover_every_default_config_generation_with_a_vector() {
    let corpus = pinned_corpus();

    // (tag, key set) of the configuration each protocol version creates by
    // default, derived from the types, with the versions that select it.
    let mut generations: BTreeMap<(String, BTreeSet<String>), Vec<u32>> = BTreeMap::new();
    for protocol_version in ALL_VERSIONS {
        let config = DataContractConfig::default_for_version(platform_version(protocol_version))
            .unwrap_or_else(|e| panic!("protocol version {protocol_version}: {e}"));
        let wire = serde_json::to_value(config).expect("the configuration serializes");
        let tag = wire["$formatVersion"]
            .as_str()
            .expect("the configuration carries a format tag")
            .to_string();
        generations
            .entry((tag, keys(&wire)))
            .or_default()
            .push(protocol_version);
    }

    for ((tag, wire_keys), protocol_versions) in generations {
        let covered = corpus.cases.iter().any(|case| {
            let default_tag =
                DataContractConfig::default_for_version(platform_version(case.platform_version))
                    .expect("the case's version has a default configuration")
                    .version()
                    .to_string();
            default_tag == tag
                && case.expect["$formatVersion"] == tag
                && keys(&case.expect) == wire_keys
        });
        assert!(
            covered,
            "{}",
            mirror_notice(&format!(
                "protocol versions {protocol_versions:?} create a configuration tagged \
                 \"{tag}\" with keys {wire_keys:?} by default, and no vector generated at \
                 one of those versions expects exactly that key set"
            ))
        );
    }
}

#[test]
fn should_pin_the_v0_and_v1_config_key_sets() {
    let v0: BTreeSet<String> = [
        "$formatVersion",
        "canBeDeleted",
        "readonly",
        "keepsHistory",
        "documentsKeepHistoryContractDefault",
        "documentsMutableContractDefault",
        "documentsCanBeDeletedContractDefault",
        "requiresIdentityEncryptionBoundedKey",
        "requiresIdentityDecryptionBoundedKey",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let mut v1 = v0.clone();
    v1.insert("sizedIntegerTypes".to_string());

    let v0_wire = serde_json::to_value(DataContractConfig::V0(DataContractConfigV0::default()))
        .expect("a V0 configuration serializes");
    assert!(
        keys(&v0_wire) == v0,
        "{}",
        mirror_notice(&format!(
            "the V0 configuration keys changed to {:?}",
            keys(&v0_wire)
        ))
    );

    let v1_wire = serde_json::to_value(DataContractConfig::V1(DataContractConfigV1::default()))
        .expect("a V1 configuration serializes");
    assert!(
        keys(&v1_wire) == v1,
        "{}",
        mirror_notice(&format!(
            "the V1 configuration keys changed to {:?}",
            keys(&v1_wire)
        ))
    );
}
