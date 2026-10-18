#[path = "fixtures/contract_corpus/mod.rs"]
mod contract_corpus;

use contract_corpus::{contracts, CorpusContract, Network};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::schema::validation_schema::VALIDATION_KEYWORDS;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::DataContract;
use dpp::platform_value::Value;
use dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted,
    PlatformSerializableWithPlatformVersion,
};
use dpp::version::PlatformVersion;

/// Loads a stored contract the way Drive does: trusted bytes, no full validation.
fn load(contract: &CorpusContract, platform_version: &PlatformVersion) -> DataContract {
    DataContract::versioned_deserialize_trusted(&contract.bytes, false, platform_version)
        .unwrap_or_else(|error| {
            panic!(
                "{:?} contract {} failed to load at protocol version {}: {error}",
                contract.network, contract.id, platform_version.protocol_version
            )
        })
}

#[test]
fn should_hold_every_mainnet_contract_and_every_distinct_testnet_schema() {
    let corpus = contracts();
    let count = |network| corpus.iter().filter(|c| c.network == network).count();
    assert_eq!(count(Network::Mainnet), 55);
    assert_eq!(count(Network::Testnet), 694);
}

#[test]
fn should_load_every_corpus_contract_at_protocol_version_14() {
    let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
    let document_types: usize = contracts()
        .iter()
        .map(|contract| {
            let loaded = load(contract, platform_version);
            assert_eq!(
                loaded
                    .id()
                    .to_string(dpp::platform_value::string_encoding::Encoding::Base58),
                contract.id
            );
            assert_eq!(loaded.version(), contract.version);
            loaded.document_types().len()
        })
        .sum();
    assert_eq!(document_types, 106 + 1397);
}

#[test]
fn should_load_every_corpus_contract_at_the_latest_protocol_version() {
    let platform_version = PlatformVersion::latest();
    for contract in contracts() {
        let loaded = load(&contract, platform_version);
        assert_eq!(loaded.version(), contract.version);
    }
}

#[test]
fn should_reload_every_corpus_contract_reserialized_at_the_latest_protocol_version() {
    let platform_version = PlatformVersion::latest();
    for contract in contracts() {
        let loaded = load(&contract, platform_version);
        let bytes = loaded
            .serialize_to_bytes_with_platform_version(platform_version)
            .unwrap_or_else(|error| {
                panic!(
                    "{:?} contract {} failed to serialize: {error}",
                    contract.network, contract.id
                )
            });
        let reloaded = DataContract::versioned_deserialize_trusted(&bytes, false, platform_version)
            .unwrap_or_else(|error| {
                panic!(
                    "{:?} contract {} failed to reload: {error}",
                    contract.network, contract.id
                )
            });
        assert_eq!(
            reloaded, loaded,
            "{:?} contract {} changed through a serialization round trip",
            contract.network, contract.id
        );
    }
}

/// The validation keywords of a schema the way the validation schema holds them: only the
/// keywords of `VALIDATION_KEYWORDS`, at every node.
fn validation_keywords_of(schema: &Value) -> Value {
    let Value::Map(map) = schema else {
        return schema.clone();
    };
    Value::Map(
        map.iter()
            .filter_map(|(key, value)| {
                let keyword = key.as_text()?;
                if !VALIDATION_KEYWORDS.contains(&keyword) {
                    return None;
                }
                let value = match (keyword, value) {
                    ("properties", Value::Map(properties)) => Value::Map(
                        properties
                            .iter()
                            .map(|(name, node)| (name.clone(), validation_keywords_of(node)))
                            .collect(),
                    ),
                    ("items" | "contains", node) => validation_keywords_of(node),
                    (_, value) => value.clone(),
                };
                Some((key.clone(), value))
            })
            .collect(),
    )
}

/// A schema with its maps sorted by key and every integer widened, so two schemas compare
/// by what they say rather than by key order or integer width.
fn normalized(value: &Value) -> Value {
    match value {
        Value::Map(map) => {
            let mut map: Vec<_> = map
                .iter()
                .map(|(key, value)| (key.clone(), normalized(value)))
                .collect();
            map.sort_by(|(left, _), (right, _)| left.as_text().cmp(&right.as_text()));
            Value::Map(map)
        }
        Value::Array(elements) => Value::Array(elements.iter().map(normalized).collect()),
        value if value.is_integer() => Value::I128(value.as_integer().expect("an integer")),
        value => value.clone(),
    }
}

fn resolve<'a>(root: &'a Value, reference: &str) -> &'a Value {
    reference
        .strip_prefix("#/")
        .expect("expected a JSON pointer")
        .split('/')
        .fold(root, |target, segment| {
            let segment = segment.replace("~1", "/").replace("~0", "~");
            target
                .as_map()
                .and_then(|map| {
                    map.iter()
                        .find(|(key, _)| key.as_text() == Some(segment.as_str()))
                        .map(|(_, value)| value)
                })
                .unwrap_or_else(|| panic!("expected {reference} to resolve"))
        })
}

#[test]
fn should_build_a_validation_schema_holding_every_validation_keyword_of_every_corpus_document_type()
{
    let platform_version = PlatformVersion::latest();
    let mut built = 0;
    let mut with_references = 0;
    for contract in contracts() {
        let loaded = load(&contract, platform_version);
        for (name, document_type) in loaded.document_types() {
            let schema = loaded
                .document_type_validation_schema(name, platform_version)
                .unwrap_or_else(|error| {
                    panic!(
                        "{:?} contract {} document type {name}: {error}",
                        contract.network, contract.id
                    )
                });

            // What the document validator compiles today.
            let root = DocumentType::enrich_with_base_schema(
                document_type.schema().clone(),
                loaded.schema_defs().map(|defs| Value::from(defs.clone())),
                platform_version,
            )
            .expect("expected the base schema to apply");
            let root =
                DocumentType::expand_property_type_shorthands(&root, false, platform_version)
                    .expect("expected the shorthands to expand")
                    .unwrap_or(root);

            assert_eq!(
                normalized(&schema.root.to_value()),
                normalized(&validation_keywords_of(&root)),
                "{:?} contract {} document type {name}",
                contract.network,
                contract.id
            );
            for (reference, node) in &schema.references {
                assert_eq!(
                    normalized(&node.to_value()),
                    normalized(&validation_keywords_of(resolve(&root, reference))),
                    "{:?} contract {} document type {name} reference {reference}",
                    contract.network,
                    contract.id
                );
            }
            if !schema.references.is_empty() {
                with_references += 1;
            }
            built += 1;
        }
    }
    assert_eq!(built, 106 + 1397);
    assert!(
        with_references > 0,
        "expected some corpus document type to use a $ref"
    );
}
