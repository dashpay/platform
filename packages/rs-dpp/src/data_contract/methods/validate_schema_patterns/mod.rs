mod v0;

use crate::data_contract::DocumentName;
use crate::prelude::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use v0::DefinitionsCount;

impl DataContract {
    /// Holds the regular expressions a contract's schemas carry to a bound, before anything
    /// compiles one: compiling takes time and memory in proportion to the compiled size, not
    /// to the pattern's length, and every pattern is compiled when a contract is parsed and
    /// again for the validator its documents are checked with.
    ///
    /// A pattern is every string under a `pattern` key and every `regexPattern` of a contested
    /// index's `fieldMatches`, wherever the document schemas and `$defs` hold one: the walk
    /// reads keys, not the grammar, so a value a `const`, `enum` or `default` holds under one
    /// of these keys counts too. A pattern in `$defs` counts once for each document type,
    /// since the schema check reads every definition for each type it checks. A contract may
    /// hold `max_contract_schema_patterns` patterns so counted, and each must compile within
    /// `max_schema_pattern_compiled_size` bytes, a `pattern` as the document validator
    /// compiles it and a `regexPattern` as the index parser does. A protocol version that
    /// predates the rule refuses nothing.
    pub fn validate_schema_patterns(
        document_schemas: &BTreeMap<DocumentName, Value>,
        schema_defs: Option<&BTreeMap<String, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_schema_patterns
        {
            None => Ok(SimpleConsensusValidationResult::new()),
            Some(0) => Ok(Self::validate_schema_patterns_v0(
                document_schemas,
                schema_defs,
                DefinitionsCount::OncePerDocumentType,
                false,
                platform_version,
            )),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_schema_patterns".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// What a mempool holds a contract's patterns to: the bound of
    /// [`Self::validate_schema_patterns`], at every protocol version, and no `format: regex`.
    ///
    /// A protocol version that predates the rule has no bound in consensus, and its blocks
    /// must keep accepting what they accepted. Its mempool applies generation 0 of the rule
    /// with its own system limits all the same, counting a pattern in `$defs` also once for
    /// each `$ref` that reaches it where that is more: the JSON Schema validator those
    /// versions check documents with compiles what a `$ref` names once for each `$ref`. That
    /// validator resolves a `$ref` through the `$id`s of a schema, so there a contract holding
    /// a pattern is refused with a `$ref` that is not `#/$defs/` and a definition's name, or
    /// an `$id` that is not a plain name.
    ///
    /// A schema declaring `format: regex` is refused at every protocol version: each value of
    /// such a property is compiled as a pattern when a document is checked, under no bound a
    /// contract can be held to. This is the one thing a mempool refuses that a block of a
    /// protocol version with the rule accepts, and it is to go once document validation checks
    /// such a value without compiling it.
    pub fn validate_schema_patterns_for_mempool(
        document_schemas: &BTreeMap<DocumentName, Value>,
        schema_defs: Option<&BTreeMap<String, Value>>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let definitions_count = match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_schema_patterns
        {
            None => DefinitionsCount::OncePerReference,
            Some(0) => DefinitionsCount::OncePerDocumentType,
            Some(version) => {
                return Err(ProtocolError::UnknownVersionMismatch {
                    method: "DataContract::validate_schema_patterns_for_mempool".to_string(),
                    known_versions: vec![0],
                    received: version,
                })
            }
        };
        Ok(Self::validate_schema_patterns_v0(
            document_schemas,
            schema_defs,
            definitions_count,
            true,
            platform_version,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::errors::DataContractError;
    use assert_matches::assert_matches;
    use platform_value::platform_value;

    /// A `pattern` and a `regexPattern` any limit admits.
    const SMALL: &str = "^[a-z]{1,20}$";
    /// Unicode letters, twenty times: over the limit in either dialect, though under the
    /// 5 and 10 MiB the parsers compile a pattern within.
    const LARGE: &str = r"^\pL{20}$";

    fn before_the_rule() -> &'static PlatformVersion {
        PlatformVersion::get(14).expect("expected protocol version 14")
    }

    /// A document type of `count` string properties, each with `pattern`.
    fn schema_with_patterns(count: usize, pattern: &str) -> Value {
        let properties: Vec<(Value, Value)> = (0..count)
            .map(|position| {
                (
                    Value::Text(format!("p{position}")),
                    platform_value!({
                        "type": "string",
                        "pattern": pattern,
                        "maxLength": 63,
                        "position": position as u64,
                    }),
                )
            })
            .collect();
        platform_value!({
            "type": "object",
            "properties": Value::Map(properties),
            "additionalProperties": false,
        })
    }

    /// A document type whose contested index lists `count` field matches of `pattern`.
    fn schema_with_field_matches(count: usize, pattern: &str) -> Value {
        let field_matches: Vec<Value> = (0..count)
            .map(|_| platform_value!({ "field": "label", "regexPattern": pattern }))
            .collect();
        platform_value!({
            "type": "object",
            "properties": {
                "label": { "type": "string", "maxLength": 63, "position": 0 },
            },
            "indices": [{
                "name": "byLabel",
                "properties": [{ "label": "asc" }],
                "unique": true,
                "contested": { "fieldMatches": field_matches, "resolution": 0 },
            }],
            "additionalProperties": false,
        })
    }

    fn schemas(schema: Value) -> BTreeMap<DocumentName, Value> {
        BTreeMap::from([("item".to_string(), schema)])
    }

    fn refusal_message(result: SimpleConsensusValidationResult) -> String {
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::InvalidContractStructure(message)
            ))] => message.clone()
        )
    }

