use crate::block::block_info::BlockInfo;
use crate::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError;
use crate::consensus::state::data_contract::data_contract_update_action_not_allowed_error::DataContractUpdateActionNotAllowedError;
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::class_methods::{
    first_repeated_key_of_definitions, first_repeated_key_of_schema,
};
use crate::data_contract::document_type::property_names::{
    MAX_PROPERTIES, PROPERTIES, REF, REQUIRED,
};
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::{Value, ValueMapHelper};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

impl DataContract {
    /// Generation 2 (protocol version 15): generation 1, once no document type of the stored
    /// contract names a key twice in an object of its schema, or of the contract's `$defs`.
    ///
    /// From protocol version 15 registration refuses such a schema (`check_document_type_schema`
    /// 1), but a contract stored earlier may hold one. The checks of an update read the stored
    /// schema as JSON, which keeps the last entry of a repeated key, where the document type
    /// parser may have read the first, so they can not judge an update of it: an update of a
    /// contract whose stored schema repeats a key is refused. The new schema can not repeat
    /// the key either, so the schemas of such a contract stay as they were stored.
    ///
    /// The rule is about an update that submits schemas. A token configuration update submits
    /// none and stores the stored schemas and `$defs` as they are, so it does not come here.
    ///
    /// Once generation 1 accepts the update, it is refused when it makes the `required` of an
    /// object of a type the contract keeps name more members than the object may hold
    /// (`maxProperties`): document type update generation 2 lets such a list gain the members
    /// the update adds, and only the contract holds the `$defs` whose caps also bind.
    ///
    /// Delegating to generation 1 is safe because that generation is shipped and therefore
    /// frozen. The checks are independent and short-circuiting, so running the extra one first
    /// changes only which error is reported when an update violates several rules at once,
    /// never whether it is rejected.
    #[inline(always)]
    pub(super) fn validate_update_v2(
        &self,
        new_data_contract: &DataContract,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let result = self.validate_update_stored_schemas_repeat_no_key();
        if !result.is_valid() {
            return Ok(result);
        }

        let result = self.validate_update_v1(new_data_contract, block_info, platform_version)?;
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(self.validate_update_required_lists_fit(new_data_contract))
    }

    /// Refuses the update when the `required` of an object of a document type the contract
    /// keeps names more members than the object may hold: `maxProperties`, written on the
    /// object, beside its `$ref`, or on the definition the `$ref` names. No update can change
    /// one, and no document could then hold the object. Only a list the update makes longer is
    /// judged, so a type stored with such a list stays updatable.
    fn validate_update_required_lists_fit(
        &self,
        new_data_contract: &DataContract,
    ) -> SimpleConsensusValidationResult {
        let definitions = new_data_contract.schema_defs();
        for (document_type_name, old_document_type) in self.document_types() {
            let Some(new_document_type) =
                new_data_contract.document_type_optional_for_name(document_type_name)
            else {
                continue;
            };
            if let Some(overfull) = first_overfull_required_list(
                old_document_type.schema(),
                new_document_type.schema(),
                definitions,
                None,
            ) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        document_type_name.clone(),
                        format!(
                            "the required of '{}' would name {} members, more than the {} its \
                             object may hold (maxProperties), so no document could hold the \
                             object",
                            overfull.path, overfull.required, overfull.max_properties
                        ),
                    )
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }

