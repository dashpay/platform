use super::index::StoredIndex;
use super::nested::MAX_STORED_RULE_DECODE_DEPTH;
use super::rules::StoredRule;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property_constraints::PropertyConstraint;
use crate::data_contract::document_type::{ContestedIndexFieldMatch, Index, LazyRegex};
use crate::tests::fixtures::get_dpns_data_contract_fixture;
use bincode::error::DecodeError;
use platform_serialization::PLATFORM_BINCODE_CONFIG;
use platform_version::version::PlatformVersion;

/// `present` under `depth` negations.
fn negated(depth: usize) -> PropertyConstraint {
    let mut rule = PropertyConstraint::Present("name".to_string());
    for _ in 0..depth {
        rule = PropertyConstraint::Not(Box::new(rule));
    }
    rule
}

/// `rule` stored, encoded, and decoded with the untrusted decoder, as from a proof.
fn round_trip(rule: &PropertyConstraint) -> Result<PropertyConstraint, DecodeError> {
    let config = PLATFORM_BINCODE_CONFIG;
    let bytes = bincode::encode_to_vec(StoredRule::from(rule), config)
        .expect("expected the rule to encode");
    bincode::borrow_decode_from_slice_untrusted::<StoredRule, _>(&bytes, config)
        .map(|(stored, _)| stored.into())
}

/// A stored rule decodes as deep as the rule parser admits one, `not`s included, and no deeper:
/// a parse places each nested condition one level deeper, up to
/// `MAX_PROPERTY_CONSTRAINT_PARSE_DEPTH`.
#[test]
fn should_refuse_a_stored_rule_nested_deeper_than_the_rule_grammar_admits() {
    let deepest = negated(MAX_STORED_RULE_DECODE_DEPTH);
    assert_eq!(
        round_trip(&deepest).expect("expected it to decode"),
        deepest
    );

    let error = round_trip(&negated(MAX_STORED_RULE_DECODE_DEPTH + 1))
        .expect_err("expected decoding to refuse it");
    assert!(error.to_string().contains("rule nesting depth"), "{error}");

    // The depth is back to zero: the next decode on this thread works
    assert_eq!(
        round_trip(&deepest).expect("expected it to decode"),
        deepest
    );
}

/// A contested field's regex is compiled when its index is read back: a stored form that holds
/// one that does not compile is refused, where the lazy regex would have panicked on first
/// use. The schema parser refuses such a pattern at registration.
#[test]
fn should_refuse_a_stored_contested_regex_that_does_not_compile() {
    let contract =
        get_dpns_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
            .data_contract_owned();
    let (name, index) = contract
        .document_types()
        .values()
        .flat_map(|document_type| document_type.indexes().iter())
        .find(|(_, index)| index.contested_index.is_some())
        .map(|(name, index)| (name.clone(), index.clone()))
        .expect("expected DPNS to have a contested index");

    let read_back = Index::try_from(StoredIndex::from(&index)).expect("expected the index back");
    assert_eq!(read_back, index);

    let mut broken = index;
    let contested = broken
        .contested_index
        .as_mut()
        .expect("expected the contested index");
    let field = contested
        .field_matches
        .keys()
        .next()
        .expect("expected a contested field")
        .clone();
    contested.field_matches.insert(
        field,
        ContestedIndexFieldMatch::Regex(LazyRegex::new("(".to_string())),
    );
    let error = Index::try_from(StoredIndex::from(&broken))
        .expect_err("expected the stored regex to be refused");
    assert!(
        error.to_string().contains("does not compile"),
        "{name}: {error}"
    );
}

/// A type indexing a typed array reads back with the index structure its parse built: the
/// levels named after the array stamped with its `maxItems`, from the fan-out the stored form
/// holds resolved.
#[test]
fn should_read_back_an_indexed_typed_array_with_the_index_structure_its_parse_built() {
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::serialized_version::DocumentTypeInSerializationFormat;
    use crate::data_contract::document_type::DocumentType;
    use platform_value::{platform_value, Identifier};
    use std::collections::BTreeMap;

    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config");
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "hashtags": {
                "type": "array",
                "maxItems": 10,
                "items": { "type": "string", "minLength": 1, "maxLength": 61 },
                "position": 0
            },
            "language": { "type": "string", "maxLength": 2, "position": 1 }
        },
        "required": ["$createdAt"],
        "indices": [
            { "name": "byHashtagTime", "properties": [{ "hashtags": "asc" }, { "$createdAt": "asc" }] },
            { "name": "byLanguageHashtag", "properties": [{ "language": "asc" }, { "hashtags": "asc" }] }
        ],
        "additionalProperties": false
    });
    let contract_id = Identifier::new([1; 32]);
    let document_type = DocumentType::try_from_schema(
        contract_id,
        1,
        config.version(),
        "post",
        schema.clone(),
        None,
        &BTreeMap::new(),
        &config,
        true,
        &mut vec![],
        platform_version,
    )
    .expect("expected the type to parse");

    let read_back = DocumentTypeInSerializationFormat::try_from_document_type(
        document_type.as_ref(),
        None,
        platform_version,
    )
    .expect("expected the type to compile")
    .into_document_type("post", contract_id, schema)
    .expect("expected the type to read back");

    assert_eq!(read_back.index_structure(), document_type.index_structure());
    let index_structure = read_back.index_structure();
    let by_hashtag = index_structure
        .sub_levels()
        .get("hashtags")
        .expect("expected the hashtags level");
    assert_eq!(by_hashtag.indexed_array_max_items(), Some(10));
    let under_language = index_structure
        .sub_levels()
        .get("language")
        .and_then(|level| level.sub_levels().get("hashtags"))
        .expect("expected the hashtags level under language");
    assert_eq!(under_language.indexed_array_max_items(), Some(10));
}

/// Generation 0 rebuilds index structures with index structure builder 0: every protocol
/// version selecting it selects that builder, and one selecting another builder cannot write
/// it. A generation the version selects that this software does not know writes nothing.
#[test]
fn should_write_stored_generation_0_only_with_index_structure_builder_0() {
    use crate::data_contract::document_type::serialized_version::DocumentTypeInSerializationFormat;
    use crate::data_contract::schema::DataContractSchemaMethodsV0;
    use platform_version::version::PLATFORM_VERSIONS;

    for platform_version in PLATFORM_VERSIONS {
        let contract_versions = &platform_version.dpp.contract_versions;
        if contract_versions.contract_storage_document_type_version == 0 {
            assert_eq!(
                contract_versions
                    .document_type_versions
                    .index_versions
                    .index_levels_from_indices,
                0,
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }

    let contract =
        get_dpns_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
            .data_contract_owned();
    let document_type = contract
        .document_types()
        .values()
        .next()
        .expect("expected a document type");
    let compile = |platform_version: &PlatformVersion| {
        DocumentTypeInSerializationFormat::try_from_document_type(
            document_type.as_ref(),
            contract.schema_defs(),
            platform_version,
        )
    };
    compile(PlatformVersion::latest()).expect("expected the type to compile");

    let mut other_index_builder = PlatformVersion::latest().clone();
    other_index_builder
        .dpp
        .contract_versions
        .document_type_versions
        .index_versions
        .index_levels_from_indices = 1;
    let error = compile(&other_index_builder).expect_err("expected the write to be refused");
    assert!(
        error.to_string().contains("index_levels_from_indices"),
        "{error}"
    );

    let mut unknown_generation = PlatformVersion::latest().clone();
    unknown_generation
        .dpp
        .contract_versions
        .contract_storage_document_type_version = 1;
    assert!(compile(&unknown_generation).is_err());
}
