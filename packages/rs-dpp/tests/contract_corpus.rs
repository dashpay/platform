#[path = "fixtures/contract_corpus/mod.rs"]
mod contract_corpus;

use contract_corpus::{contracts, CorpusContract, Network};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::schema::validation_schema::SchemaKeyword;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::DataContract;
use dpp::platform_serialization::PLATFORM_BINCODE_CONFIG;
use dpp::platform_value::Value;
use dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted,
    PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
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
fn should_load_every_mainnet_corpus_contract_at_protocol_version_13() {
    // Mainnet ran protocol version 13 when the corpus was taken. A testnet contract written
    // at 14 may use what 13 refuses, so only mainnet's are loaded here
    let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
    let document_types: usize = contracts()
        .iter()
        .filter(|contract| contract.network == Network::Mainnet)
        .map(|contract| {
            let loaded = load(contract, platform_version);
            assert_eq!(loaded.version(), contract.version);
            loaded.document_types().len()
        })
        .sum();
    assert_eq!(document_types, 106);
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

#[test]
fn should_read_back_every_corpus_contract_from_its_compiled_stored_form() {
    let platform_version = PlatformVersion::latest();
    let mut document_types = 0;
    for contract in contracts() {
        let origin = format!("{:?} contract {}", contract.network, contract.id);
        let loaded = load(&contract, platform_version);
        let stored = loaded
            .serialize_to_storage_bytes(platform_version)
            .unwrap_or_else(|error| panic!("{origin} failed to compile: {error}"));
        assert_eq!(stored[0], 2, "{origin} is not stored in format version 2");

        // Fully validated, the stored form reads as its format version 1 does: its schemas
        // parsed and checked by today's rules, which a contract registered earlier may break
        let format_v1 = loaded
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("expected the contract to serialize");
        match (
            DataContract::versioned_deserialize_trusted(&stored, true, platform_version),
            DataContract::versioned_deserialize_trusted(&format_v1, true, platform_version),
        ) {
            (Ok(from_stored), Ok(from_format_v1)) => assert_eq!(
                from_stored, from_format_v1,
                "{origin} fully validated differs from its format version 1"
            ),
            (Err(from_stored), Err(from_format_v1)) => assert_eq!(
                from_stored.to_string(),
                from_format_v1.to_string(),
                "{origin} is refused otherwise than its format version 1"
            ),
            (from_stored, from_format_v1) => panic!(
                "{origin} fully validated: {from_stored:?}, its format version 1: {from_format_v1:?}"
            ),
        }

        // As Drive reads it; as a client reads it from a proof, which authenticates its
        // compiled types; and as the untrusted decoders read it, from its schemas
        let (stored_format, _) = dpp::bincode::decode_from_slice::<
            DataContractInSerializationFormat,
            _,
        >(&stored, PLATFORM_BINCODE_CONFIG)
        .unwrap_or_else(|error| panic!("{origin} stored form does not decode: {error}"));
        let read_backs = [
            DataContract::versioned_deserialize_trusted(&stored, false, platform_version),
            DataContract::try_from_platform_versioned(
                stored_format,
                false,
                &mut vec![],
                platform_version,
            ),
            DataContract::versioned_deserialize_untrusted(&stored, false, platform_version),
        ];
        let mut read_backs = read_backs.into_iter().map(|read_back| {
            read_back.unwrap_or_else(|error| panic!("{origin} failed to read back: {error}"))
        });
        let read_back = read_backs.next().expect("expected a read back");
        for other in read_backs {
            assert_eq!(other, read_back, "{origin} reads back differently");
        }
        assert_eq!(
            read_back, loaded,
            "{origin} changed through its stored form"
        );

        // What document type equality leaves out: an integer range's key type, and the
        // typed validation schema
        for (name, document_type) in loaded.document_types() {
            let read_back_type = read_back
                .document_type_for_name(name)
                .unwrap_or_else(|_| panic!("{origin} lost document type {name}"));
            for (index_name, index) in document_type.indexes() {
                let read_back_index = &read_back_type.indexes()[index_name];
                assert_eq!(
                    index.integer_range.as_ref().map(|range| range.key_type),
                    read_back_index
                        .integer_range
                        .as_ref()
                        .map(|range| range.key_type),
                    "{origin} {name} index {index_name} changed its integer range key type"
                );
            }
            assert_eq!(
                read_back
                    .document_type_validation_schema(name, platform_version)
                    .ok(),
                loaded
                    .document_type_validation_schema(name, platform_version)
                    .ok(),
                "{origin} {name} changed its validation schema"
            );
            document_types += 1;
        }

        // Stored again, the same bytes; in format version 1, the bytes the contract had
        assert_eq!(
            read_back
                .serialize_to_storage_bytes(platform_version)
                .expect("expected the contract to compile again"),
            stored,
            "{origin} compiles to other bytes the second time"
        );
        assert_eq!(
            read_back
                .serialize_to_bytes_with_platform_version(platform_version)
                .expect("expected the contract to serialize"),
            loaded
                .serialize_to_bytes_with_platform_version(platform_version)
                .expect("expected the contract to serialize"),
            "{origin} serializes to other format version 1 bytes after its stored form"
        );
    }
    assert_eq!(document_types, 106 + 1397);
}

/// A contract registered at the latest version keeps the native validator registration built of
/// each of its types, which its documents are checked with and its stored form holds the schema
/// of: it is the one the type's schema builds when nothing was kept, for every corpus contract
/// the latest version registers.
#[test]
fn should_keep_the_validation_schema_registration_builds_as_the_one_built_anew() {
    let platform_version = PlatformVersion::latest();
    let mut compared = 0;
    for contract in contracts() {
        // Registration fully validates; older contracts may fail the latest version's checks
        let Ok(registered) =
            DataContract::versioned_deserialize_trusted(&contract.bytes, true, platform_version)
        else {
            continue;
        };
        let read = load(&contract, platform_version);
        for name in registered.document_types().keys() {
            let kept = registered.document_type_validation_schema(name, platform_version);
            let built = read.document_type_validation_schema(name, platform_version);
            match (kept, built) {
                (Ok(kept), Ok(built)) => assert_eq!(
                    kept, built,
                    "{:?} contract {} document type {name}",
                    contract.network, contract.id
                ),
                (kept, built) => panic!(
                    "{:?} contract {} document type {name}: kept {kept:?}, built {built:?}",
                    contract.network, contract.id
                ),
            }
            compared += 1;
        }
    }
    assert!(
        compared > 0,
        "expected the latest version to register some corpus contract"
    );
}

/// The validation keywords of a schema the way the validation schema holds them: only the
/// keywords that constrain a value, at every node.
fn validation_keywords_of(schema: &Value) -> Value {
    let Value::Map(map) = schema else {
        return schema.clone();
    };
    Value::Map(
        map.iter()
            .filter_map(|(key, value)| {
                let keyword = key.as_text()?;
                if !SchemaKeyword::from_name(keyword).is_some_and(SchemaKeyword::constrains_a_value)
                {
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

/// A schema as the JSON Schema validator reads it: converted to JSON, so a repeated key
/// keeps its last value, bytes are arrays of integers and every integer is 64-bit. Two such
/// schemas compare equal whatever their key order, as `preserve_order` maps compare by key.
fn validating_json(value: &Value) -> serde_json::Value {
    value
        .try_to_validating_json()
        .expect("expected a schema the validator reads")
}

fn resolve<'a>(root: &'a Value, reference: &str) -> &'a Value {
    reference
        .strip_prefix('#')
        .and_then(|pointer| root.pointer(pointer))
        .unwrap_or_else(|| panic!("expected {reference} to resolve"))
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
                validating_json(&schema.root.to_value()),
                validating_json(&validation_keywords_of(&root)),
                "{:?} contract {} document type {name}",
                contract.network,
                contract.id
            );
            for (reference, node) in &schema.references {
                assert_eq!(
                    validating_json(&node.to_value()),
                    validating_json(&validation_keywords_of(resolve(&root, reference))),
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
