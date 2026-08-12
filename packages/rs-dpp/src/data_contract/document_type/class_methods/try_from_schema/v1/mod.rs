//! Document-type parser **generation 1** — protocol versions 9 through 11,
//! and the core that generation 2 (protocol versions 12 and 13) delegates to.
//!
//! The parsing steps themselves live in [`super::common`]; this module is the
//! thin driver that names generation 1's grammar.

use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

use super::common;

// Re-exported into scope for this module's `#[cfg(test)] mod tests`, which
// reaches them through `use super::*`.
#[cfg(test)]
use crate::consensus::basic::data_contract::InvalidDocumentTypeNameError;
#[cfg(test)]
use crate::consensus::basic::BasicError;
#[cfg(test)]
use crate::consensus::ConsensusError;
#[cfg(test)]
use crate::identity::SecurityLevel;

impl DocumentTypeV1 {
    /// Parses a document type schema through the generation-1 grammar.
    ///
    /// # This entry point serves two generations
    ///
    /// Generation 1 reaches it directly (`try_from_schema: 1`, protocol
    /// versions 9–11, `document_type_schema: 0`), and generation 2 reaches it
    /// by delegation from [`DocumentTypeV2::try_from_schema`] (`try_from_schema:
    /// 2`, protocol versions 12 and 13, `document_type_schema` 1 and 2
    /// respectively).
    ///
    /// That is why the `keeps*History` gate below is a real table read rather
    /// than a constant: the flags are absent for generation 1 and for PV12, and
    /// present from PV13 — the behavior genuinely varies across the protocol
    /// versions this one function serves. It is the only version read that
    /// still *changes* a grammar (generation 0 keeps the same read, but no
    /// table selecting it ever names a schema above 0), and it is pinned by this module's
    /// `keeps_history_flags_version_gating` tests, which exercise this function
    /// at PV11 (ignored) and PV13 (parsed). The same read also selects the
    /// meta-schema, pinned across schema 0/1/2 by the `document_meta_schema_version`
    /// tests at PV1 and PV12.
    ///
    /// Everything that does *not* vary lives in
    /// [`common::parse_document_type_core`]; the constants passed below are
    /// generation 1's grammar.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_from_schema(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        name: &str,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool, // we don't need to validate if loaded from state
        validation_operations: &mut impl Extend<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // The version reads left in the parser generations live here and
        // nowhere else. See the doc comment above: this entry point backs
        // generation 1 (schema 0) and generation 2 (schema 1 at PV12,
        // schema 2 from PV13 on), so the grammar admissions that flip
        // inside that range — `keeps*History` at schema 2, count indexes
        // at PV12 — are computed here and passed down as plain booleans.
        let document_type_schema_version = platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .document_type_schema;

        common::parse_document_type_core(
            data_contract_id,
            data_contract_system_version,
            contract_config_version,
            name,
            schema,
            schema_defs,
            token_configurations,
            data_contact_config,
            full_validation,
            validation_operations,
            &common::ParserGeneration {
                document_type_schema_version,
                admit_history: document_type_schema_version >= 2,
                admit_count_indexes: platform_version.protocol_version >= 12,
                // The `refersTo` reference keyword arrived with generation 3
                admit_property_references: false,
                meta_schema_method_name: "DocumentTypeV1::try_from_schema (document_type_schema)",
                // RANKED: generation 1 predates the ranked aggregates entirely
                // — its index grammar has no `ranked*` keywords, and it
                // therefore has no ranked key ceiling to enforce.
                admit_ranked: false,
                ranked_index_key_length_check: common::no_ranked_index_key_length_check,
            },
            platform_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use platform_value::platform_value;

    /// Generation-specific tests must pin a protocol version that actually
    /// selects their own generation: `PlatformVersion::latest()` silently
    /// retargets these tests onto a different parser generation and a
    /// different document meta-schema whenever LATEST moves. PV11 is the
    /// highest protocol version whose `try_from_schema` selects generation 1.
    fn generation_1_platform_version() -> &'static PlatformVersion {
        PlatformVersion::get(11).expect("protocol version 11 exists")
    }

    mod keeps_history_flags_version_gating {
        use super::*;
        use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use platform_value::Value;