    fn validate(
        document_schemas: &BTreeMap<DocumentName, Value>,
        schema_defs: Option<&BTreeMap<String, Value>>,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        DataContract::validate_schema_patterns(document_schemas, schema_defs, platform_version)
            .expect("expected the rule to run")
    }

    fn validate_for_mempool(
        document_schemas: &BTreeMap<DocumentName, Value>,
        schema_defs: Option<&BTreeMap<String, Value>>,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        DataContract::validate_schema_patterns_for_mempool(
            document_schemas,
            schema_defs,
            platform_version,
        )
        .expect("expected the rule to run")
    }

    #[test]
    fn should_hold_a_contract_to_the_most_patterns_it_may_hold() {
        let platform_version = PlatformVersion::latest();
        let most = usize::from(platform_version.system_limits.max_contract_schema_patterns);

        let at_the_limit = schemas(schema_with_patterns(most, SMALL));
        assert!(validate(&at_the_limit, None, platform_version).is_valid());

        let over_the_limit = schemas(schema_with_patterns(most + 1, SMALL));
        let message = refusal_message(validate(&over_the_limit, None, platform_version));
        assert!(message.contains("at most 64 patterns"), "{message}");
    }

    #[test]
    fn should_count_the_patterns_of_every_document_type_and_of_the_defs_together() {
        let platform_version = PlatformVersion::latest();
        let most = usize::from(platform_version.system_limits.max_contract_schema_patterns);
        let document_schemas = BTreeMap::from([
            ("a".to_string(), schema_with_patterns(most / 2, SMALL)),
            (
                "b".to_string(),
                schema_with_field_matches(most - most / 2, SMALL),
            ),
        ]);
        assert!(validate(&document_schemas, None, platform_version).is_valid());

        let schema_defs = BTreeMap::from([(
            "label".to_string(),
            platform_value!({ "type": "string", "pattern": SMALL }),
        )]);
        let message = refusal_message(validate(
            &document_schemas,
            Some(&schema_defs),
            platform_version,
        ));
        assert!(message.contains("at most 64 patterns"), "{message}");
    }

    #[test]
    fn should_refuse_too_many_patterns_without_compiling_one() {
        // None of these compiles, and the refusal is for their number
        let document_schemas = schemas(schema_with_field_matches(65, "("));
        let message = refusal_message(validate(&document_schemas, None, PlatformVersion::latest()));
        assert!(message.contains("at most 64 patterns"), "{message}");
    }

    #[test]
    fn should_refuse_a_pattern_that_compiles_to_more_than_the_size_limit() {
        let platform_version = PlatformVersion::latest();
        for document_schemas in [
            schemas(schema_with_patterns(1, LARGE)),
            schemas(schema_with_field_matches(1, LARGE)),
        ] {
            let message = refusal_message(validate(&document_schemas, None, platform_version));
            assert!(
                message.contains(LARGE) && message.contains("compiles within 131072 bytes"),
                "{message}"
            );
        }
        // A long pattern compiles to little: the limit is on the compiled size
        let long = format!("^({})$", vec!["[a-z]{1,20}"; 40].join("|"));
        assert!(validate(
            &schemas(schema_with_patterns(1, &long)),
            None,
            platform_version
        )
        .is_valid());
    }