    /// Refuses the update when an object of a stored document type's schema, or of the
    /// stored `$defs` a type's parse reads, names a key more than once, as registration from
    /// protocol version 15 refuses a schema that does (the same walk). A type's schema is
    /// reported on the type, `$defs` on the contract.
    ///
    /// The walk reads each stored schema and the stored `$defs` once, so it is linear in the
    /// size of the stored contract, which the update already fetched and pays for: the count
    /// of values read, which registration bills on a refusal, is not billed here.
    fn validate_update_stored_schemas_repeat_no_key(&self) -> SimpleConsensusValidationResult {
        let mut read = 0;
        for (name, document_type) in self.document_types() {
            if let Some(repeated) = first_repeated_key_of_schema(document_type.schema(), &mut read)
            {
                return SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.id(),
                        name,
                        format!(
                            "its stored schema declares `{}` more than once {}, and a \
                             contract whose schema repeats a key can not be updated",
                            repeated.key(),
                            repeated.place()
                        ),
                    )
                    .into(),
                );
            }
        }
        if let Some(repeated) = first_repeated_key_of_definitions(self.schema_defs(), &mut read) {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractUpdateActionNotAllowedError::new(
                    self.id(),
                    format!(
                        "update a contract whose stored `$defs` declare `{}` more than once {}",
                        repeated.key(),
                        repeated.place()
                    ),
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

/// An object whose `required` names more members than it may hold.
struct OverfullRequiredList {
    path: String,
    required: usize,
    max_properties: u64,
}

/// The first object below `new` (a schema of an object, or of the document type when `path` is
/// `None`) that the old schema `old` also holds, whose `required` the update lengthened past the
/// object's `maxProperties`. The walk follows the objects written inline; a `$ref` member is
/// judged with the definition it names, whose own lists an update can not change.
fn first_overfull_required_list(
    old: &Value,
    new: &Value,
    definitions: Option<&BTreeMap<String, Value>>,
    path: Option<&str>,
) -> Option<OverfullRequiredList> {
    let (old_members, new_members) = (inline_members(old)?, inline_members(new)?);
    new_members.iter().find_map(|(name, new_member)| {
        let name = name.as_text()?;
        let old_member = old_members.get_optional_key(name)?;
        let member_path = path.map_or_else(|| name.to_string(), |path| format!("{path}.{name}"));
        let required = required_count(new_member);
        if required > required_count(old_member) {
            let max_properties = [
                max_properties(new_member),
                referenced_definition(new_member, definitions).and_then(max_properties),
            ]
            .into_iter()
            .flatten()
            .min();
            if let Some(max_properties) = max_properties {
                if required as u64 > max_properties {
                    return Some(OverfullRequiredList {
                        path: member_path,
                        required,
                        max_properties,
                    });
                }
            }
        }
        first_overfull_required_list(old_member, new_member, definitions, Some(&member_path))
    })
}

/// The `properties` of the object `schema` describes inline, `None` for a `$ref`.
fn inline_members(schema: &Value) -> Option<&Vec<(Value, Value)>> {
    let schema = schema.as_map()?;
    if schema.get_optional_key(REF).is_some() {
        return None;
    }
    schema.get_optional_key(PROPERTIES)?.as_map()
}

/// How many distinct names the `required` of `schema` holds.
fn required_count(schema: &Value) -> usize {
    schema
        .as_map()
        .and_then(|schema| schema.get_optional_key(REQUIRED))
        .and_then(Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_text)
                .collect::<BTreeSet<_>>()
                .len()
        })
        .unwrap_or_default()
}

fn max_properties(schema: &Value) -> Option<u64> {
    schema
        .as_map()?
        .get_optional_key(MAX_PROPERTIES)?
        .as_integer::<u64>()
}

/// The definition of `definitions` the `$ref` of `schema` names (`#/$defs/<name>`).
fn referenced_definition<'a>(
    schema: &Value,
    definitions: Option<&'a BTreeMap<String, Value>>,
) -> Option<&'a Value> {
    let reference = schema.as_map()?.get_optional_key(REF)?.as_text()?;
    definitions?.get(reference.strip_prefix("#/$defs/")?)
}

#[cfg(test)]
mod tests {
    use crate::block::block_info::BlockInfo;
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use crate::data_contract::document_type::DocumentPropertyType;
    use crate::data_contract::methods::validate_update::DataContractUpdateValidationMethodsV0;
    use crate::data_contract::schema::DataContractSchemaMethodsV0;
    use crate::data_contract::DataContract;
    use crate::prelude::IdentityNonce;
    use crate::tests::fixtures::get_data_contract_fixture;
    use assert_matches::assert_matches;
    use platform_value::{platform_value, Value};
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    const TYPE_NAME: &str = "t";

    fn text(value: &str) -> Value {
        Value::Text(value.to_string())
    }

    fn string_a() -> Value {
        platform_value!({ "a": { "type": "string", "maxLength": 10, "position": 0 } })
    }

    fn integer_a() -> Value {
        platform_value!({ "a": { "type": "integer", "minimum": 0, "maximum": 100, "position": 0 } })
    }

    /// A document type schema holding `properties` once for each of `properties`, in order.
    fn schema(properties: Vec<Value>) -> Value {
        let mut entries = vec![(text("type"), text("object"))];
        entries.extend(
            properties
                .into_iter()
                .map(|properties| (text("properties"), properties)),
        );
        entries.push((text("additionalProperties"), Value::Bool(false)));
        Value::Map(entries)
    }