        /// A minimal document type schema carrying `keepsTransferHistory` with
        /// the given value.
        fn schema_with_transfer_history_flag(flag_value: Value) -> Value {
            Value::Map(vec![
                (Value::Text("type".into()), Value::Text("object".into())),
                (Value::Text("keepsTransferHistory".into()), flag_value),
                (
                    Value::Text("properties".into()),
                    Value::Map(vec![(
                        Value::Text("name".into()),
                        Value::Map(vec![
                            (Value::Text("type".into()), Value::Text("string".into())),
                            (Value::Text("position".into()), Value::U64(0)),
                            (Value::Text("maxLength".into()), Value::U64(10)),
                        ]),
                    )]),
                ),
                (
                    Value::Text("additionalProperties".into()),
                    Value::Bool(false),
                ),
            ])
        }

        fn parse_at_version(
            schema: Value,
            protocol_version: u32,
        ) -> Result<DocumentTypeV1, ProtocolError> {
            let platform_version =
                PlatformVersion::get(protocol_version).expect("expected platform version");
            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");
            DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "test_doc",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut vec![],
                platform_version,
            )
        }

        /// Historical protocol versions used document meta-schema v0, which
        /// accepted and ignored unknown top-level keys — including a
        /// non-boolean value under a name that later became a config flag.
        /// The parser must keep ignoring them so replay validation of
        /// historical contracts is unchanged.
        #[test]
        fn non_boolean_flag_is_ignored_before_meta_schema_v2() {
            let parsed = parse_at_version(
                schema_with_transfer_history_flag(Value::Text("not-a-bool".into())),
                11,
            )
            .expect("expected historical parse to accept and ignore the unknown key");

            assert!(!parsed.documents_keep_transfer_history());
        }

        /// Even a well-formed boolean must not affect the typed configuration
        /// before the flags activate: the base implementation had no such
        /// config, so a historical contract carrying the key parses to the
        /// same document type it always did.
        #[test]
        fn boolean_flag_is_ignored_before_meta_schema_v2() {
            let parsed = parse_at_version(schema_with_transfer_history_flag(Value::Bool(true)), 11)
                .expect("expected historical parse to accept and ignore the unknown key");

            assert!(!parsed.documents_keep_transfer_history());
        }