    #[test]
    fn should_compile_each_pattern_as_what_reads_it_does() {
        // The document validator reads `\w` as the ASCII word characters, the index parser as
        // every Unicode one, which compiles to far more
        let platform_version = PlatformVersion::latest();
        let words = r"^\w{20}$";
        assert!(validate(
            &schemas(schema_with_patterns(1, words)),
            None,
            platform_version
        )
        .is_valid());
        let message = refusal_message(validate(
            &schemas(schema_with_field_matches(1, words)),
            None,
            platform_version,
        ));
        assert!(message.contains("`regexPattern`"), "{message}");
    }

    #[test]
    fn should_refuse_a_pattern_that_is_not_a_regular_expression() {
        let message = refusal_message(validate(
            &schemas(schema_with_patterns(1, "(")),
            None,
            PlatformVersion::latest(),
        ));
        assert!(message.contains("`pattern` `(`"), "{message}");
    }

    #[test]
    fn should_walk_a_schema_nested_deeper_than_the_stack_holds() {
        let mut schema = platform_value!({ "pattern": "(" });
        for _ in 0..200_000 {
            schema = Value::Array(vec![schema]);
        }
        let document_schemas = schemas(schema);
        let message = refusal_message(validate(&document_schemas, None, PlatformVersion::latest()));
        assert!(message.contains("`pattern` `(`"), "{message}");
        // Dropping a value this deep recurses as deep
        std::mem::forget(document_schemas);
    }

    #[test]
    fn should_refuse_nothing_in_consensus_before_protocol_version_15() {
        for document_schemas in [
            schemas(schema_with_patterns(65, SMALL)),
            schemas(schema_with_patterns(1, LARGE)),
            schemas(schema_with_field_matches(1, "(")),
        ] {
            assert!(validate(&document_schemas, None, before_the_rule()).is_valid());
        }
    }

    #[test]
    fn should_hold_the_mempool_to_the_bound_before_consensus_has_it() {
        let message = refusal_message(validate_for_mempool(
            &schemas(schema_with_patterns(65, SMALL)),
            None,
            before_the_rule(),
        ));
        assert!(message.contains("at most 64 patterns"), "{message}");

        let message = refusal_message(validate_for_mempool(
            &schemas(schema_with_field_matches(1, LARGE)),
            None,
            before_the_rule(),
        ));
        assert!(
            message.contains("compiles within 131072 bytes"),
            "{message}"
        );

        assert!(validate_for_mempool(
            &schemas(schema_with_patterns(64, SMALL)),
            None,
            before_the_rule()
        )
        .is_valid());
    }