    /// The fixture contract with the type `t` of `stored_schema` and the `$defs`
    /// `stored_defs`, placed as a contract read back from state is parsed (without
    /// validation), and its update to `new_schema` and `new_defs`, parsed under full
    /// validation as a block parses the update's contract.
    fn stored_and_updated(
        stored_schema: Value,
        stored_defs: Option<BTreeMap<String, Value>>,
        new_schema: Value,
        new_defs: Option<BTreeMap<String, Value>>,
        platform_version: &PlatformVersion,
    ) -> (DataContract, DataContract) {
        let mut stored = get_data_contract_fixture(
            None,
            IdentityNonce::default(),
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let with_fixture_defs = |contract: &DataContract, defs: Option<BTreeMap<String, Value>>| {
            defs.map(|defs| {
                let mut all = contract.schema_defs().cloned().unwrap_or_default();
                all.extend(defs);
                all
            })
        };
        if let Some(defs) = with_fixture_defs(&stored, stored_defs) {
            stored
                .set_schema_defs(Some(defs), false, &mut Vec::new(), platform_version)
                .expect("expected the stored $defs to be placed");
        }
        stored
            .set_document_schema(
                TYPE_NAME,
                stored_schema,
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("expected the stored schema to load");

        let mut updated = stored.clone();
        updated.set_version(stored.version() + 1);
        if let Some(defs) = with_fixture_defs(&updated, new_defs) {
            updated
                .set_schema_defs(Some(defs), false, &mut Vec::new(), platform_version)
                .expect("expected the new $defs to be placed");
        }
        updated
            .set_document_schema(
                TYPE_NAME,
                new_schema,
                true,
                &mut Vec::new(),
                platform_version,
            )
            .expect("expected the new schema to register");
        (stored, updated)
    }

    fn parsed_type_of(contract: &DataContract, path: &str) -> DocumentPropertyType {
        contract
            .document_type_for_name(TYPE_NAME)
            .expect("expected the type")
            .flattened_properties()
            .get(path)
            .expect("expected the property")
            .property_type
            .clone()
    }

    fn assert_update_refused(stored: &DataContract, updated: &DataContract, key: &str) {
        let result = stored
            .validate_update(updated, &BlockInfo::default(), PlatformVersion::latest())
            .expect("expected the update to be judged");
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                if e.additional_message().contains(&format!("declares `{key}` more than once"))
        );
    }

    /// The stored type reads one entry of a repeated root `properties`, the update keeps the
    /// other: whichever order, the update is refused, so the parsed type of `a` stays as it
    /// was stored.
    #[test]
    fn should_refuse_an_update_of_a_stored_schema_repeating_root_properties() {
        let platform_version = PlatformVersion::latest();
        for (stored_schema, new_properties) in [
            (schema(vec![string_a(), integer_a()]), integer_a()),
            (schema(vec![integer_a(), string_a()]), string_a()),
        ] {
            let (stored, updated) = stored_and_updated(
                stored_schema,
                None,
                schema(vec![new_properties]),
                None,
                platform_version,
            );
            assert_ne!(
                std::mem::discriminant(&parsed_type_of(&stored, "a")),
                std::mem::discriminant(&parsed_type_of(&updated, "a")),
            );
            assert_update_refused(&stored, &updated, "properties");
        }
    }

    /// The same for a repeated `properties` of a nested object.
    #[test]
    fn should_refuse_an_update_of_a_stored_schema_repeating_nested_properties() {
        let platform_version = PlatformVersion::latest();
        let note = |properties: Vec<Value>| {
            let mut entries = vec![
                (text("type"), text("object")),
                (text("position"), Value::U64(0)),
            ];
            entries.extend(
                properties
                    .into_iter()
                    .map(|properties| (text("properties"), properties)),
            );
            entries.push((text("additionalProperties"), Value::Bool(false)));
            schema(vec![platform_value!({ "note": Value::Map(entries) })])
        };
        let (stored, updated) = stored_and_updated(
            note(vec![string_a(), integer_a()]),
            None,
            note(vec![integer_a()]),
            None,
            platform_version,
        );
        assert_update_refused(&stored, &updated, "properties");
    }

    /// The same for a repeated key of the stored `$defs` a property of the type refers to.
    #[test]
    fn should_refuse_an_update_of_a_stored_definition_repeating_a_key() {
        let platform_version = PlatformVersion::latest();
        let word = |properties: Vec<Value>| {
            let mut entries = vec![(text("type"), text("object"))];
            entries.extend(
                properties
                    .into_iter()
                    .map(|properties| (text("properties"), properties)),
            );
            entries.push((text("additionalProperties"), Value::Bool(false)));
            BTreeMap::from([("word".to_string(), Value::Map(entries))])
        };
        let referring = schema(vec![
            platform_value!({ "w": { "$ref": "#/$defs/word", "position": 0 } }),
        ]);
        let (stored, updated) = stored_and_updated(
            referring.clone(),
            Some(word(vec![string_a(), integer_a()])),
            referring,
            Some(word(vec![integer_a()])),
            platform_version,
        );
        let result = stored
            .validate_update(&updated, &BlockInfo::default(), platform_version)
            .expect("expected the update to be judged");
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractUpdateActionNotAllowedError(e))]
                if e.action().contains("`$defs` declare `properties` more than once at `$defs.word`")
        );
    }

    /// The stored `$defs` are read once, whatever the number of document types, and a
    /// type's own schema is reported before them.
    #[test]
    fn should_report_a_stored_schema_repeating_a_key_before_the_stored_definitions() {
        let platform_version = PlatformVersion::latest();
        let word = BTreeMap::from([(
            "word".to_string(),
            Value::Map(vec![
                (text("type"), text("string")),
                (text("type"), text("string")),
            ]),
        )]);
        let (stored, updated) = stored_and_updated(
            schema(vec![string_a(), string_a()]),
            Some(word),
            schema(vec![string_a()]),
            Some(BTreeMap::from([(
                "word".to_string(),
                platform_value!({ "type": "string" }),
            )])),
            platform_version,
        );
        assert_update_refused(&stored, &updated, "properties");
    }

    /// An update of a contract whose stored schema names each key once is judged as before.
    #[test]
    fn should_admit_an_update_of_a_stored_schema_naming_each_key_once() {
        let platform_version = PlatformVersion::latest();
        let (stored, updated) = stored_and_updated(
            schema(vec![string_a()]),
            None,
            schema(vec![string_a()]),
            None,
            platform_version,
        );
        let result = stored
            .validate_update(&updated, &BlockInfo::default(), platform_version)
            .expect("expected the update to be judged");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    /// The rule starts with protocol version 15: before it, an update of a contract whose
    /// stored schema names a key twice is judged by the rules of generation 1 alone.
    #[test]
    fn should_refuse_an_update_of_a_stored_schema_repeating_a_key_from_protocol_version_15() {
        for (platform_version, refused) in [
            (
                PlatformVersion::get(14).expect("protocol version 14"),
                false,
            ),
            (PlatformVersion::latest(), true),
        ] {
            let (stored, updated) = stored_and_updated(
                schema(vec![string_a(), string_a()]),
                None,
                schema(vec![string_a()]),
                None,
                platform_version,
            );
            let result = stored
                .validate_update(&updated, &BlockInfo::default(), platform_version)
                .expect("expected the update to be judged");
            assert_eq!(!result.is_valid(), refused, "{:?}", result.errors);
        }
    }

    /// A definition's member required from the contract version an update creates, through the
    /// `required` beside a `$ref`, passes the contract's `$defs` compatibility as an added
    /// member; and a document type added later may take the definition whatever version its
    /// member names, since no stored document holds the new type.
    #[test]
    fn should_admit_a_definition_member_required_from_the_version_an_update_creates() {
        let platform_version = PlatformVersion::latest();
        let definition = |with_x: bool| {
            let mut members = platform_value!({
                "y": { "type": "boolean", "position": 0 },
            });
            if with_x {
                members
                    .set_value(
                        "x",
                        platform_value!({ "type": "boolean", "position": 1, "requiredSince": 2 }),
                    )
                    .expect("expected a map");
            }
            BTreeMap::from([(
                "m".to_string(),
                platform_value!({
                    "type": "object",
                    "properties": members,
                    "additionalProperties": false,
                }),
            )])
        };
        let referencing = |required: Value| {
            platform_value!({
                "type": "object",
                "properties": {
                    "meta": { "$ref": "#/$defs/m", "position": 0, "required": required },
                },
                "additionalProperties": false,
            })
        };

        // The stored contract, and its update: the new schema is placed before the new `$defs`,
        // which a parse of the old schema with the new definition would refuse
        let (stored, _) = stored_and_updated(
            referencing(platform_value!(["y"])),
            Some(definition(false)),
            referencing(platform_value!(["y"])),
            Some(definition(false)),
            platform_version,
        );
        let mut updated = stored.clone();
        updated.set_version(stored.version() + 1);
        updated
            .set_document_schema(
                TYPE_NAME,
                referencing(platform_value!(["y", "x"])),
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("expected the new schema to be placed");
        let mut defs = updated.schema_defs().cloned().unwrap_or_default();
        defs.extend(definition(true));
        updated
            .set_schema_defs(Some(defs), true, &mut Vec::new(), platform_version)
            .expect("expected the new $defs to register");
        let result = stored
            .validate_update(&updated, &BlockInfo::default(), platform_version)
            .expect("expected the update to be judged");
        assert!(result.is_valid(), "{:?}", result.errors);

        let mut with_new_type = updated.clone();
        with_new_type.set_version(updated.version() + 1);
        with_new_type
            .set_document_schema(
                "other",
                referencing(platform_value!(["y", "x"])),
                true,
                &mut Vec::new(),
                platform_version,
            )
            .expect("expected the new type to register");
        let result = updated
            .validate_update(&with_new_type, &BlockInfo::default(), platform_version)
            .expect("expected the update to be judged");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    /// An object may hold at most `maxProperties` members, which an update can not change: an
    /// update that makes its `required` name more members than that is refused, since no
    /// document could then hold the object. The cap is the object's own, the one beside its
    /// `$ref`, or its definition's.
    #[test]
    fn should_refuse_a_required_list_growing_past_its_object_s_max_properties() {
        use crate::consensus::basic::BasicError;

        let platform_version = PlatformVersion::latest();
        let inline = |max_properties: u32, with_x: bool| {
            let mut members = platform_value!({ "y": { "type": "boolean", "position": 0 } });
            let mut required = vec![text("y")];
            if with_x {
                members
                    .set_value(
                        "x",
                        platform_value!({ "type": "boolean", "position": 1, "requiredSince": 2 }),
                    )
                    .expect("expected a map");
                required.push(text("x"));
            }
            platform_value!({
                "type": "object",
                "properties": {
                    "meta": {
                        "type": "object",
                        "position": 0,
                        "properties": members,
                        "required": required,
                        "maxProperties": max_properties,
                        "additionalProperties": false,
                    },
                },
                "required": ["meta"],
                "additionalProperties": false,
            })
        };
        let judge = |stored: &DataContract, updated: &DataContract| {
            stored
                .validate_update(updated, &BlockInfo::default(), platform_version)
                .expect("expected the update to be judged")
        };

        let (stored, updated) = stored_and_updated(
            inline(1, false),
            None,
            inline(1, true),
            None,
            platform_version,
        );
        assert_matches!(
            judge(&stored, &updated).errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
            )] if e.details().contains("meta") && e.details().contains("maxProperties")
        );

        let (stored, updated) = stored_and_updated(
            inline(2, false),
            None,
            inline(2, true),
            None,
            platform_version,
        );
        let result = judge(&stored, &updated);
        assert!(result.is_valid(), "{:?}", result.errors);

        // The cap of the definition a `$ref` names
        let definition = |with_x: bool| {
            let mut members = platform_value!({ "y": { "type": "boolean", "position": 0 } });
            if with_x {
                members
                    .set_value(
                        "x",
                        platform_value!({ "type": "boolean", "position": 1, "requiredSince": 2 }),
                    )
                    .expect("expected a map");
            }
            BTreeMap::from([(
                "m".to_string(),
                platform_value!({
                    "type": "object",
                    "properties": members,
                    "maxProperties": 1,
                    "additionalProperties": false,
                }),
            )])
        };
        let referencing = |required: Value| {
            platform_value!({
                "type": "object",
                "properties": {
                    "meta": { "$ref": "#/$defs/m", "position": 0, "required": required },
                },
                "additionalProperties": false,
            })
        };
        let (stored, _) = stored_and_updated(
            referencing(platform_value!(["y"])),
            Some(definition(false)),
            referencing(platform_value!(["y"])),
            Some(definition(false)),
            platform_version,
        );
        let mut updated = stored.clone();
        updated.set_version(stored.version() + 1);
        updated
            .set_document_schema(
                TYPE_NAME,
                referencing(platform_value!(["y", "x"])),
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("expected the new schema to be placed");
        let mut defs = updated.schema_defs().cloned().unwrap_or_default();
        defs.extend(definition(true));
        updated
            .set_schema_defs(Some(defs), true, &mut Vec::new(), platform_version)
            .expect("expected the new $defs to register");
        assert_matches!(
            judge(&stored, &updated).errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
            )] if e.details().contains("maxProperties")
        );
    }
}