        /// From document meta-schema v2 (protocol version 13) the flag is
        /// parsed into the typed configuration.
        #[test]
        fn boolean_flag_is_parsed_from_meta_schema_v2() {
            let parsed = parse_at_version(schema_with_transfer_history_flag(Value::Bool(true)), 13)
                .expect("expected parse at protocol version 13");

            assert!(parsed.documents_keep_transfer_history());
        }
    }

    mod nested_property_position_handling {
        use super::*;
        use platform_value::Value;

        /// Builds `outer(object) -> { inner_a(string, position = <pos>), inner_b(string,
        /// position = 1) }`. Two nested sub-properties are required so the (now-removed) property
        /// sort would have invoked its comparator, with the candidate `position` on a *nested*
        /// property.
        fn schema_with_nested_position(inner_a_position: Value) -> Value {
            let string_prop = |position: Value| {
                Value::Map(vec![
                    (Value::Text("type".into()), Value::Text("string".into())),
                    (Value::Text("position".into()), position),
                    (Value::Text("maxLength".into()), Value::U64(10)),
                ])
            };
            let outer = Value::Map(vec![
                (Value::Text("type".into()), Value::Text("object".into())),
                (Value::Text("position".into()), Value::U64(0)),
                (
                    Value::Text("properties".into()),
                    Value::Map(vec![
                        (Value::Text("inner_a".into()), string_prop(inner_a_position)),
                        (Value::Text("inner_b".into()), string_prop(Value::U64(1))),
                    ]),
                ),
                (
                    Value::Text("additionalProperties".into()),
                    Value::Bool(false),
                ),
            ]);
            Value::Map(vec![
                (Value::Text("type".into()), Value::Text("object".into())),
                (
                    Value::Text("properties".into()),
                    Value::Map(vec![(Value::Text("outer".into()), outer)]),
                ),
                (
                    Value::Text("additionalProperties".into()),
                    Value::Bool(false),
                ),
            ])
        }

        fn parse(schema: Value, full_validation: bool) -> Result<DocumentTypeV1, ProtocolError> {
            let platform_version = generation_1_platform_version();
            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");
            DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "test_doc",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                full_validation,
                &mut vec![],
                platform_version,
            )
        }

        /// A nested-property `position` that is a zero-fraction float (`0.0`) is a valid integer
        /// per the document meta-schema (JSON-Schema "integer" admits `0.0`); nested positions are
        /// not otherwise consensus-relevant, so it parses in both modes. Previously the property
        /// sort's `.expect()` panicked on it — the pinned contract is now: parse, never panic.
        #[test]
        fn nested_float_position_parses_in_both_modes() {
            assert_matches!(
                parse(schema_with_nested_position(Value::Float(0.0)), true),
                Ok(_)
            );
            assert_matches!(
                parse(schema_with_nested_position(Value::Float(0.0)), false),
                Ok(_)
            );
        }

        /// On the `check_tx` path (`full_validation = false`) the meta-schema is skipped and nested
        /// positions are not read, so malformed nested positions are admitted to the mempool (they
        /// are caught under full validation — see below). Pinned contract: parse, never panic.
        #[test]
        fn malformed_nested_positions_admitted_in_check_tx() {
            assert_matches!(
                parse(schema_with_nested_position(Value::I64(-1)), false),
                Ok(_)
            );
            assert_matches!(
                parse(
                    schema_with_nested_position(Value::U128(u64::MAX as u128 + 1)),
                    false
                ),
                Ok(_)
            );
        }

        /// Under full validation (block execution) the meta-schema rejects out-of-range nested
        /// positions with a clean consensus error — never a panic. This pins the rejection path
        /// the old `.expect()` short-circuited.
        #[test]
        fn out_of_range_nested_positions_rejected_under_full_validation() {
            // Negative position -> meta-schema `minimum: 0`.
            assert_matches!(
                parse(schema_with_nested_position(Value::I64(-1)), true),
                Err(ProtocolError::ConsensusError(_))
            );
            // Position > u64::MAX -> integer-out-of-bounds during meta-schema value conversion.
            assert_matches!(
                parse(
                    schema_with_nested_position(Value::U128(u64::MAX as u128 + 1)),
                    true
                ),
                Err(ProtocolError::ConsensusError(_))
            );
        }

        /// A well-formed schema with valid integer nested positions still parses successfully:
        /// removing the dead sort did not change accepted-contract behavior.
        #[test]
        fn valid_nested_positions_still_parse() {
            let result = parse(schema_with_nested_position(Value::U64(0)), true);
            assert!(
                result.is_ok(),
                "valid nested positions must still parse: {:?}",
                result.err()
            );
        }
    }

    mod document_meta_schema_version {
        use super::*;

        /// These two tests are about the document *meta-schema*, not the
        /// parser generation: they are the strict-meta-schema half of the
        /// contrast with `v0_schema_allows_unknown_properties` above. PV12 is
        /// where `document_type_schema` is 1, so this is the pin that makes
        /// the tests exercise the meta-schema version their names claim
        /// (under `latest()` they silently drifted onto meta-schema v2).
        fn meta_schema_v1_platform_version() -> &'static PlatformVersion {
            PlatformVersion::get(12).expect("protocol version 12 exists")
        }

        #[test]
        fn v0_schema_allows_unknown_properties() {
            let platform_version = PlatformVersion::first();

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test_field": {
                        "type": "string",
                        "position": 0
                    }
                },
                "additionalProperties": false,
                "unknownProp": true
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "test_doc",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut vec![],
                platform_version,
            );

            assert!(
                result.is_ok(),
                "v0 schema should allow unknown top-level properties, got error: {:?}",
                result.err()
            );
        }

        #[test]
        fn v1_schema_rejects_unknown_properties() {
            let platform_version = meta_schema_v1_platform_version();

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test_field": {
                        "type": "string",
                        "position": 0
                    }
                },
                "additionalProperties": false,
                "unknownProp": true
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "test_doc",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut vec![],
                platform_version,
            );

            assert!(
                result.is_err(),
                "v1 schema should reject unknown top-level properties"
            );

            let err = result.unwrap_err();
            let err_str = format!("{:?}", err);
            let err_str_lower = err_str.to_lowercase();
            assert!(
                err_str_lower.contains("additional properties"),
                "Error should mention additional properties, got: {}",
                err_str
            );
        }

        #[test]
        fn v1_schema_accepts_known_properties() {
            let platform_version = meta_schema_v1_platform_version();

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "test_field": {
                        "type": "string",
                        "position": 0
                    }
                },
                "additionalProperties": false,
                "required": ["test_field"],
                "$comment": "hello"
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "test_doc",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut vec![],
                platform_version,
            );

            assert!(
                result.is_ok(),
                "v1 schema should accept known properties like required and $comment, got error: {:?}",
                result.err()
            );
        }
    }

    mod document_type_name {
        use super::*;

        #[test]
        fn should_be_valid() {
            let platform_version = generation_1_platform_version();

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "valid_name": {
                        "type": "string",
                        "position": 0
                    }
                },
                "additionalProperties": false
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let _result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "valid_name-a-b-123",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut vec![],
                platform_version,
            )
            .expect("should be valid");
        }

        #[test]
        fn should_no_be_empty() {
            let platform_version = generation_1_platform_version();

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "valid_name": {
                        "type": "string",
                        "position": 0
                    }
                },
                "additionalProperties": false
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut vec![],
                platform_version,
            );

            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(boxed)) => {
                    assert_matches!(
                        boxed.as_ref(),
                        ConsensusError::BasicError(
                            BasicError::InvalidDocumentTypeNameError(InvalidDocumentTypeNameError { .. })
                        )
                    )
                }
            );
        }

        #[test]
        fn should_no_be_longer_than_64_chars() {
            let platform_version = generation_1_platform_version();

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "valid_name": {
                        "type": "string",
                        "position": 0
                    }
                },
                "additionalProperties": false
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                &"a".repeat(65),
                schema,
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut vec![],
                platform_version,
            );

            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(boxed)) => {
                    assert_matches!(
                        boxed.as_ref(),
                        ConsensusError::BasicError(
                            BasicError::InvalidDocumentTypeNameError(InvalidDocumentTypeNameError { .. })
                        )
                    )
                }
            );
        }

        #[test]
        fn should_no_be_alphanumeric() {
            let platform_version = generation_1_platform_version();

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "valid_name": {
                        "type": "string",
                        "position": 0
                    }
                },
                "additionalProperties": false
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "invalid name",
                schema.clone(),
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut vec![],
                platform_version,
            );

            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(boxed)) => {
                    assert_matches!(
                        boxed.as_ref(),
                        ConsensusError::BasicError(
                            BasicError::InvalidDocumentTypeNameError(InvalidDocumentTypeNameError { .. })
                        )
                    )
                }
            );

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                config.version(),
                "invalid&name",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut vec![],
                platform_version,
            );

            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(boxed)) => {
                    assert_matches!(
                        boxed.as_ref(),
                        ConsensusError::BasicError(
                            BasicError::InvalidDocumentTypeNameError(InvalidDocumentTypeNameError { .. })
                        )
                    )
                }
            );
        }
    }

    mod error_paths {
        use super::*;
        use crate::data_contract::document_type::token_costs::accessors::TokenCostGettersV0;

        fn default_config() -> DataContractConfig {
            DataContractConfig::default_for_version(generation_1_platform_version())
                .expect("should create a default config")
        }

        // ---------- Index errors ----------
        #[test]
        fn duplicate_index_name_returns_error() {
            let platform_version = generation_1_platform_version();
            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 40_u32},
                    "b": {"type": "string", "position": 1, "maxLength": 40_u32},
                },
                "indices": [
                    {"name": "dup", "properties": [{"a": "asc"}]},
                    {"name": "dup", "properties": [{"b": "asc"}]},
                ],
                "additionalProperties": false,
            });
            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(),
                &default_config(),
                true,
                &mut vec![],
                platform_version,
            );
            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(boxed)) => {
                    assert_matches!(
                        boxed.as_ref(),
                        ConsensusError::BasicError(BasicError::DuplicateIndexNameError(_))
                    )
                }
            );
        }

        #[test]
        fn undefined_index_property_returns_error() {
            let platform_version = generation_1_platform_version();
            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 40_u32},
                },
                "indices": [
                    {"name": "idx", "properties": [{"missing": "asc"}]},
                ],
                "additionalProperties": false,
            });
            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(),
                &default_config(),
                true,
                &mut vec![],
                platform_version,
            );
            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(boxed)) => {
                    assert_matches!(
                        boxed.as_ref(),
                        ConsensusError::BasicError(BasicError::UndefinedIndexPropertyError(_))
                    )
                }
            );
        }

        #[test]
        fn missing_positions_returns_error() {
            let platform_version = generation_1_platform_version();
            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 10_u32},
                    "c": {"type": "string", "position": 2, "maxLength": 10_u32},
                },
                "additionalProperties": false,
            });
            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(),
                &default_config(),
                true,
                &mut vec![],
                platform_version,
            );
            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(boxed)) => {
                    assert_matches!(
                        boxed.as_ref(),
                        ConsensusError::BasicError(
                            BasicError::MissingPositionsInDocumentTypePropertiesError(_)
                        )
                    )
                }
            );
        }

        #[test]
        fn indexed_string_exceeding_max_length_returns_error() {
            let platform_version = generation_1_platform_version();
            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "big": {"type": "string", "position": 0, "maxLength": 1000_u32},
                },
                "indices": [
                    {"name": "byBig", "properties": [{"big": "asc"}]},
                ],
                "additionalProperties": false,
            });
            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(),
                &default_config(),
                true,
                &mut vec![],
                platform_version,
            );
            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(boxed)) => {
                    assert_matches!(
                        boxed.as_ref(),
                        ConsensusError::BasicError(
                            BasicError::InvalidIndexedPropertyConstraintError(_)
                        )
                    )
                }
            );
        }

        // ---------- Token cost: InvalidTokenPositionError ----------
        #[test]
        fn token_cost_with_unknown_position_and_no_contract_id_errors() {
            let platform_version = generation_1_platform_version();
            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 40_u32},
                },
                "tokenCost": {
                    "create": {
                        // No contractId and an unknown tokenPosition -> error
                        "tokenPosition": 99_u64,
                        "amount": 1_u64,
                    }
                },
                "additionalProperties": false,
            });

            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(), // no token configurations
                &default_config(),
                true,
                &mut vec![],
                platform_version,
            );
            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(boxed)) => {
                    assert_matches!(
                        boxed.as_ref(),
                        ConsensusError::BasicError(BasicError::InvalidTokenPositionError(_))
                    )
                }
            );
        }

        // ---------- Token cost: RedundantDocumentPaidForByTokenWithContractId ----------
        #[test]
        fn token_cost_with_own_contract_id_errors_redundant() {
            let platform_version = generation_1_platform_version();
            let own_id = Identifier::new([42; 32]);

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 40_u32},
                },
                "tokenCost": {
                    "create": {
                        "contractId": own_id.to_buffer(),
                        "tokenPosition": 0_u64,
                        "amount": 1_u64,
                    }
                },
                "additionalProperties": false,
            });

            let result = DocumentTypeV1::try_from_schema(
                own_id,
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(),
                &default_config(),
                true,
                &mut vec![],
                platform_version,
            );
            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(boxed)) => {
                    assert_matches!(
                        boxed.as_ref(),
                        ConsensusError::BasicError(
                            BasicError::RedundantDocumentPaidForByTokenWithContractId(_)
                        )
                    )
                }
            );
        }

        // ---------- Token cost: BurnToken on external contract is not allowed ----------
        #[test]
        fn burn_token_on_external_contract_returns_error() {
            let platform_version = generation_1_platform_version();
            let own_id = Identifier::new([42; 32]);
            let external_id = Identifier::new([99; 32]);

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 40_u32},
                },
                "tokenCost": {
                    "create": {
                        "contractId": external_id.to_buffer(),
                        "tokenPosition": 0_u64,
                        "amount": 1_u64,
                        "effect": 1_u64, // BurnToken
                    }
                },
                "additionalProperties": false,
            });

            let result = DocumentTypeV1::try_from_schema(
                own_id,
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(),
                &default_config(),
                true,
                &mut vec![],
                platform_version,
            );
            assert_matches!(
                result,
                Err(ProtocolError::ConsensusError(boxed)) => {
                    assert_matches!(
                        boxed.as_ref(),
                        ConsensusError::BasicError(
                            BasicError::TokenPaymentByBurningOnlyAllowedOnInternalTokenError(_)
                        )
                    )
                }
            );
        }

        // ---------- Token cost: valid external contract transfer is accepted ----------
        #[test]
        fn valid_token_cost_with_external_contract_is_accepted() {
            let platform_version = generation_1_platform_version();
            let own_id = Identifier::new([42; 32]);
            let external_id = Identifier::new([99; 32]);

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 40_u32},
                },
                "tokenCost": {
                    "create": {
                        "contractId": external_id.to_buffer(),
                        "tokenPosition": 0_u64,
                        "amount": 5_u64,
                        "effect": 0_u64, // TransferTokenToContractOwner
                    }
                },
                "additionalProperties": false,
            });

            let dt = DocumentTypeV1::try_from_schema(
                own_id,
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(),
                &default_config(),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("should be accepted");
            // The create cost should be populated
            let cost = dt.token_costs.document_creation_token_cost();
            assert!(cost.is_some());
            let cost = cost.unwrap();
            assert_eq!(cost.token_amount, 5);
            assert_eq!(cost.token_contract_position, 0);
            assert_eq!(cost.contract_id, Some(external_id));
        }

        // ---------- With full_validation = false, token cost validations are skipped
        #[test]
        fn invalid_token_cost_without_validation_still_constructs() {
            let platform_version = generation_1_platform_version();
            let own_id = Identifier::new([42; 32]);

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 40_u32},
                },
                "tokenCost": {
                    "create": {
                        // own contract id but validation skipped
                        "contractId": own_id.to_buffer(),
                        "tokenPosition": 0_u64,
                        "amount": 1_u64,
                    }
                },
                "additionalProperties": false,
            });

            let dt = DocumentTypeV1::try_from_schema(
                own_id,
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(),
                &default_config(),
                false, // skip validation
                &mut vec![],
                platform_version,
            )
            .expect("should construct without validation");
            assert!(dt.token_costs.document_creation_token_cost().is_some());
        }

        // ---------- TRANSFERABLE u8 conversion failure path ----------
        #[test]
        fn invalid_transferable_integer_returns_error() {
            let platform_version = generation_1_platform_version();
            let schema = platform_value!({
                "type": "object",
                "transferable": 7_u64,
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 10_u32}
                },
                "additionalProperties": false,
            });
            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(),
                &default_config(),
                false, // skip schema validation
                &mut vec![],
                platform_version,
            );
            assert!(result.is_err());
        }

        // ---------- Non-object schema fails in .to_map() ----------
        #[test]
        fn non_object_schema_returns_error_without_validation() {
            let platform_version = generation_1_platform_version();
            let schema = platform_value!("not_an_object");
            let result = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(),
                &default_config(),
                false,
                &mut vec![],
                platform_version,
            );
            assert!(result.is_err());
        }

        // ---------- Valid schema with all optional configuration fields set ----------
        #[test]
        fn full_config_options_are_preserved_on_successful_build() {
            let platform_version = generation_1_platform_version();
            let schema = platform_value!({
                "type": "object",
                "documentsKeepHistory": true,
                "documentsMutable": true,
                "canBeDeleted": false,
                "transferable": 1_u64,
                "tradeMode": 1_u64,
                "creationRestrictionMode": 1_u64,
                "signatureSecurityLevelRequirement": 1_u64,
                "requiresIdentityEncryptionBoundedKey": 0_u64,
                "requiresIdentityDecryptionBoundedKey": 0_u64,
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 10_u32},
                },
                "additionalProperties": false,
            });
            let dt = DocumentTypeV1::try_from_schema(
                Identifier::new([1; 32]),
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &BTreeMap::new(),
                &default_config(),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("should build");
            assert!(dt.documents_keep_history);
            assert!(dt.documents_mutable);
            assert!(!dt.documents_can_be_deleted);
            assert!(dt.documents_transferable.is_transferable());
            // Non-default SecurityLevel was parsed (1 = CRITICAL vs default HIGH)
            assert_eq!(dt.security_level_requirement, SecurityLevel::CRITICAL);
            assert!(dt.requires_identity_encryption_bounded_key.is_some());
            assert!(dt.requires_identity_decryption_bounded_key.is_some());
        }

        // ---------- v1 behavior: BurnToken is allowed if contract is "own" (no contractId) ----------
        #[test]
        fn burn_effect_on_own_contract_is_allowed_when_token_configured() {
            use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
            use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
            use crate::data_contract::TokenContractPosition;
            use platform_value::string_encoding::Encoding;

            let platform_version = generation_1_platform_version();

            let schema = platform_value!({
                "type": "object",
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 40_u32},
                },
                "tokenCost": {
                    "create": {
                        // No contractId => "own contract"; Burn is allowed
                        "tokenPosition": 0_u64,
                        "amount": 1_u64,
                        "effect": 1_u64,
                    }
                },
                "additionalProperties": false,
            });

            let token_cfg = TokenConfigurationV0::default_most_restrictive();
            let mut token_configurations: BTreeMap<TokenContractPosition, TokenConfiguration> =
                BTreeMap::new();
            token_configurations.insert(0, TokenConfiguration::V0(token_cfg));

            // Also silence an unused-import warning on Encoding in case the compile path differs.
            let _ = Encoding::Base58;

            let dt = DocumentTypeV1::try_from_schema(
                Identifier::new([42; 32]),
                1,
                default_config().version(),
                "doc",
                schema,
                None,
                &token_configurations,
                &default_config(),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("should construct with own-contract burn");
            assert!(dt.token_costs.document_creation_token_cost().is_some());
        }
    }
}