    #[test]
    fn should_keep_a_regex_format_out_of_the_mempool_and_leave_it_to_consensus() {
        let document_schemas = schemas(platform_value!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "format": "regex",
                    "maxLength": 63,
                    "position": 0,
                },
            },
            "additionalProperties": false,
        }));

        for platform_version in [before_the_rule(), PlatformVersion::latest()] {
            let message = refusal_message(validate_for_mempool(
                &document_schemas,
                None,
                platform_version,
            ));
            assert!(message.contains("`format: regex`"), "{message}");
            assert!(validate(&document_schemas, None, platform_version).is_valid());
        }
    }

    /// `count` document types, each with one property that is a `$ref` to `definition`.
    fn schemas_referring_to(count: usize, definition: &str) -> BTreeMap<DocumentName, Value> {
        let reference = format!("#/$defs/{definition}");
        (0..count)
            .map(|index| {
                (
                    format!("item{index}"),
                    platform_value!({
                        "type": "object",
                        "properties": {
                            "label": { "$ref": reference.clone(), "position": 0 },
                        },
                        "additionalProperties": false,
                    }),
                )
            })
            .collect()
    }

    /// A definition `labels` holding `count` string properties, each with a pattern.
    fn definitions_with_patterns(count: usize) -> BTreeMap<String, Value> {
        BTreeMap::from([("labels".to_string(), schema_with_patterns(count, SMALL))])
    }

    #[test]
    fn should_count_a_pattern_in_the_defs_for_each_document_type() {
        // The schema check reads every definition for each type, referred to or not
        let platform_version = PlatformVersion::latest();
        let schema_defs = definitions_with_patterns(16);

        let four_types = schemas_referring_to(4, "labels");
        assert!(validate(&four_types, Some(&schema_defs), platform_version).is_valid());

        let five_types = schemas_referring_to(5, "labels");
        let message = refusal_message(validate(&five_types, Some(&schema_defs), platform_version));
        assert!(
            message.contains("counted for each document type"),
            "{message}"
        );

        let mut never_referring = schemas(schema_with_patterns(0, SMALL));
        never_referring.extend(schemas_referring_to(4, "labels"));
        let message = refusal_message(validate(
            &never_referring,
            Some(&schema_defs),
            platform_version,
        ));
        assert!(
            message.contains("counted for each document type"),
            "{message}"
        );
    }

    #[test]
    fn should_count_the_patterns_of_the_defs_once_in_a_contract_without_document_types() {
        let platform_version = PlatformVersion::latest();
        let no_document_types = BTreeMap::new();
        assert!(validate(
            &no_document_types,
            Some(&definitions_with_patterns(64)),
            platform_version
        )
        .is_valid());
        let message = refusal_message(validate(
            &no_document_types,
            Some(&definitions_with_patterns(65)),
            platform_version,
        ));
        assert!(message.contains("at most 64 patterns"), "{message}");
    }

    /// One document type whose `count` properties are each a `$ref` to `definition`.
    fn schema_referring_often_to(count: usize, definition: &str) -> BTreeMap<DocumentName, Value> {
        let reference = format!("#/$defs/{definition}");
        let properties: Vec<(Value, Value)> = (0..count)
            .map(|position| {
                (
                    Value::Text(format!("p{position}")),
                    platform_value!({
                        "$ref": reference.clone(),
                        "position": position as u64,
                    }),
                )
            })
            .collect();
        schemas(platform_value!({
            "type": "object",
            "properties": Value::Map(properties),
            "additionalProperties": false,
        }))
    }

    #[test]
    fn should_count_a_pattern_for_each_ref_reaching_it_in_the_mempool_before_the_rule() {
        // The JSON Schema validator compiles a definition once for each `$ref` to it; the
        // native one, once for the type
        let schema_defs = definitions_with_patterns(1);
        let within = schema_referring_often_to(64, "labels");
        let over = schema_referring_often_to(65, "labels");

        assert!(validate_for_mempool(&within, Some(&schema_defs), before_the_rule()).is_valid());
        let message = refusal_message(validate_for_mempool(
            &over,
            Some(&schema_defs),
            before_the_rule(),
        ));
        assert!(message.contains("for each `$ref` to it"), "{message}");

        assert!(validate(&over, Some(&schema_defs), PlatformVersion::latest()).is_valid());
    }

    #[test]
    fn should_count_a_pattern_through_the_refs_of_the_definitions_before_the_rule() {
        // `d0` refers twice to `d1`, which refers twice to `d2`, and so on to `d7`, which
        // holds the one pattern: 128 `$ref`s reach it
        let mut schema_defs = definitions_with_patterns(0);
        for level in 0..7 {
            let next = format!("#/$defs/d{}", level + 1);
            schema_defs.insert(
                format!("d{level}"),
                platform_value!({
                    "type": "object",
                    "properties": {
                        "a": { "$ref": next.clone(), "position": 0 },
                        "b": { "$ref": next, "position": 1 },
                    },
                    "additionalProperties": false,
                }),
            );
        }
        schema_defs.insert(
            "d7".to_string(),
            platform_value!({ "type": "string", "pattern": SMALL, "maxLength": 63 }),
        );

        let from_the_top = schema_referring_often_to(1, "d0");
        let message = refusal_message(validate_for_mempool(
            &from_the_top,
            Some(&schema_defs),
            before_the_rule(),
        ));
        assert!(message.contains("for each `$ref` to it"), "{message}");

        let from_half_way = schema_referring_often_to(1, "d2");
        assert!(
            validate_for_mempool(&from_half_way, Some(&schema_defs), before_the_rule()).is_valid()
        );
    }

    #[test]
    fn should_stop_at_definitions_referring_to_each_other() {
        let cycle = |pattern: Option<&str>| {
            let mut leaf = platform_value!({ "type": "string", "maxLength": 63 });
            if let Some(pattern) = pattern {
                leaf.insert("pattern".to_string(), Value::Text(pattern.to_string()))
                    .expect("expected a map");
            }
            BTreeMap::from([
                (
                    "a".to_string(),
                    platform_value!({ "allOf": [{ "$ref": "#/$defs/b" }, leaf] }),
                ),
                (
                    "b".to_string(),
                    platform_value!({ "allOf": [{ "$ref": "#/$defs/a" }] }),
                ),
            ])
        };
        let document_schemas = schema_referring_often_to(1, "a");

        // Nothing to compile however far the cycle is followed
        assert!(
            validate_for_mempool(&document_schemas, Some(&cycle(None)), before_the_rule())
                .is_valid()
        );
        // A pattern compiled once more each time round
        let message = refusal_message(validate_for_mempool(
            &document_schemas,
            Some(&cycle(Some(SMALL))),
            before_the_rule(),
        ));
        assert!(message.contains("for each `$ref` to it"), "{message}");
    }

    #[test]
    fn should_keep_an_id_that_may_move_a_ref_out_of_the_mempool_before_the_rule() {
        // The JSON Schema validator looks a `$ref` up among the `$id`s of a schema before it
        // reads it as a pointer: each `$ref` to `plain` reaches `holder`, though by its
        // pointer it reaches no pattern
        let holder_and_plain = |id: &str| {
            let mut holder = schema_with_patterns(1, SMALL);
            holder
                .insert("$id".to_string(), Value::Text(id.to_string()))
                .expect("expected a map");
            BTreeMap::from([
                ("holder".to_string(), holder),
                ("plain".to_string(), platform_value!({ "type": "integer" })),
            ])
        };
        let document_schemas = schema_referring_often_to(65, "plain");

        let message = refusal_message(validate_for_mempool(
            &document_schemas,
            Some(&holder_and_plain("#/$defs/plain")),
            before_the_rule(),
        ));
        assert!(
            message.contains("`$id` that is not a plain name"),
            "{message}"
        );

        // A plain name is what a keyword's data holds under this key, and moves nothing
        assert!(validate_for_mempool(
            &document_schemas,
            Some(&holder_and_plain("targetId")),
            before_the_rule()
        )
        .is_valid());

        // Nothing to compile, wherever a `$ref` leads
        let mut no_pattern = holder_and_plain("#/$defs/plain");
        no_pattern.insert(
            "holder".to_string(),
            platform_value!({ "$id": "#/$defs/plain", "type": "string" }),
        );
        assert!(
            validate_for_mempool(&document_schemas, Some(&no_pattern), before_the_rule())
                .is_valid()
        );

        // The validation schema of protocol version 15 refuses such an `$id` itself
        assert!(validate_for_mempool(
            &document_schemas,
            Some(&holder_and_plain("#/$defs/plain")),
            PlatformVersion::latest()
        )
        .is_valid());
    }

    #[test]
    fn should_keep_a_ref_that_is_no_plain_pointer_out_of_the_mempool_before_the_rule() {
        let schema_defs = definitions_with_patterns(1);
        for reference in [
            // `labels`, percent-encoded, and with a tab a URL parser drops
            "#/$defs/%6Cabels",
            "#/$defs/lab\tels",
            " #/$defs/labels",
            // The document schema itself, and a part of it
            "#",
            "#/properties/label",
            "#/$defs/labels/",
            "https://example.com/schema#/$defs/labels",
        ] {
            let document_schemas = schemas(platform_value!({
                "type": "object",
                "properties": {
                    "label": { "$ref": reference, "position": 0 },
                },
                "additionalProperties": false,
            }));
            let message = refusal_message(validate_for_mempool(
                &document_schemas,
                Some(&schema_defs),
                before_the_rule(),
            ));
            assert!(
                message.contains("`$ref` that is not `#/$defs/`"),
                "{reference}: {message}"
            );
            assert!(
                validate_for_mempool(&document_schemas, None, before_the_rule()).is_valid(),
                "{reference}"
            );
        }

        let into_a_definition = schemas(platform_value!({
            "type": "object",
            "properties": {
                "label": { "$ref": "#/$defs/labels/properties/p0", "position": 0 },
            },
            "additionalProperties": false,
        }));
        assert!(
            validate_for_mempool(&into_a_definition, Some(&schema_defs), before_the_rule())
                .is_valid()
        );
    }
}
