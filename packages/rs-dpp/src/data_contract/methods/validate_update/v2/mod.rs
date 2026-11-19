use crate::block::block_info::BlockInfo;
use crate::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError;
use crate::consensus::state::data_contract::data_contract_update_action_not_allowed_error::DataContractUpdateActionNotAllowedError;
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::class_methods::{
    first_repeated_key_of_definitions, first_repeated_key_of_schema,
};
use crate::data_contract::document_type::{DocumentProperty, DocumentPropertyType};
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use indexmap::IndexMap;
use platform_version::version::PlatformVersion;

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
    /// Once generation 1 accepts the update, the members of the objects of a document type the
    /// update adds are held to the bound its own properties are held to there: no
    /// `requiredSince` past the version the update creates. A schema set through
    /// `set_document_schema` is not held to the bound a contract's deserialization checks.
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

        Ok(self.validate_update_new_document_types_member_required_since(new_data_contract))
    }

    /// Refuses a document type the update adds when a member of one of its objects, at any
    /// depth, carries a `requiredSince` past the version the update creates. Generation 1 holds
    /// the type's own properties to that version exactly; a member may name an earlier one, as
    /// a member of a shared `$defs` definition does.
    fn validate_update_new_document_types_member_required_since(
        &self,
        new_data_contract: &DataContract,
    ) -> SimpleConsensusValidationResult {
        let new_version = new_data_contract.version();
        for (document_type_name, new_document_type) in new_data_contract.document_types() {
            if self
                .document_type_optional_for_name(document_type_name)
                .is_some()
            {
                continue;
            }
            let past =
                new_document_type
                    .as_ref()
                    .properties()
                    .iter()
                    .find_map(|(name, property)| match &property.property_type {
                        DocumentPropertyType::Object(members) => {
                            first_member_required_past(members, name, new_version)
                        }
                        _ => None,
                    });
            if let Some((path, required_since)) = past {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractInvalidRequiredFieldsUpdateError::new(
                        document_type_name.clone(),
                        format!(
                            "new document type property '{path}' carries requiredSince \
                             {required_since}, a contract version past the {new_version} this \
                             update creates"
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

/// The dotted path and `requiredSince` of the first of `members`, the members of the object at
/// `path`, at any depth, required from a version past `version`.
fn first_member_required_past(
    members: &IndexMap<String, DocumentProperty>,
    path: &str,
    version: u32,
) -> Option<(String, u32)> {
    members.iter().find_map(|(name, member)| {
        let member_path = format!("{path}.{name}");
        if let Some(required_since) = member
            .required_since
            .filter(|required_since| *required_since > version)
        {
            return Some((member_path, required_since));
        }
        match &member.property_type {
            DocumentPropertyType::Object(members) => {
                first_member_required_past(members, &member_path, version)
            }
            _ => None,
        }
    })
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

    /// A definition whose member is required from an earlier contract version, through the
    /// `required` beside a `$ref`, may be taken by a document type an update adds: no stored
    /// document holds the new type, and the member is stored with a presence byte whatever
    /// version it names.
    #[test]
    fn should_admit_a_new_document_type_taking_a_definition_whose_member_is_required_from_an_earlier_version(
    ) {
        let platform_version = PlatformVersion::latest();
        let definition = BTreeMap::from([(
            "m".to_string(),
            platform_value!({
                "type": "object",
                "properties": {
                    "y": { "type": "boolean", "position": 0 },
                    "x": { "type": "boolean", "position": 1, "requiredSince": 1 },
                },
                "additionalProperties": false,
            }),
        )]);
        let referencing = platform_value!({
            "type": "object",
            "properties": {
                "meta": { "$ref": "#/$defs/m", "position": 0, "required": ["y", "x"] },
            },
            "additionalProperties": false,
        });
        let (stored, mut updated) = stored_and_updated(
            referencing.clone(),
            Some(definition.clone()),
            referencing.clone(),
            Some(definition),
            platform_version,
        );
        updated
            .set_document_schema(
                "other",
                referencing,
                true,
                &mut Vec::new(),
                platform_version,
            )
            .expect("expected the new type to register");
        let result = stored
            .validate_update(&updated, &BlockInfo::default(), platform_version)
            .expect("expected the update to be judged");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    /// A document type an update adds may not carry a member required from a version the
    /// contract has not reached, as its own properties may not: a schema set through
    /// `set_document_schema` is not held to the bound a contract's deserialization checks.
    #[test]
    fn should_refuse_a_new_document_type_whose_member_is_required_from_a_later_version() {
        use crate::consensus::basic::BasicError;

        let platform_version = PlatformVersion::latest();
        let stored = get_data_contract_fixture(
            None,
            IdentityNonce::default(),
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let new_type = |required_since: u32| {
            platform_value!({
                "type": "object",
                "properties": {
                    "meta": {
                        "type": "object",
                        "position": 0,
                        "properties": {
                            "x": { "type": "boolean", "position": 0, "requiredSince": required_since },
                        },
                        "required": ["x"],
                        "additionalProperties": false,
                    },
                },
                "additionalProperties": false,
            })
        };
        let update = |required_since: u32| {
            let mut updated = stored.clone();
            updated.set_version(stored.version() + 1);
            updated
                .set_document_schema(
                    "other",
                    new_type(required_since),
                    true,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("expected the new type to register");
            stored
                .validate_update(&updated, &BlockInfo::default(), platform_version)
                .expect("expected the update to be judged")
        };

        assert_matches!(
            update(3).errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
            )] if e.details().contains("meta.x") && e.details().contains("requiredSince 3")
        );
        let result = update(2);
        assert!(result.is_valid(), "{:?}", result.errors);
    }
}
