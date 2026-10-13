use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::class_methods::apply_required_since::apply_required_since;
use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
use crate::data_contract::document_type::class_methods::parse_typed_array::parse_typed_array;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::data_contract::document_type::property::reference_where::{
    WHERE_CONST, WHERE_CONTAINS, WHERE_CONTAINS_ALL, WHERE_IN, WHERE_NOT_EQUAL,
};
use crate::data_contract::document_type::property_constraints::{
    either_version_path, parse_delete_constraints, parse_property_constraints, referenced_path,
    referenced_read, stored_version_path, whole_integer, AffixPosition, AggregateBinding,
    AggregateKind, AggregateRead, ElementKind, EqualityKind, PropertyConstraint, PropertyRead,
    SystemChange, SystemProperty, SystemRead, SystemValue, REFERENCED_DOCUMENT_PREFIX,
    STORED_DOCUMENT_PREFIX, STORED_OWNER_ID,
};
use crate::data_contract::document_type::reference_lookup::{
    clearable_once_its_document_is_deleted, owner_can_change,
    schema_property_is_fixed_once_written, LOOKUP_REFERENCE_VALUE, MAX_CHOOSE_FIRST_ORDER,
    MAX_LOOKUP_KEYS, MAX_LOOKUP_PATH_LENGTH,
};
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::document_type::{
    is_path_listed, is_referenced_system_agreement_property,
    is_referring_system_agreement_property, is_transient, property_at_path, property_names,
    ChooseFirstOrder, ContractReferenceModeration, ContractReferenceOwner,
    ContractReferenceRequirements, DistinctFrom, DocumentProperty, DocumentPropertyReferenceTarget,
    DocumentPropertyType, DocumentPropertyTypeParsingOptions, DocumentReferenceDeclaration,
    DocumentReferenceKind, DocumentReferenceLookup, DocumentType, DocumentTypeRef, EncryptedFor,
    EncryptedForRecipient, EncryptionScheme, GeneratedFrom, GenerationParam,
    IdentityKeyReferenceRequirements, KeyIdReference, KeyReferenceIdentityProperty,
    ListElementReference, LookupChooseFirst, LookupHashKey, LookupKeySource, PropertyReference,
    ReferenceCombinator, ReferenceOperands, SystemFunction, WhereBoundRead, WhereCondition,
    WhereContainsBound, WhereLiteral, WhereLiteralKind, WhereNotEqualBound, WhereOrdering,
    WhereOrderingBound, COMBINABLE_REFERENCE_TARGET_TYPES,
};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::document::property_names::{CREATOR_ID, ID, OWNER_ID};
use crate::identity::Purpose;
use crate::nft::TradeMode;
use crate::util::json_schema::resolve_uri;
use crate::validation::operations::ProtocolValidationOperation;
use crate::ProtocolError;
use indexmap::IndexMap;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::string_encoding::Encoding;
use platform_value::{Identifier, Value, ValueMapHelper};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, BTreeSet};

mod common;
mod v0;
mod v1;
mod v2;
mod v3;

pub(in crate::data_contract) use v3::{
    resolve_derived_index_properties, validate_preallocated_indexes_kept_on_removal,
    validate_summable_off_count_indexes_lossless,
};

const NOT_ALLOWED_SYSTEM_PROPERTIES: [&str; 1] = ["$id"];

/// How a `$ref` to one of the contract's `$defs` starts: `#/$defs/<name>`.
const DEFINITIONS_REF_PREFIX: &str = "#/$defs/";

/// The longest property path a keyword may name: `keyIdProperty`, the
/// `where` entries and the `encryptedFor` paths share it, and the
/// meta-schema states the same bound as `maxLength`.
const MAX_PROPERTY_PATH_LENGTH: usize = 256;

const MAX_INDEXED_STRING_PROPERTY_LENGTH: u16 = 63;
const MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH: u16 = 255;
const MAX_INDEXED_ARRAY_ITEMS: usize = 1024;

impl DocumentType {
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_schema(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        name: &str,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        validation_operations: &mut impl Extend<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .class_method_versions
            .try_from_schema
        {
            0 => DocumentTypeV0::try_from_schema(
                data_contract_id,
                data_contract_system_version,
                contract_config_version,
                name,
                schema,
                schema_defs,
                data_contact_config,
                full_validation,
                validation_operations,
                platform_version,
            )
            .map(|document_type| document_type.into()),
            1 => DocumentTypeV1::try_from_schema(
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
                platform_version,
            )
            .map(|document_type| document_type.into()),
            2 => DocumentType::try_from_schema_v2(
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
                platform_version,
            ),
            3 => DocumentType::try_from_schema_v3(
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
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "try_from_schema".to_string(),
                known_versions: vec![0, 1, 2, 3],
                received: version,
            }),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_values(
    document_properties: &mut IndexMap<String, DocumentProperty>,
    known_required: &BTreeSet<String>,
    known_transient: &BTreeSet<String>,
    prefix: Option<String>,
    property_key: String,
    property_value: &Value,
    root_schema: &Value,
    config: &DataContractConfig,
    platform_version: &PlatformVersion,
) -> Result<(), DataContractError> {
    // Each item carries the `$ref` targets resolved on its path from the root,
    // so a cyclic `$ref` errors instead of expanding forever.
    let mut to_visit: Vec<(Option<String>, String, &Value, Vec<*const Value>)> =
        vec![(prefix, property_key, property_value, Vec::new())];

    while let Some((prefix, property_key, property_value, mut ref_path)) = to_visit.pop() {
        let is_top_level = prefix.is_none();
        let prefixed_property_key = match prefix {
            None => property_key,
            Some(prefix) => [prefix, property_key].join(".").to_owned(),
        };

        let mut inner_properties = property_value.to_btree_ref_string_map()?;

        if let Some(schema_ref) = inner_properties.get_optional_str(property_names::REF)? {
            let referenced_sub_schema = resolve_uri(root_schema, schema_ref)?;
            enter_ref(&mut ref_path, schema_ref, referenced_sub_schema)?;

            inner_properties = referenced_sub_schema.to_btree_ref_string_map()?
        }

        let is_required = known_required.contains(&prefixed_property_key);
        let is_transient = known_transient.contains(&prefixed_property_key);
        let required_since = apply_required_since(
            &inner_properties,
            is_required,
            is_top_level,
            platform_version,
        )?;

        let options: DocumentPropertyTypeParsingOptions = config.into();
        let property_type = match parse_typed_array(&inner_properties, &options, platform_version)?
        {
            Some(typed_array) => typed_array,
            None => DocumentPropertyType::try_from_value_map(&inner_properties, &options)?,
        };

        match property_type {
            DocumentPropertyType::Object(_) => {
                if let Some(properties_as_value) = inner_properties.get(property_names::PROPERTIES)
                {
                    let properties =
                        properties_as_value
                            .as_map()
                            .ok_or(DataContractError::ValueWrongType(
                                "properties must be a map".to_string(),
                            ))?;

                    for (object_property_key, object_property_value) in properties.iter() {
                        let object_property_string = object_property_key
                            .as_text()
                            .ok_or(DataContractError::KeyWrongType(
                                "property key must be a string".to_string(),
                            ))?
                            .to_string();
                        to_visit.push((
                            Some(prefixed_property_key.clone()),
                            object_property_string,
                            object_property_value,
                            ref_path.clone(),
                        ));
                    }
                }
            }
            property_type => {
                let property_type =
                    apply_property_reference(&inner_properties, property_type, platform_version)?;
                let property_type =
                    apply_max_bytes(&inner_properties, property_type, platform_version)?;
                let distinct_from =
                    apply_distinct_from(&inner_properties, &property_type, platform_version)?;
                let encrypted_for =
                    apply_encrypted_for(&inner_properties, &property_type, platform_version)?;
                let generated_from =
                    apply_generated_from(&inner_properties, &property_type, platform_version)?;
                let revealed_reference =
                    apply_revealed_reference(&inner_properties, &property_type, platform_version)?;
                document_properties.insert(
                    prefixed_property_key,
                    DocumentProperty {
                        property_type,
                        required: is_required,
                        transient: is_transient,
                        required_since,
                        distinct_from,
                        encrypted_for,
                        generated_from,
                        revealed_reference,
                    },
                );
            }
        };
    }

    Ok(())
}

/// Records `target`, the schema `schema_ref` resolved to, on the path of
/// `$ref` targets from the root, refusing one already on it: expanding it
/// again would never end.
///
/// Shared by every parser generation without a version gate. Under full
/// validation the depth check runs first and refuses such a cycle in every
/// generation, so this fires only on input the parse never finished before:
/// accepted contracts, their fees and stored contracts parse as they did.
fn enter_ref(
    ref_path: &mut Vec<*const Value>,
    schema_ref: &str,
    target: &Value,
) -> Result<(), DataContractError> {
    let target = target as *const Value;
    if ref_path.contains(&target) {
        return Err(DataContractError::InvalidURI(format!(
            "the ref '{}' contains cycles",
            schema_ref
        )));
    }
    ref_path.push(target);
    Ok(())
}

// TODO: This is quite big
/// `ref_path` holds the `$ref` targets resolved on the path from the root to
/// this property; it is left as it was found on success.
#[allow(clippy::too_many_arguments)]
fn insert_values_nested(
    document_properties: &mut IndexMap<String, DocumentProperty>,
    known_required: &BTreeSet<String>,
    known_transient: &BTreeSet<String>,
    is_top_level: bool,
    property_key: String,
    property_value: &Value,
    root_schema: &Value,
    ref_path: &mut Vec<*const Value>,
    config: &DataContractConfig,
    platform_version: &PlatformVersion,
) -> Result<(), DataContractError> {
    let ref_path_len = ref_path.len();
    let mut inner_properties = property_value.to_btree_ref_string_map()?;

    if let Some(schema_ref) = inner_properties.get_optional_str(property_names::REF)? {
        let referenced_sub_schema = resolve_uri(root_schema, schema_ref)?;
        enter_ref(ref_path, schema_ref, referenced_sub_schema)?;

        inner_properties = referenced_sub_schema.to_btree_ref_string_map()?;
    }

    let is_required = known_required.contains(&property_key);

    let is_transient = known_transient.contains(&property_key);

    let required_since = apply_required_since(
        &inner_properties,
        is_required,
        is_top_level,
        platform_version,
    )?;

    let options: DocumentPropertyTypeParsingOptions = config.into();
    let property_type = match parse_typed_array(&inner_properties, &options, platform_version)? {
        Some(typed_array) => typed_array,
        None => DocumentPropertyType::try_from_value_map(&inner_properties, &options)?,
    };

    let property_type = match property_type {
        DocumentPropertyType::Object(_) => {
            let mut nested_properties = IndexMap::new();
            if let Some(properties_as_value) = inner_properties.get(property_names::PROPERTIES) {
                let properties =
                    properties_as_value
                        .as_map()
                        .ok_or(DataContractError::ValueWrongType(
                            "properties must be a map".to_string(),
                        ))?;

                // Nested properties are emitted below in source-map order (the
                // `properties.iter()` loop), and that `IndexMap` insertion order is
                // consensus-observable: historical contracts were committed in source-map
                // order, so re-sorting nested properties by `position` would soft-fork any
                // contract whose source order differs from its position order. A previous
                // `position`-based `sort_by` here was dead code (its sorted result was never
                // read) and read `position` with `.expect()`, which could panic on adversarial
                // schema input during block execution. Removed (ordering unchanged). Do NOT
                // reintroduce a nested-property sort — even a correct one — nor a panicking
                // `position` read here.

                // Create a new set with the prefix removed from the keys: an entry for
                // a member of this object is the object's name, one separator byte, then
                // the member's own entry.
                //
                // Every protocol version reaches this helper, so the match stays
                // output-identical to the byte-offset slice it replaced: `str::get`
                // returns that same slice wherever the slice was valid, and `None` (no
                // member entry) elsewhere. Requiring the separator to be '.' would change
                // how some schemas that parse today are read, so it needs a new
                // generation.
                let stripped_required: BTreeSet<String> = known_required
                    .iter()
                    .filter_map(|key| {
                        key.strip_prefix(property_key.as_str())
                            .and_then(|rest| rest.get(1..))
                            .map(str::to_string)
                    })
                    .collect();

                // Matched exactly like `stripped_required` above
                let stripped_transient: BTreeSet<String> = known_transient
                    .iter()
                    .filter_map(|key| {
                        key.strip_prefix(property_key.as_str())
                            .and_then(|rest| rest.get(1..))
                            .map(str::to_string)
                    })
                    .collect();

                for (object_property_key, object_property_value) in properties.iter() {
                    let object_property_string = object_property_key
                        .as_text()
                        .ok_or(DataContractError::KeyWrongType(
                            "property key must be a string".to_string(),
                        ))?
                        .to_string();

                    insert_values_nested(
                        &mut nested_properties,
                        &stripped_required,
                        &stripped_transient,
                        false,
                        object_property_string,
                        object_property_value,
                        root_schema,
                        ref_path,
                        config,
                        platform_version,
                    )?;
                }
            }

            DocumentPropertyType::Object(nested_properties)
        }
        property_type => property_type,
    };
    ref_path.truncate(ref_path_len);

    let property_type =
        apply_property_reference(&inner_properties, property_type, platform_version)?;
    let property_type = apply_max_bytes(&inner_properties, property_type, platform_version)?;
    let distinct_from = apply_distinct_from(&inner_properties, &property_type, platform_version)?;
    let encrypted_for = apply_encrypted_for(&inner_properties, &property_type, platform_version)?;
    let generated_from = apply_generated_from(&inner_properties, &property_type, platform_version)?;
    let revealed_reference =
        apply_revealed_reference(&inner_properties, &property_type, platform_version)?;

    document_properties.insert(
        property_key,
        DocumentProperty {
            property_type,
            required: is_required,
            transient: is_transient,
            required_since,
            distinct_from,
            encrypted_for,
            generated_from,
            revealed_reference,
        },
    );

    Ok(())
}

/// Reads a `distinctFrom` declaration off an identifier property, or off the
/// `items` of a typed array of identifiers: what its value (every element's
/// value) must differ from, the document's `$ownerId` or another property of
/// the same document type. Non-identifier properties cannot carry it.
///
/// Versioned on `apply_distinct_from` in the platform version's document type
/// schema versions. `None` selects the behavior of the versions that predate
/// the keyword: it is ignored entirely, so their parses stay byte-for-byte
/// identical to what they always produced.
///
/// The named property is checked against the rest of the document type once
/// every property is parsed, by [`validate_distinct_from_targets`].
fn apply_distinct_from(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: &DocumentPropertyType,
    platform_version: &PlatformVersion,
) -> Result<Option<DistinctFrom>, DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_distinct_from
    {
        None => Ok(None),
        Some(0) => apply_distinct_from_v0(inner_properties, property_type),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "apply_distinct_from version {version} is not supported"
        ))),
    }
}

fn apply_distinct_from_v0(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: &DocumentPropertyType,
) -> Result<Option<DistinctFrom>, DataContractError> {
    // A typed array carries the declaration on its `items`: every element must
    // differ from the named value, so the elements must be identifiers
    if let DocumentPropertyType::TypedArray(typed_array) = property_type {
        let Some(distinct_from_value) = typed_array_items_keyword(
            inner_properties,
            property_names::DISTINCT_FROM,
            "applies to every element",
        )?
        else {
            return Ok(None);
        };
        if !matches!(
            *typed_array.item_type,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_)
        ) {
            return Err(DataContractError::InvalidContractStructure(
                "distinctFrom is only allowed on identifier elements of a typed array".to_string(),
            ));
        }
        let name = distinct_from_value.as_text().ok_or_else(|| {
            DataContractError::InvalidContractStructure(
                "distinctFrom must be a string naming $ownerId or a property of the same \
                 document type"
                    .to_string(),
            )
        })?;
        return DistinctFrom::from_wire_name(name).map(Some);
    }

    let Some(distinct_from_value) = inner_properties.get(property_names::DISTINCT_FROM) else {
        return Ok(None);
    };

    if !matches!(
        property_type,
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_)
    ) {
        return Err(DataContractError::InvalidContractStructure(
            "distinctFrom is only allowed on identifier properties".to_string(),
        ));
    }

    let name = distinct_from_value.as_text().ok_or_else(|| {
        DataContractError::InvalidContractStructure(
            "distinctFrom must be a string naming $ownerId or a property of the same document \
             type"
                .to_string(),
        )
    })?;

    DistinctFrom::from_wire_name(name).map(Some)
}

/// Checks every `distinctFrom` declaration of a document type against the
/// rest of its properties, once they are all parsed: a named property must
/// exist, must be an identifier (the only kind the value can be compared
/// with), and must not be the declaring property itself. `$ownerId` needs no
/// check, every document has one.
///
/// Runs on every parse, validating or not: the rule is a property of the
/// document type, and the write-time check reads the target through the
/// same flattened map, so a target that does not resolve here could never
/// be judged there.
///
/// Versioned on `apply_distinct_from`, the version that parsed the
/// declarations: `None` predates the keyword, so there is nothing to check.
fn validate_distinct_from_targets(
    flattened_properties: &IndexMap<String, DocumentProperty>,
    document_type_name: &str,
    platform_version: &PlatformVersion,
) -> Result<(), DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_distinct_from
    {
        None => Ok(()),
        Some(0) => validate_distinct_from_targets_v0(flattened_properties, document_type_name),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "validate_distinct_from_targets version {version} is not supported"
        ))),
    }
}

fn validate_distinct_from_targets_v0(
    flattened_properties: &IndexMap<String, DocumentProperty>,
    document_type_name: &str,
) -> Result<(), DataContractError> {
    for (path, property) in flattened_properties {
        let Some(DistinctFrom::Property(target)) = &property.distinct_from else {
            continue;
        };
        if target == path {
            return Err(DataContractError::InvalidContractStructure(format!(
                "document type \"{document_type_name}\" property \"{path}\" declares distinctFrom \
                 itself: name $ownerId or another identifier property of the document type"
            )));
        }
        let Some(target_property) = flattened_properties.get(target) else {
            if names_an_object(flattened_properties, target) {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "document type \"{document_type_name}\" property \"{path}\" declares distinctFrom \
                     \"{target}\", which is an object, not an identifier property: name one of \
                     its identifier members"
                )));
            }
            return Err(DataContractError::InvalidContractStructure(format!(
                "document type \"{document_type_name}\" property \"{path}\" declares distinctFrom \
                 \"{target}\", but the document type has no property at that path"
            )));
        };
        if !matches!(
            target_property.property_type,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_)
        ) {
            return Err(DataContractError::InvalidContractStructure(format!(
                "document type \"{document_type_name}\" property \"{path}\" declares distinctFrom \
                 \"{target}\", which is not an identifier property"
            )));
        }
    }
    Ok(())
}

/// Whether `path`, missing from the flattened map, names an object of the
/// document type: objects are not in the flattened map, only their members are.
fn names_an_object(flattened_properties: &IndexMap<String, DocumentProperty>, path: &str) -> bool {
    flattened_properties.keys().any(|key| {
        key.strip_prefix(path)
            .is_some_and(|rest| rest.starts_with('.'))
    })
}

/// The value of an element keyword on the `items` of a typed array property,
/// refused on the array itself: the keyword binds every element, so it belongs
/// on the items. `binds` finishes the refusal ("applies to every element").
fn typed_array_items_keyword<'a>(
    inner_properties: &BTreeMap<String, &'a Value>,
    keyword: &str,
    binds: &str,
) -> Result<Option<&'a Value>, DataContractError> {
    if inner_properties.contains_key(keyword) {
        return Err(DataContractError::InvalidContractStructure(format!(
            "{keyword} on a typed array belongs on its items, where it {binds}"
        )));
    }
    let Some(items) = inner_properties.get(property_names::ITEMS) else {
        return Ok(None);
    };
    Ok(items.to_btree_ref_string_map()?.get(keyword).copied())
}

/// Folds a `maxBytes` declaration into a string property's sizes, or, declared
/// on the `items` of a typed array of strings, into the element type's sizes:
/// the most UTF-8 bytes the value (every element) may take. `maxLength` counts
/// characters, which are up to four bytes each, so it cannot bound the stored
/// size on its own.
///
/// Versioned on `apply_max_bytes` in the platform version's document type
/// schema versions. `None` selects the behavior of the versions that predate
/// the keyword: it is ignored entirely, so their parses stay byte-for-byte
/// identical to what they always produced.
fn apply_max_bytes(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: DocumentPropertyType,
    platform_version: &PlatformVersion,
) -> Result<DocumentPropertyType, DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_max_bytes
    {
        None => Ok(property_type),
        Some(0) => apply_max_bytes_v0(inner_properties, property_type),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "apply_max_bytes version {version} is not supported"
        ))),
    }
}

fn apply_max_bytes_v0(
    inner_properties: &BTreeMap<String, &Value>,
    mut property_type: DocumentPropertyType,
) -> Result<DocumentPropertyType, DataContractError> {
    let declared = if matches!(property_type, DocumentPropertyType::TypedArray(_)) {
        typed_array_items_keyword(
            inner_properties,
            property_names::MAX_BYTES,
            "bounds every element",
        )?
    } else {
        inner_properties.get(property_names::MAX_BYTES).copied()
    };
    let Some(max_bytes_value) = declared else {
        return Ok(property_type);
    };
    let sizes = match &mut property_type {
        DocumentPropertyType::String(sizes) => sizes,
        DocumentPropertyType::TypedArray(typed_array) => match typed_array.item_type.as_mut() {
            DocumentPropertyType::String(sizes) => sizes,
            _ => {
                return Err(DataContractError::InvalidContractStructure(
                    "maxBytes is only allowed on string elements of a typed array".to_string(),
                ))
            }
        },
        _ => {
            return Err(DataContractError::InvalidContractStructure(
                "maxBytes is only allowed on string properties".to_string(),
            ))
        }
    };
    sizes.max_bytes = Some(parse_max_bytes(max_bytes_value, sizes.min_length)?);
    Ok(property_type)
}

/// A `maxBytes` bound: 1 to 65535, and no lower than `minLength`, since a
/// string of `minLength` characters is at least that many bytes and a bound
/// below it would refuse every value.
fn parse_max_bytes(value: &Value, min_length: Option<u16>) -> Result<u16, DataContractError> {
    let max_bytes = value
        .to_integer::<u16>()
        .ok()
        .filter(|max_bytes| *max_bytes > 0)
        .ok_or_else(|| {
            DataContractError::InvalidContractStructure(
                "maxBytes must be an integer from 1 to 65535".to_string(),
            )
        })?;
    if let Some(min_length) = min_length {
        if max_bytes < min_length {
            return Err(DataContractError::InvalidContractStructure(format!(
                "maxBytes {max_bytes} is below minLength {min_length}: a string of \
                 {min_length} characters is at least {min_length} bytes, so no value could \
                 be valid"
            )));
        }
    }
    Ok(max_bytes)
}

/// Folds a `refersTo` declaration into the property type: an identifier property
/// with `refersTo` becomes `IdentifierWithReference(target)`, and a `u32` key id
/// property with an `identityPublicKey` declaration naming `identityProperty`
/// becomes `KeyIdWithReference(identity property)`. No other property can carry
/// `refersTo`.
/// The typed array parser calls it on an identifier element's `items`
/// schema, so an element reference is read by exactly this code; on the
/// array itself the declaration is refused, it belongs on the items.
///
/// Versioned on `apply_property_reference` in the platform version's document
/// type schema versions. `None` selects the behavior of the versions that
/// predate the keyword: it is ignored entirely, so their parses stay
/// byte-for-byte identical to what they always produced.
pub(in crate::data_contract::document_type::class_methods) fn apply_property_reference(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: DocumentPropertyType,
    platform_version: &PlatformVersion,
) -> Result<DocumentPropertyType, DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_property_reference
    {
        None => Ok(property_type),
        Some(0) => apply_property_reference_v0(inner_properties, property_type),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "apply_property_reference version {version} is not supported"
        ))),
    }
}

fn apply_property_reference_v0(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: DocumentPropertyType,
) -> Result<DocumentPropertyType, DataContractError> {
    let Some(refers_to_value) = inner_properties.get(property_names::REFERS_TO) else {
        return Ok(property_type);
    };

    // A string or byte array property's value is not an id: a declaration
    // with `findBy` on one is read by `apply_revealed_reference`, which admits
    // only a document reference whose `findBy` reads the value. Every other
    // declaration on one is refused below
    if is_revealed_declaration(&property_type, refers_to_value) {
        return Ok(property_type);
    }

    // A typed array only exists from protocol version 14, where its element
    // reference is read off the items by the typed array parser
    if matches!(property_type, DocumentPropertyType::TypedArray(_)) {
        let message = if is_identity_public_key_reference(refers_to_value)? {
            "identityPublicKey refersTo is not allowed on a typed array or on its elements: it \
             pairs one key id with the reference, which cannot pair with many elements"
        } else {
            "refersTo on a typed array belongs on its items, where it applies to every element"
        };
        return Err(DataContractError::InvalidContractStructure(
            message.to_string(),
        ));
    }

    let refers_to_map = refers_to_value.to_btree_ref_string_map()?;

    // A reference expression in place of a single target: `anyOf` or `allOf`
    // as the declaration's one key. It sits on an identifier, as every leaf
    // it admits does
    if let Some((combinator, operands_value)) = reference_combinator_of(&refers_to_map) {
        if !matches!(
            property_type,
            DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_)
        ) {
            return Err(DataContractError::InvalidContractStructure(format!(
                "refersTo {} is only allowed on identifier properties",
                combinator.wire_name()
            )));
        }
        return Ok(DocumentPropertyType::IdentifierWithReference(
            parse_reference_expression(&refers_to_map, combinator, operands_value, "")?,
        ));
    }

    let reference_type = refers_to_map
        .get_str(property_names::TYPE)
        .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;

    validate_reference_target_keys(&refers_to_map, reference_type)?;

    // A key reference declared on the key id property itself names whose key
    // it is through `identityProperty`; it is the one form that sits on a
    // non-identifier property
    if let Some(identity_property_value) = refers_to_map.get(property_names::IDENTITY_PROPERTY) {
        return apply_key_id_reference_v0(
            inner_properties,
            &refers_to_map,
            reference_type,
            identity_property_value,
            property_type,
        );
    }

    if !matches!(
        property_type,
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_)
    ) {
        return Err(DataContractError::InvalidContractStructure(
            "refersTo is only allowed on identifier properties, except an identityPublicKey \
             reference with identityProperty, which sits on the key id property, and a document \
             reference whose findBy reads a string or byte array property's value"
                .to_string(),
        ));
    }

    Ok(DocumentPropertyType::IdentifierWithReference(
        parse_reference_target(&refers_to_map, reference_type)?,
    ))
}

/// Reads the `refersTo` of a string or byte array property: its value is not
/// an id, so the one declaration it may carry is a document reference, of
/// either kind, found by a `findBy` reading the value: as `"."`, filling an
/// index property holding the same kind of value (a name a document is unique
/// by), or as a param of a function computing the key, a commitment the value
/// is revealed to (a salt a preorder hashed). `None` for every other property
/// type, and for a declaration without `findBy`, which
/// [`apply_property_reference`] folds into the type or refuses.
///
/// Versioned on `apply_property_reference`, the gate of the declaration it
/// reads, and dispatched with it, so the declarations one skips the other
/// reads: `None` ignores the keyword, as it does on an identifier.
fn apply_revealed_reference(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: &DocumentPropertyType,
    platform_version: &PlatformVersion,
) -> Result<Option<DocumentPropertyReferenceTarget>, DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_property_reference
    {
        None => Ok(None),
        Some(0) => apply_revealed_reference_v0(inner_properties, property_type),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "apply_property_reference version {version} is not supported"
        ))),
    }
}

/// Whether `property_type` carrying `refers_to_value` is a revealed reference:
/// a string or byte array whose declaration has `findBy`. Version 0 of
/// `apply_property_reference` leaves exactly these to
/// [`apply_revealed_reference_v0`], which parses exactly these.
fn is_revealed_declaration(property_type: &DocumentPropertyType, refers_to_value: &Value) -> bool {
    matches!(
        property_type,
        DocumentPropertyType::String(_) | DocumentPropertyType::ByteArray(_)
    ) && declares_find_by(refers_to_value)
}

fn apply_revealed_reference_v0(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: &DocumentPropertyType,
) -> Result<Option<DocumentPropertyReferenceTarget>, DataContractError> {
    let Some(refers_to_value) = inner_properties
        .get(property_names::REFERS_TO)
        .filter(|refers_to_value| is_revealed_declaration(property_type, refers_to_value))
    else {
        return Ok(None);
    };
    let refused = || {
        DataContractError::InvalidContractStructure(
            "refersTo on a string or byte array property must be a permanentDocument or \
             deletableDocument reference found by a findBy reading the value, as \".\" or a \
             param of its function: the value is not an id"
                .to_string(),
        )
    };
    let refers_to_map = refers_to_value.to_btree_ref_string_map()?;
    if reference_combinator_of(&refers_to_map).is_some()
        || refers_to_map.contains_key(property_names::IDENTITY_PROPERTY)
    {
        return Err(refused());
    }
    let reference_type = refers_to_map
        .get_str(property_names::TYPE)
        .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;
    if !matches!(reference_type, "permanentDocument" | "deletableDocument") {
        return Err(refused());
    }
    validate_reference_target_keys(&refers_to_map, reference_type)?;
    let target = parse_reference_target(&refers_to_map, reference_type)?;
    // A key found by `findBy`, not a list: an `inList` holds ids. That the key
    // reads the value exactly once, as `"."` or by the property's path in a
    // function's params, is a referring-side rule, checked once the type is
    // parsed, and that `"."` fills an index property of the value's kind a
    // referenced-side one
    let found_by_find_by = target
        .as_any_document_reference()
        .and_then(|declaration| declaration.lookup)
        .is_some();
    if !found_by_find_by {
        return Err(refused());
    }
    Ok(Some(target))
}

/// Whether a `refersTo` declaration is a single target with `findBy`, the one
/// form a string or byte array property may carry.
fn declares_find_by(refers_to_value: &Value) -> bool {
    refers_to_value
        .to_btree_ref_string_map()
        .is_ok_and(|declaration| declaration.contains_key(property_names::FIND_BY))
}

/// The combinator a `refersTo` declaration (or one operand of an expression)
/// names through its key, `anyOf` or `allOf`, with the key's value; `None` for a
/// single target.
fn reference_combinator_of<'a>(
    declaration: &BTreeMap<String, &'a Value>,
) -> Option<(ReferenceCombinator, &'a Value)> {
    ReferenceCombinator::ALL.into_iter().find_map(|combinator| {
        declaration
            .get(combinator.wire_name())
            .map(|operands| (combinator, *operands))
    })
}

/// A reference expression: `declaration` names `combinator` and holds nothing
/// else, and `operands_value` lists two or more operands, each a leaf (see
/// [`parse_reference_expression_leaf`]) or an expression of the other
/// combinator (an `anyOf` directly inside an `anyOf` says what one flat list
/// says, and so does an `allOf` inside an `allOf`). `path` is where the
/// expression sits in the declaration (`anyOf[1]`, empty at the top), for the
/// errors.
///
/// These are rules of the declaration's shape, checked on every parse. The
/// limits on it (`SystemLimits::max_reference_operands` per list,
/// `max_reference_expression_depth` for the nesting) and that no two operands
/// of a list are alike, which needs the declaring contract's id to see a
/// target naming it explicitly as the one that omits it, are checked under
/// full validation with the other reference limits.
fn parse_reference_expression(
    declaration: &BTreeMap<String, &Value>,
    combinator: ReferenceCombinator,
    operands_value: &Value,
    path: &str,
) -> Result<DocumentPropertyReferenceTarget, DataContractError> {
    let name = combinator.wire_name();
    let here = if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}.{name}")
    };
    if declaration.len() != 1 {
        return Err(DataContractError::InvalidContractStructure(format!(
            "refersTo {here} declares nothing beside {name}: every other key belongs to one of \
             its operands"
        )));
    }
    let Some(operand_values) = operands_value.as_array() else {
        return Err(DataContractError::InvalidContractStructure(format!(
            "refersTo {here} must be a list of operands"
        )));
    };
    if operand_values.len() < 2 {
        return Err(DataContractError::InvalidContractStructure(format!(
            "refersTo {here} must list at least two operands: a single one is declared on its own"
        )));
    }

    let mut operands = Vec::with_capacity(operand_values.len());
    for (index, operand_value) in operand_values.iter().enumerate() {
        let operand_path = format!("{here}[{index}]");
        let operand_map = operand_value.to_btree_ref_string_map()?;
        let operand = match reference_combinator_of(&operand_map) {
            Some((inner, _)) if inner == combinator => {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "refersTo {operand_path} is an {name} directly inside an {name}, which says \
                     what one flat list says: list its operands in the outer {name}"
                )));
            }
            Some((inner, inner_operands)) => {
                parse_reference_expression(&operand_map, inner, inner_operands, &operand_path)?
            }
            None => parse_reference_expression_leaf(&operand_map, &operand_path)?,
        };
        operands.push(operand);
    }
    let operands = ReferenceOperands::new(operands);
    Ok(match combinator {
        ReferenceCombinator::AnyOf => DocumentPropertyReferenceTarget::AnyOf(operands),
        ReferenceCombinator::AllOf => DocumentPropertyReferenceTarget::AllOf(operands),
    })
}

/// A leaf of a reference expression, at `path`: an ordinary target declaration
/// of a type in [`COMBINABLE_REFERENCE_TARGET_TYPES`], never the key id form,
/// parsed and checked exactly as the same declaration on its own.
fn parse_reference_expression_leaf(
    declaration: &BTreeMap<String, &Value>,
    path: &str,
) -> Result<DocumentPropertyReferenceTarget, DataContractError> {
    let reference_type = declaration
        .get_str(property_names::TYPE)
        .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;
    refuse_replaced_reference_keywords(declaration, reference_type)?;
    // A deletableDocument leaf found by `findBy` is an existence check like
    // the others; it only asks every replace to re-validate the expression
    let deletable_lookup =
        reference_type == "deletableDocument" && declaration.contains_key(property_names::FIND_BY);
    let refusal = if deletable_lookup {
        None
    } else {
        expression_leaf_refusal_reason(reference_type)
    };
    if let Some(reason) = refusal {
        return Err(DataContractError::InvalidContractStructure(format!(
            "refersTo {path} is a reference of type {reference_type}, which a reference \
             expression does not take: {reason}"
        )));
    }
    if declaration.contains_key(property_names::IDENTITY_PROPERTY) {
        return Err(DataContractError::InvalidContractStructure(format!(
            "refersTo {path}: {reference_type} refersTo does not take identityProperty"
        )));
    }
    validate_reference_target_keys(declaration, reference_type)?;
    parse_reference_target(declaration, reference_type)
}

/// Why a reference expression does not take a leaf of `reference_type`, `None`
/// for the types it takes ([`COMBINABLE_REFERENCE_TARGET_TYPES`]). Those are
/// existence checks, with a `where` on a document, against
/// entities that can never be deleted, so an expression of them holds for good
/// once it holds and a replace re-validates it only when the value or a
/// property bound to a leaf changed, as it does a single target. The others
/// carry semantics that do not compose with other operands.
fn expression_leaf_refusal_reason(reference_type: &str) -> Option<&'static str> {
    match reference_type {
        _ if COMBINABLE_REFERENCE_TARGET_TYPES.contains(&reference_type) => None,
        "deletableDocument" => Some(
            "by id it is re-validated on every replace and may be cleared once its document is \
             deleted, which assumes the property refers to that one target; declare it with \
             findBy to combine it",
        ),
        "moderatedDocument" => Some(
            "its value may resolve to a moderator's removal record rather than a document, \
             which no other operand accounts for",
        ),
        "identityPublicKey" => {
            Some("it pairs the value with a key id property, which no other operand reads")
        }
        "contract" => Some(
            "its requirements are gates judged against the block time and the writer rather \
             than an existence check, and a contract id is never also an identity or document id",
        ),
        "token" => Some("a token id is never also an identity or document id"),
        _ => Some("it is not a refersTo type"),
    }
}

/// Refuses the keywords a protocol version 14 beta spelled a document
/// reference with, on every parse, so a contract written with them never
/// loads with another meaning: `lookup`, now `findBy` (the index is the one
/// whose properties it names); `propertyAgreement`, now `where`, keyed by the
/// referenced property; and the `listElement` type, now a `permanentDocument`
/// found by its `$id` with `inList`.
fn refuse_replaced_reference_keywords(
    refers_to_map: &BTreeMap<String, &Value>,
    reference_type: &str,
) -> Result<(), DataContractError> {
    let refusal = if reference_type == property_names::REPLACED_LIST_ELEMENT {
        Some(
            "refersTo type listElement was replaced: declare a permanentDocument reference with \
             findBy { \"$id\": <the property holding the document's id> } and inList",
        )
    } else if refers_to_map.contains_key(property_names::REPLACED_LOOKUP) {
        Some(
            "refersTo lookup was replaced by findBy: map every property of the unique index to \
             its source, { \"<index property>\": <source> }, and leave the index name out; a \
             propertyAgreement function goes in findBy too",
        )
    } else if refers_to_map.contains_key(property_names::REPLACED_PROPERTY_AGREEMENT) {
        Some(
            "refersTo propertyAgreement was replaced by where, keyed by the referenced \
             document's property: { \"<referenced property>\": \"<referring property or \
             $ownerId>\" }",
        )
    } else {
        None
    };
    match refusal {
        Some(refusal) => Err(DataContractError::InvalidContractStructure(
            refusal.to_string(),
        )),
        None => Ok(()),
    }
}

/// Checks the keys of a single target declaration against its `type`: the
/// keys that belong to one kind of target alone (`contractRequirements`,
/// `keyRequirements`, `findBy`, `where`, `inList`) are refused on the others,
/// and the keywords `findBy` and `where` replaced everywhere.
fn validate_reference_target_keys(
    refers_to_map: &BTreeMap<String, &Value>,
    reference_type: &str,
) -> Result<(), DataContractError> {
    refuse_replaced_reference_keywords(refers_to_map, reference_type)?;

    // Requirements on the referenced contract belong to contract references alone
    if reference_type != "contract"
        && refers_to_map.contains_key(property_names::CONTRACT_REQUIREMENTS)
    {
        return Err(DataContractError::InvalidContractStructure(format!(
            "{} refersTo does not take contractRequirements",
            reference_type
        )));
    }

    // Requirements on the referenced key belong to identity key references alone
    if reference_type != "identityPublicKey"
        && refers_to_map.contains_key(property_names::KEY_REQUIREMENTS)
    {
        return Err(DataContractError::InvalidContractStructure(format!(
            "{} refersTo does not take keyRequirements",
            reference_type
        )));
    }

    // `findBy` finds a referenced DOCUMENT through a unique index of its type,
    // of either kind: a permanent one never dangles, and a deletable one is
    // re-validated on every replace, since a key into a deletable type may find
    // a new document once the one it found is deleted. A moderated one takes no
    // `findBy`: a moderator's removal frees the document's unique index keys, so
    // a key could come to find another document while the reference is held to
    // the removed one's record. `where` compares a referenced DOCUMENT's values,
    // of any kind. The other targets are found by the value itself and have no
    // document body
    if refers_to_map.contains_key(property_names::FIND_BY)
        && !matches!(reference_type, "permanentDocument" | "deletableDocument")
    {
        let reason = if reference_type == "moderatedDocument" {
            "a moderator's removal frees the unique index keys of the document it found, so \
             the same key could find another document; refer to the document by its id"
        } else {
            "it is only allowed on permanentDocument and deletableDocument references"
        };
        return Err(DataContractError::InvalidContractStructure(format!(
            "{reference_type} refersTo does not take {}: {reason}",
            property_names::FIND_BY
        )));
    }
    if refers_to_map.contains_key(property_names::WHERE)
        && !matches!(
            reference_type,
            "permanentDocument" | "moderatedDocument" | "deletableDocument"
        )
    {
        return Err(DataContractError::InvalidContractStructure(format!(
            "{reference_type} refersTo does not take {}: it is only allowed on \
             permanentDocument, moderatedDocument and deletableDocument references",
            property_names::WHERE
        )));
    }

    // `inList` names a list on the document found by its `$id`, whose value
    // must stay an element for good: the document is never deleted and the
    // list never changes, which a permanent document promises
    if refers_to_map.contains_key(property_names::IN_LIST) && reference_type != "permanentDocument"
    {
        return Err(DataContractError::InvalidContractStructure(format!(
            "{reference_type} refersTo does not take inList: only a permanentDocument reference, \
             whose document is never deleted, names a list the value must be in"
        )));
    }

    // `minimumAgeBlocks` and `chooseFirst` sit in `constraints`, not beside it
    if let Some(keyword) = [
        property_names::MINIMUM_AGE_BLOCKS,
        property_names::CHOOSE_FIRST,
    ]
    .into_iter()
    .find(|keyword| refers_to_map.contains_key(*keyword))
    {
        return Err(DataContractError::InvalidContractStructure(format!(
            "{reference_type} refersTo declares {keyword} inside its constraints, \
             {{ \"constraints\": {{ \"{keyword}\": ... }} }}, not beside findBy"
        )));
    }
    // `constraints` and `consume` describe the document `findBy` finds when the
    // referring document is created: they sit beside `findBy` and need it. That
    // the reference is judged on the create alone (a function in `findBy`, or a
    // declaring type whose documents are never replaced) needs the declaring
    // type, and is checked once it is parsed (`validate_reference_lookup_sources`);
    // `inList` refuses both (`parse_list_element_reference`)
    if let Some(keyword) = [property_names::CONSTRAINTS, property_names::CONSUME]
        .into_iter()
        .find(|keyword| refers_to_map.contains_key(*keyword))
    {
        if !refers_to_map.contains_key(property_names::FIND_BY) {
            return Err(DataContractError::InvalidContractStructure(format!(
                "{reference_type} refersTo does not take {keyword} without findBy: it describes \
                 the document findBy finds when the referring document is created"
            )));
        }
    }

    Ok(())
}

/// The document type a document reference of `reference_type` names: its
/// `contractId`, absent for a document type of the declaring contract itself,
/// and its `documentType`.
fn parse_document_reference_type(
    refers_to_map: &BTreeMap<String, &Value>,
    reference_type: &str,
) -> Result<(Option<Identifier>, String), DataContractError> {
    let contract_id = refers_to_map
        .get(property_names::CONTRACT_ID)
        .map(|value| {
            value
                .to_identifier()
                .map_err(|e| DataContractError::ValueWrongType(e.to_string()))
        })
        .transpose()?;

    let document_type_name = refers_to_map
        .get_str(property_names::DOCUMENT_TYPE)
        .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;

    if document_type_name.is_empty() || document_type_name.len() > 64 {
        return Err(DataContractError::InvalidContractStructure(format!(
            "{reference_type} refersTo documentType must be between 1 and 64 characters"
        )));
    }
    Ok((contract_id, document_type_name.to_string()))
}

/// Parses a single target declaration of `reference_type` whose keys
/// [`validate_reference_target_keys`] accepted: the target of an identifier
/// property, or one target of an `anyOf`.
fn parse_reference_target(
    refers_to_map: &BTreeMap<String, &Value>,
    reference_type: &str,
) -> Result<DocumentPropertyReferenceTarget, DataContractError> {
    let target = match reference_type {
        "identity" => DocumentPropertyReferenceTarget::Identity,
        "contract" => DocumentPropertyReferenceTarget::Contract {
            contract_requirements: parse_contract_reference_requirements(refers_to_map)?,
        },
        "token" => DocumentPropertyReferenceTarget::Token,
        // A moderated document is found by the value as its id alone (the keys
        // refused `findBy` and `inList`), with the declaration shape the other
        // two share; whether the referenced document type leaves its documents
        // only to a moderator's recorded removal is checked against state at
        // contract registration
        //
        // A writer gate (a `where` entry valued `"$ownerId"`) is asked about on
        // every replace, and once a moderator removes the document only its owner
        // and its id are sure to be in its removal record (the fields a type keeps,
        // `moderatorAbilities.deleteKeepsFields`, are left out where the document
        // held none, and the type may be another contract's): a gate on any other
        // property could refuse every later replace, for good on a property that
        // can not be repointed or cleared, so an equality or a `notEqual` with the
        // writer may compare only those two. A `contains` of the writer is the one
        // gate admitted here on another property, a typed array, which no system
        // identifier is: a list the document held none of is read as empty, as
        // against the document in state, so the gate refuses after a removal what
        // it refused before; registration, with the referenced type in hand, keeps
        // the list to one every removal record holds
        "moderatedDocument" => {
            let (contract_id, document_type_name) =
                parse_document_reference_type(refers_to_map, reference_type)?;
            let (property_agreement, where_conditions) =
                parse_where(refers_to_map, reference_type)?;
            let gates = property_agreement
                .iter()
                .filter(|(referring_property, _)| {
                    is_referring_system_agreement_property(referring_property)
                })
                .map(|(_, referenced_property)| referenced_property)
                .chain(
                    where_conditions
                        .iter()
                        .filter(|(_, condition)| {
                            matches!(
                                condition,
                                WhereCondition::NotEqual(WhereNotEqualBound::Writer)
                            )
                        })
                        .map(|(referenced_property, _)| referenced_property),
                );
            if let Some(referenced_property) = gates
                .into_iter()
                .find(|referenced_property| !matches!(referenced_property.as_str(), OWNER_ID | ID))
            {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "moderatedDocument refersTo where compares the writer with \
                     \"{referenced_property}\": a writer is checked on every replace, and \
                     once a moderator removes the document only its $ownerId and $id are sure \
                     to be in its removal record, so the writer may be compared with those \
                     alone"
                )));
            }
            DocumentPropertyReferenceTarget::ModeratedDocument {
                contract_id,
                document_type_name,
                property_agreement,
                where_conditions,
            }
        }
        // The two document targets share one declaration shape; they differ in
        // whether the referenced document type must forbid deletion, checked
        // against state at contract registration. The document is found by the
        // value as its id, by `findBy` through a unique index, or, for a list
        // the value must be in (`inList`, permanent only), by a `findBy` naming
        // its `$id` from another property
        document_target @ ("permanentDocument" | "deletableDocument") => {
            let (contract_id, document_type_name) =
                parse_document_reference_type(refers_to_map, document_target)?;

            let (property_agreement, where_conditions) =
                parse_where(refers_to_map, document_target)?;
            let find_by = refers_to_map.get(property_names::FIND_BY);

            if refers_to_map.contains_key(property_names::IN_LIST) {
                let Some(find_by) = find_by else {
                    return Err(DataContractError::InvalidContractStructure(
                        "permanentDocument refersTo inList needs findBy { \"$id\": <the property \
                         holding the document's id> }: the value is an element of the list, not \
                         the id of the document holding it"
                            .to_string(),
                    ));
                };
                return Ok(DocumentPropertyReferenceTarget::ListElement(
                    parse_list_element_reference(
                        refers_to_map,
                        find_by,
                        contract_id,
                        document_type_name,
                        property_agreement,
                        where_conditions,
                    )?,
                ));
            }

            // A lookup is its own variant, so an id reference keeps its shape
            // (and its encoding in the reference errors)
            let Some(find_by) = find_by else {
                return Ok(match document_target {
                    "permanentDocument" => DocumentPropertyReferenceTarget::PermanentDocument {
                        contract_id,
                        document_type_name,
                        property_agreement,
                        where_conditions,
                    },
                    _ => DocumentPropertyReferenceTarget::DeletableDocument {
                        contract_id,
                        document_type_name,
                        property_agreement,
                        where_conditions,
                    },
                });
            };
            let lookup = parse_find_by(refers_to_map, find_by)?;
            match document_target {
                "permanentDocument" => {
                    // A permanent document is never deleted, so there is
                    // nothing to consume
                    if lookup.consume {
                        return Err(DataContractError::InvalidContractStructure(
                            "permanentDocument refersTo cannot consume the document findBy \
                             finds, which is never deleted: consume needs a deletableDocument \
                             reference"
                                .to_string(),
                        ));
                    }
                    // `chooseFirst` reads commitments that expire (`ttl`), which a permanent
                    // document never does
                    if lookup.choose_first.is_some() {
                        return Err(DataContractError::InvalidContractStructure(
                            "permanentDocument refersTo does not take chooseFirst: the \
                             commitments it reads must expire, and a permanent document never \
                             leaves state: chooseFirst needs a deletableDocument reference"
                                .to_string(),
                        ));
                    }
                    DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                        contract_id,
                        document_type_name,
                        property_agreement,
                        where_conditions,
                        lookup,
                    }
                }
                _ => {
                    // Consuming deletes the found document, so only its
                    // owner's own reveal may: the `$ownerId` entry makes the
                    // writer the found document's owner
                    if lookup.consume
                        && property_agreement.get(OWNER_ID).map(String::as_str) != Some(OWNER_ID)
                    {
                        return Err(DataContractError::InvalidContractStructure(
                            "deletableDocument refersTo may consume the document findBy finds \
                             only when the writer owns it: declare the where entry \
                             \"$ownerId\": \"$ownerId\""
                                .to_string(),
                        ));
                    }
                    DocumentPropertyReferenceTarget::DeletableDocumentLookup {
                        contract_id,
                        document_type_name,
                        property_agreement,
                        where_conditions,
                        lookup,
                    }
                }
            }
        }
        "identityPublicKey" => {
            let key_id_property = refers_to_map
                .get_str(property_names::KEY_ID_PROPERTY)
                .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;

            if key_id_property.is_empty() || key_id_property.len() > MAX_PROPERTY_PATH_LENGTH {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "identityPublicKey refersTo keyIdProperty must be between 1 and \
                     {MAX_PROPERTY_PATH_LENGTH} characters"
                )));
            }

            DocumentPropertyReferenceTarget::IdentityPublicKey {
                key_id_property: key_id_property.to_string(),
                key_requirements: parse_identity_key_reference_requirements(refers_to_map)?,
            }
        }
        other => {
            return Err(DataContractError::InvalidContractStructure(format!(
                "invalid refersTo type {other}"
            )))
        }
    };

    Ok(target)
}

/// A parsed `where`: the equalities, `{referring property: referenced
/// property}`, and the other entries, `{referenced property: condition}`.
type ParsedWhere = (BTreeMap<String, String>, BTreeMap<String, WhereCondition>);

/// The `where` of a document reference of `document_target`: what the
/// referenced document must hold once found, each entry keyed by a property of
/// the referenced document type (a property path, or `$ownerId`, `$creatorId`
/// or `$id`). An entry valued with a referring-side value it must equal (a
/// property path of the declaring type, or `$ownerId`, the writer) is an
/// equality, returned the way the parsed model holds it, `{referring property:
/// referenced property}`, so a referring value may be compared once. Every
/// other entry is an object with one key, a [`WhereCondition`]: `const`, a
/// literal it must equal; `notEqual`, a referring property, `"$ownerId"` or a
/// `const` it must differ from; `in`, two or more distinct literals of one
/// kind it must be one of; an ordering (`greaterThan`, `greaterThanOrEqual`,
/// `lessThan`, `lessThanOrEqual`) with a referring property, an integer
/// `const` or a time or height of the referring document as its bound, never
/// of an identifier; or, of a typed array, `contains`, a referring property or
/// `"$ownerId"` it must hold, or `containsAll`, a referring typed array whose
/// every element it must hold. `where` never finds the document and reads
/// neither `"."` nor a function, which belong to `findBy`. What the names
/// resolve to, whether a literal fits the property it is compared with, and
/// whether an equality or a condition compares lists, is checked at
/// registration.
fn parse_where(
    refers_to_map: &BTreeMap<String, &Value>,
    document_target: &str,
) -> Result<ParsedWhere, DataContractError> {
    let Some(where_value) = refers_to_map.get(property_names::WHERE) else {
        return Ok((BTreeMap::new(), BTreeMap::new()));
    };
    let where_map = where_value.to_btree_ref_string_map()?;
    if where_map.is_empty() || where_map.len() > 10 {
        return Err(DataContractError::InvalidContractStructure(format!(
            "{document_target} refersTo where must compare between 1 and 10 properties"
        )));
    }
    let mut pairs = BTreeMap::new();
    let mut conditions = BTreeMap::new();
    for (referenced_property, value) in where_map {
        if referenced_property.is_empty() || referenced_property.len() > MAX_PROPERTY_PATH_LENGTH {
            return Err(DataContractError::InvalidContractStructure(format!(
                "refersTo where property paths must be between 1 and {MAX_PROPERTY_PATH_LENGTH} \
                 characters"
            )));
        }
        // Either side may name a system property, but only one the
        // comparison can read back as an identifier: `$ownerId`, `$creatorId`
        // or `$id` on the referenced side; the writer's `$ownerId` on the
        // referring side (a write gate)
        if referenced_property.starts_with('$')
            && !is_referenced_system_agreement_property(&referenced_property)
        {
            return Err(DataContractError::InvalidContractStructure(
                "refersTo where keys must name a property of the referenced document type or \
                 one of its $ownerId, $creatorId and $id system properties"
                    .to_string(),
            ));
        }
        if let Value::Map(_) = value {
            let condition = parse_where_condition(&referenced_property, value)?;
            conditions.insert(referenced_property, condition);
            continue;
        }
        let referring_property = value.as_text().ok_or_else(|| {
            DataContractError::InvalidContractStructure(
                "refersTo where values must name a property of the referring document type or \
                 \"$ownerId\", the writer, or be an object holding const, notEqual, in, an \
                 ordering, contains or containsAll"
                    .to_string(),
            )
        })?;
        let referring_property =
            parse_where_referring_property(&referenced_property, referring_property)?;
        if pairs
            .insert(referring_property.to_string(), referenced_property.clone())
            .is_some()
        {
            return Err(DataContractError::InvalidContractStructure(format!(
                "refersTo where compares \"{referring_property}\" twice: each referring value may \
                 be compared once"
            )));
        }
    }
    Ok((pairs, conditions))
}

/// The referring-side value a `where` entry keyed by `referenced_property`
/// names, `referring_property`: a property path of the declaring type, or
/// `$ownerId`, the writer. Neither `"."`, which only `findBy` reads, nor any
/// other system property.
fn parse_where_referring_property<'a>(
    referenced_property: &str,
    referring_property: &'a str,
) -> Result<&'a str, DataContractError> {
    if referring_property.is_empty() || referring_property.len() > MAX_PROPERTY_PATH_LENGTH {
        return Err(DataContractError::InvalidContractStructure(format!(
            "refersTo where property paths must be between 1 and {MAX_PROPERTY_PATH_LENGTH} \
             characters"
        )));
    }
    if referring_property == LOOKUP_REFERENCE_VALUE {
        return Err(DataContractError::InvalidContractStructure(format!(
            "refersTo where \"{referenced_property}\" reads \".\", the reference's own \
             value, which only findBy reads"
        )));
    }
    if referring_property.starts_with('$')
        && !is_referring_system_agreement_property(referring_property)
    {
        return Err(DataContractError::InvalidContractStructure(
            "refersTo where values must name a property of the referring document type or \
             its $ownerId"
                .to_string(),
        ));
    }
    Ok(referring_property)
}

/// The [`WhereCondition`] a `where` entry keyed by `referenced_property`
/// declares with `value`, an object with one key: `const`, `notEqual`, `in`,
/// an ordering, `contains` or `containsAll`. Any other key is refused: a
/// function computes a key that finds the document, which belongs to `findBy`.
/// `contains` and `containsAll` read a list, which `$ownerId`, `$creatorId`
/// and `$id` are not.
fn parse_where_condition(
    referenced_property: &str,
    value: &Value,
) -> Result<WhereCondition, DataContractError> {
    let invalid = |message: String| {
        DataContractError::InvalidContractStructure(format!(
            "refersTo where \"{referenced_property}\" {message}"
        ))
    };
    let Value::Map(entries) = value else {
        return Err(invalid("must be an object".to_string()));
    };
    if entries
        .iter()
        .any(|(key, _)| key.as_text() == Some(property_names::FUNCTION))
    {
        return Err(invalid(
            "holds a function, which computes a key that finds the document: declare it in \
             findBy"
                .to_string(),
        ));
    }
    let [(Value::Text(key), body)] = entries.as_slice() else {
        return Err(invalid(
            "must be an object with one key: const, notEqual, in, greaterThan, \
             greaterThanOrEqual, lessThan, lessThanOrEqual, contains or containsAll"
                .to_string(),
        ));
    };
    let literal = |value: &Value, at: &str| {
        WhereLiteral::from_value(value).ok_or_else(|| {
            invalid(format!(
                "{at} must be an integer, true, false or a string (a base58 identifier for an \
                 identifier, hex for a byte array)"
            ))
        })
    };
    // `{ "const": literal }`, the one object a bound may be
    let constant = |value: &Value, at: &str| match value {
        Value::Map(entries) => match entries.as_slice() {
            [(Value::Text(key), literal_value)] if key == WHERE_CONST => {
                literal(literal_value, &format!("{at}.const"))
            }
            _ => Err(invalid(format!(
                "{at} must be a property path, or {{ \"const\": ... }}"
            ))),
        },
        _ => Err(invalid(format!(
            "{at} must be a property path, or {{ \"const\": ... }}"
        ))),
    };
    let identifier_side = referenced_property.starts_with('$');
    match key.as_str() {
        WHERE_CONST => Ok(WhereCondition::Constant(literal(body, WHERE_CONST)?)),
        WHERE_NOT_EQUAL => {
            let bound = match body {
                Value::Text(path) if path == OWNER_ID => WhereNotEqualBound::Writer,
                Value::Text(path) => WhereNotEqualBound::Property(
                    parse_where_referring_property(referenced_property, path)?.to_string(),
                ),
                body => WhereNotEqualBound::Constant(constant(body, WHERE_NOT_EQUAL)?),
            };
            Ok(WhereCondition::NotEqual(bound))
        }
        WHERE_IN => {
            let Some(values) = body.as_array().filter(|values| values.len() >= 2) else {
                return Err(invalid("in must list two or more values".to_string()));
            };
            let mut literals = Vec::with_capacity(values.len());
            let mut first_kind = None;
            for (index, value) in values.iter().enumerate() {
                let parsed = literal(value, &format!("in[{index}]"))?;
                let kind = std::mem::discriminant(&parsed);
                if *first_kind.get_or_insert(kind) != kind {
                    return Err(invalid(format!(
                        "in[{index}] is of another kind than in[0]: the values an in lists are \
                         all integers, all booleans or all strings"
                    )));
                }
                if literals.contains(&parsed) {
                    return Err(invalid(format!("in[{index}] repeats an earlier value")));
                }
                literals.push(parsed);
            }
            Ok(WhereCondition::In(literals))
        }
        list_key @ (WHERE_CONTAINS | WHERE_CONTAINS_ALL) => {
            if identifier_side {
                return Err(invalid(format!(
                    "reads {list_key} of an identifier, which is one value: {list_key} reads a \
                     typed array"
                )));
            }
            let Value::Text(path) = body else {
                return Err(invalid(format!(
                    "{list_key} must be a property path of the referring document type{}",
                    if list_key == WHERE_CONTAINS {
                        ", or \"$ownerId\""
                    } else {
                        ""
                    }
                )));
            };
            if list_key == WHERE_CONTAINS {
                let bound = if path == OWNER_ID {
                    WhereContainsBound::Writer
                } else {
                    WhereContainsBound::Property(
                        parse_where_referring_property(referenced_property, path)?.to_string(),
                    )
                };
                return Ok(WhereCondition::Contains(bound));
            }
            // The writer is one identifier, not a list of them
            if path == OWNER_ID {
                return Err(invalid(
                    "containsAll reads $ownerId, the writer, which is one identifier: \
                     containsAll reads a typed array of the referring document type, and \
                     contains the writer"
                        .to_string(),
                ));
            }
            Ok(WhereCondition::ContainsAll(
                parse_where_referring_property(referenced_property, path)?.to_string(),
            ))
        }
        ordering_key => {
            let Some(ordering) = WhereOrdering::ALL
                .into_iter()
                .find(|ordering| ordering.wire_name() == ordering_key)
            else {
                return Err(invalid(format!(
                    "names \"{ordering_key}\", which is not const, notEqual, in, greaterThan, \
                     greaterThanOrEqual, lessThan, lessThanOrEqual, contains or containsAll: a \
                     function computes a key that finds the document, which belongs in findBy"
                )));
            };
            if identifier_side {
                return Err(invalid(format!(
                    "orders an identifier, which {ordering_key} does not: an ordering compares \
                     integers"
                )));
            }
            let bound = match body {
                Value::Text(path) if path == OWNER_ID => {
                    return Err(invalid(format!(
                        "orders by $ownerId, an identifier, which {ordering_key} does not: an \
                         ordering compares integers"
                    )));
                }
                Value::Text(path) => match SystemProperty::from_name(path) {
                    Some(property) => WhereOrderingBound::System(property),
                    None => WhereOrderingBound::Property(
                        parse_where_referring_property(referenced_property, path)?.to_string(),
                    ),
                },
                body => match constant(body, ordering_key)? {
                    WhereLiteral::Integer(integer) => WhereOrderingBound::Constant(integer),
                    _ => {
                        return Err(invalid(format!(
                            "{ordering_key}.const must be an integer: an ordering compares \
                             integers"
                        )))
                    }
                },
            };
            Ok(WhereCondition::Ordering { ordering, bound })
        }
    }
}

/// A reference to a list the value must be an element of, declared as a
/// `permanentDocument` with `inList`, beyond what it shares with the other
/// document references: `findBy` must be exactly `{ "$id": <property> }`,
/// naming the property of the declaring type (a schema property: no document
/// has the writer's id) that holds the id of the document the list is read
/// from, and `inList` names the typed array of identifiers of the referenced
/// type the value must be an element of. The `$id` entry joins the `where`
/// comparisons in the parsed model's `property_agreement`, as `{property:
/// "$id"}`. What the names resolve to is checked under full validation, once
/// the document types are parsed: the `$id` property against the declaring
/// type ([`validate_list_element_sources`]), the list against the referenced
/// one (at contract level for a type of the same contract, at registration
/// for one of another contract), the `where` comparisons as every agreement's.
/// `constraints` and `consume` are refused on every parse: they describe a
/// document `findBy` finds by a key, which a list element reference has no
/// field for, so they would be dropped unenforced.
fn parse_list_element_reference(
    refers_to_map: &BTreeMap<String, &Value>,
    find_by_value: &Value,
    contract_id: Option<Identifier>,
    document_type_name: String,
    mut property_agreement: BTreeMap<String, String>,
    where_conditions: BTreeMap<String, WhereCondition>,
) -> Result<ListElementReference, DataContractError> {
    if refers_to_map.contains_key(property_names::CONSTRAINTS)
        || refers_to_map.contains_key(property_names::CONSUME)
    {
        return Err(DataContractError::InvalidContractStructure(
            "permanentDocument refersTo inList does not take constraints or consume: they \
             describe a document findBy finds by a key, and inList checks that a list holds the \
             value"
                .to_string(),
        ));
    }
    let find_by_map = find_by_value.to_btree_ref_string_map()?;
    let document_id_property = match find_by_map.get(ID) {
        Some(value) if find_by_map.len() == 1 => value.as_text(),
        _ => None,
    }
    .ok_or_else(|| {
        DataContractError::InvalidContractStructure(
            "permanentDocument refersTo inList finds the document holding the list by its $id: \
             findBy must be exactly { \"$id\": <the property holding the document's id> }"
                .to_string(),
        )
    })?;
    if document_id_property.starts_with('$')
        || document_id_property == LOOKUP_REFERENCE_VALUE
        || document_id_property.is_empty()
        || document_id_property.len() > MAX_PROPERTY_PATH_LENGTH
    {
        return Err(DataContractError::InvalidContractStructure(
            "permanentDocument refersTo inList findBy $id must read a property of the referring \
             document type: no document has the writer's id, and the value itself is the list \
             element"
                .to_string(),
        ));
    }
    if property_agreement
        .values()
        .any(|referenced| referenced.as_str() == ID)
        || where_conditions.contains_key(ID)
    {
        return Err(DataContractError::InvalidContractStructure(
            "permanentDocument refersTo inList compares $id in findBy, so where may not compare \
             it again"
                .to_string(),
        ));
    }
    if property_agreement
        .insert(document_id_property.to_string(), ID.to_string())
        .is_some()
    {
        return Err(DataContractError::InvalidContractStructure(format!(
            "permanentDocument refersTo inList reads \"{document_id_property}\" in findBy and \
             in where: each referring value may be compared once"
        )));
    }

    let in_list = refers_to_map
        .get_str(property_names::IN_LIST)
        .map_err(|e| DataContractError::ValueWrongType(e.to_string()))?;
    if in_list.is_empty() || in_list.len() > MAX_PROPERTY_PATH_LENGTH || in_list.starts_with('$') {
        return Err(DataContractError::InvalidContractStructure(format!(
            "permanentDocument refersTo inList must be a property path of 1 to \
             {MAX_PROPERTY_PATH_LENGTH} characters"
        )));
    }

    Ok(ListElementReference {
        contract_id,
        document_type_name,
        property_agreement,
        where_conditions,
        in_list: in_list.to_string(),
    })
}

/// Checks the referring side of every `refersTo` with `inList` of a document
/// type, alone or as a leaf of a reference expression, once all its
/// properties are parsed: the `findBy` `$id` must read a stored identifier
/// property of the type. See [`ListElementReference::referring_side_error`].
///
/// Full validation only (registration), like the meta-schema, but in every
/// build: a contract read back from state passed it when it was written, and
/// the write-time check refuses a value whose `$id` property finds no
/// document rather than relying on it. Generation 3 is the only parser
/// admitting `refersTo` at all.
pub(super) fn validate_list_element_sources(
    document_type: DocumentTypeRef,
    document_type_name: &str,
) -> Result<(), DataContractError> {
    // On the writer or the creator, on an identifier property or on the
    // elements of a typed array
    for (holder, reference) in document_type.reference_declarations() {
        let Some(target) = reference.target() else {
            continue;
        };
        // Each leaf of a reference expression is checked as it would be
        // alone, and the error names the leaf (`refersTo anyOf[1] inList`)
        for (leaf_path, leaf) in target.leaves_with_paths() {
            let Some(list_reference) = leaf.as_list_element_reference() else {
                continue;
            };
            if let Some(reason) = list_reference.referring_side_error(document_type) {
                let at = if leaf_path.is_empty() {
                    String::new()
                } else {
                    format!(" {leaf_path}")
                };
                return Err(DataContractError::InvalidContractStructure(format!(
                    "document type \"{document_type_name}\" {}{at} inList: {reason}",
                    holder.describe()
                )));
            }
        }
    }
    Ok(())
}

/// An `identityPublicKey` declaration on the KEY ID property: `identityProperty` names
/// whose key the value is (`$ownerId`, `$creatorId` or an identifier property of the
/// same document type), so the declaration takes no `keyIdProperty` and sits on an
/// integer property declaring exactly the range of a key id (`minimum` 0, `maximum`
/// 4294967295), the bounds the meta-schema pins, read from the schema rather than the
/// inferred type so that the rule does not depend on the contract's sized integer
/// types setting. Whether a named property or `$creatorId` fits the document type is
/// checked at contract registration.
fn apply_key_id_reference_v0(
    inner_properties: &BTreeMap<String, &Value>,
    refers_to_map: &BTreeMap<String, &Value>,
    reference_type: &str,
    identity_property_value: &Value,
    property_type: DocumentPropertyType,
) -> Result<DocumentPropertyType, DataContractError> {
    if reference_type != "identityPublicKey" {
        return Err(DataContractError::InvalidContractStructure(format!(
            "{reference_type} refersTo does not take identityProperty"
        )));
    }
    if refers_to_map.contains_key(property_names::KEY_ID_PROPERTY) {
        return Err(DataContractError::InvalidContractStructure(
            "identityPublicKey refersTo takes either keyIdProperty, on the identity \
             property, or identityProperty, on the key id property, not both"
                .to_string(),
        ));
    }
    let identity_property_name = identity_property_value.as_text().ok_or_else(|| {
        DataContractError::InvalidContractStructure(
            "identityPublicKey refersTo identityProperty must be a string".to_string(),
        )
    })?;
    let identity_property = KeyReferenceIdentityProperty::from_wire_name(identity_property_name)
        .ok_or_else(|| {
            DataContractError::InvalidContractStructure(format!(
                "identityPublicKey refersTo identityProperty {identity_property_name:?} is \
                 invalid, expected one of {:?} or a property path of 1 to 256 characters",
                KeyReferenceIdentityProperty::SYSTEM_WIRE_NAMES
            ))
        })?;
    let minimum = inner_properties.get_optional_integer::<i64>(property_names::MINIMUM)?;
    let maximum = inner_properties.get_optional_integer::<i64>(property_names::MAXIMUM)?;
    let is_key_id_range = minimum == Some(0) && maximum == Some(i64::from(u32::MAX));
    if !property_type.is_integer() || !is_key_id_range {
        return Err(DataContractError::InvalidContractStructure(
            "identityPublicKey refersTo with identityProperty is only allowed on a key id \
             property: an integer with minimum 0 and maximum 4294967295"
                .to_string(),
        ));
    }
    Ok(DocumentPropertyType::KeyIdWithReference(KeyIdReference {
        identity_property,
        key_requirements: parse_identity_key_reference_requirements(refers_to_map)?,
    }))
}

/// Whether a `refersTo` declaration names the `identityPublicKey` target.
fn is_identity_public_key_reference(refers_to: &Value) -> Result<bool, DataContractError> {
    Ok(refers_to
        .to_btree_ref_string_map()?
        .get(property_names::TYPE)
        .and_then(|reference_type| reference_type.as_text())
        == Some("identityPublicKey"))
}

/// Folds a `refersTo` declared on the `items` of a typed array into the
/// element type, as [`apply_property_reference`] folds one into a scalar
/// identifier, which the version 0 rules call for the declaration itself:
/// only an identifier element may carry one, and never an
/// `identityPublicKey` one, in either form, since that pairs one key id with
/// the reference (a sibling `keyIdProperty`, or the key id itself through
/// `identityProperty`), which cannot pair with many elements.
///
/// Versioned on `apply_property_reference`, the gate of the declarations it
/// reads: `None` ignores the keyword on an element exactly as it does on a
/// property.
pub(in crate::data_contract::document_type::class_methods) fn apply_element_reference(
    items: &BTreeMap<String, &Value>,
    element_type: DocumentPropertyType,
    platform_version: &PlatformVersion,
) -> Result<DocumentPropertyType, DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_property_reference
    {
        None => Ok(element_type),
        Some(0) => apply_element_reference_v0(items, element_type),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "apply_element_reference version {version} is not supported"
        ))),
    }
}

fn apply_element_reference_v0(
    items: &BTreeMap<String, &Value>,
    element_type: DocumentPropertyType,
) -> Result<DocumentPropertyType, DataContractError> {
    let Some(refers_to) = items.get(property_names::REFERS_TO) else {
        return Ok(element_type);
    };
    if !matches!(element_type, DocumentPropertyType::Identifier) {
        return Err(DataContractError::InvalidContractStructure(
            "refersTo is only allowed on identifier elements of a typed array".to_string(),
        ));
    }
    if is_identity_public_key_reference(refers_to)? {
        return Err(DataContractError::InvalidContractStructure(
            "identityPublicKey refersTo is not allowed on the elements of a typed array: it pairs \
             one key id with the reference, which cannot pair with many elements"
                .to_string(),
        ));
    }
    apply_property_reference_v0(items, element_type)
}

/// Reads one of a document type's own `refersTo` keywords, `keyword`: one
/// declaration whose value is an identity of the document rather than a
/// property's value, `ownerRefersTo` the document's `$ownerId` (the writer)
/// and `creatorRefersTo` its `$creatorId` (the creator), named `value` in the
/// errors. The declaration goes through [`apply_property_reference`] as one
/// declared on an identifier property does, `findBy` and `where` included
/// (where `"."` is that identity), but only these targets can hold an
/// identity: `identity`, a `permanentDocument` found by `findBy`, one with
/// `inList` (the identity an element of the list), and a `deletableDocument`
/// found by `findBy`, on the writer, or on the creator when the key is
/// computed (a commitment the create reveals), alone or as the leaves of an
/// `anyOf` / `allOf` expression.
/// The others are refused: `contract`, `token` and a document by id, since an
/// identity id is never a contract, token or document id, so a type declaring
/// one could never be written, and `identityPublicKey`, which pairs the value
/// with a key id the document does not carry.
///
/// Only parser generation 3 calls it, once the core parse has run the
/// meta-schema, on every parse, validating or not, like its other
/// doctype-level keywords. Versioned through `apply_property_reference`, the
/// gate of the declaration it reads: `None` leaves the keyword unread, as it
/// leaves `refersTo` on a property, refusals included.
pub(super) fn parse_doctype_reference(
    schema: &Value,
    keyword: &str,
    value: &str,
    documents_mutable: bool,
    platform_version: &PlatformVersion,
) -> Result<Option<DocumentPropertyReferenceTarget>, DataContractError> {
    if platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_property_reference
        .is_none()
    {
        return Ok(None);
    }
    // A schema that is not an object carries no keyword: the core parser
    // refuses it, and a value error here must not replace that refusal
    let Ok(schema_map) = schema.to_map() else {
        return Ok(None);
    };
    let Some(declaration) = schema_map.get_optional_key(keyword) else {
        return Ok(None);
    };

    let reference_type = declaration
        .to_btree_ref_string_map()?
        .get(property_names::TYPE)
        .and_then(|reference_type| reference_type.as_text())
        .map(str::to_string);
    let refusal = match reference_type.as_deref() {
        Some("contract") => Some(format!(
            "{keyword} does not take a contract reference: its value is {value}, an identity, \
             which is never a contract"
        )),
        Some("identityPublicKey") => Some(format!(
            "{keyword} does not take an identityPublicKey reference: it pairs the value with a \
             key id, which {value} does not carry"
        )),
        Some("token") => Some(format!(
            "{keyword} does not take a token reference: its value, {value}'s identity id, is \
             never a token id"
        )),
        _ => None,
    };
    if let Some(refusal) = refusal {
        return Err(DataContractError::InvalidContractStructure(refusal));
    }

    let inner_properties = BTreeMap::from([(property_names::REFERS_TO.to_string(), declaration)]);
    let target = match apply_property_reference(
        &inner_properties,
        DocumentPropertyType::Identifier,
        platform_version,
    )? {
        DocumentPropertyType::IdentifierWithReference(target) => target,
        // The declaration is not read where the keyword is not active
        DocumentPropertyType::Identifier => return Ok(None),
        _ => {
            return Err(DataContractError::InvalidContractStructure(format!(
                "{keyword} must declare what {value} refers to"
            )))
        }
    };
    // A single target, or every leaf of an `anyOf` / `allOf` expression, must
    // be one the identity can be: a leaf by id could never hold either
    for (leaf_path, leaf) in target.leaves_with_paths() {
        let at = if leaf_path.is_empty() {
            String::new()
        } else {
            format!(" {leaf_path}")
        };
        match leaf {
            // An identity id can be an element of a list of identities: the
            // charters' "the writer is a seated member"
            DocumentPropertyReferenceTarget::Identity
            | DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. }
            | DocumentPropertyReferenceTarget::ListElement(_) => {}
            // A deletable document found by `findBy` gates the writer on
            // it existing now, and every replace asks again: the writer never
            // changes (ownerRefersTo is only on a type whose documents stay
            // with it), so the gate is the writer's own. The creator's would
            // outlive a transfer, leaving a new owner unable to replace the
            // document once the creator's document is gone, so creatorRefersTo
            // keeps to targets that hold for good
            DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. }
                if keyword == property_names::OWNER_REFERS_TO => {}
            // A lookup judged when the document is created only (a commitment
            // the creator revealed, or on a type whose documents are never
            // replaced), which no replace asks for again, so the creator's gate
            // cannot outlive a transfer
            DocumentPropertyReferenceTarget::DeletableDocumentLookup { lookup, .. }
                if lookup.is_checked_on_create_only(documents_mutable) => {}
            DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. } => {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "{keyword}{at} does not take a deletableDocument reference unless it is \
                     judged on the create alone (a findBy function, or a document type whose \
                     documents are never replaced): the creator never changes, and a document a \
                     transfer handed on could not be replaced once the one findBy found is \
                     deleted"
                )))
            }
            DocumentPropertyReferenceTarget::PermanentDocument { .. }
            | DocumentPropertyReferenceTarget::DeletableDocument { .. }
            | DocumentPropertyReferenceTarget::ModeratedDocument { .. } => {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "{keyword}{at} takes a document reference only with findBy: its value, \
                     {value}'s identity id, is never a document id"
                )))
            }
            _ => {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "{keyword}{at} takes an identity reference, or a document reference with \
                     findBy"
                )))
            }
        }
    }
    Ok(Some(target))
}

/// The `findBy` of a document reference, `find_by_value`: properties of the
/// referenced document type mapped to where each value comes from on the
/// referring side (`"."`, `"$ownerId"`, a property path, or a function that
/// computes it, at most one), the key of the unique index whose properties are
/// exactly those it names, which is resolved against the referenced type (see
/// [`DocumentReferenceLookup::resolve_index`]). `"."`, the reference's own
/// value, fills the key exactly once; beside a function it may be left out,
/// the function reading the value in its params or the reference being an
/// `ownerRefersTo` or `creatorRefersTo` one (checked with the other
/// referring-side rules). `$id` finds a document by id only for a list
/// ([`parse_list_element_reference`]): without `findBy` the value is the id.
/// Beside a function the `refersTo`, `refers_to_map`, may declare `consume` and
/// `constraints` (`minimumAgeBlocks`, `chooseFirst`), what the commitment the key
/// finds must be, whether the create deletes it and which commitments to the
/// same values it must precede, held on the parsed lookup. What the
/// names resolve to is checked once the document types are parsed: the
/// sources against the declaring type ([`validate_reference_lookup_sources`]),
/// the index against the referenced one (at contract level for a type of the
/// same contract, at registration for one of another contract).
fn parse_find_by(
    refers_to_map: &BTreeMap<String, &Value>,
    find_by_value: &Value,
) -> Result<DocumentReferenceLookup, DataContractError> {
    let find_by_map = find_by_value.to_btree_ref_string_map()?;
    if find_by_map.is_empty() || find_by_map.len() > MAX_LOOKUP_KEYS {
        return Err(DataContractError::InvalidContractStructure(format!(
            "refersTo findBy must name between 1 and {MAX_LOOKUP_KEYS} properties, those of a \
             unique index of the referenced document type"
        )));
    }
    if find_by_map.contains_key(ID) {
        return Err(DataContractError::InvalidContractStructure(
            "refersTo findBy $id names the document by id: leave findBy out when the value is \
             the document's $id, or declare inList to find the document holding a list"
                .to_string(),
        ));
    }

    let mut keys = BTreeMap::new();
    let mut computed_keys = 0;
    for (index_property, source_value) in find_by_map {
        if index_property.is_empty() || index_property.len() > MAX_LOOKUP_PATH_LENGTH {
            return Err(DataContractError::InvalidContractStructure(format!(
                "refersTo findBy property names must be between 1 and {MAX_LOOKUP_PATH_LENGTH} \
                 characters"
            )));
        }
        // A function computes the index property's value from the referring
        // document
        let source = if matches!(source_value, Value::Map(_)) {
            if index_property.starts_with('$') {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "refersTo findBy function \"{index_property}\" must fill a schema property \
                     of the referenced document type, which the function computes"
                )));
            }
            computed_keys += 1;
            if computed_keys > 1 {
                return Err(DataContractError::InvalidContractStructure(
                    "refersTo findBy holds at most one function".to_string(),
                ));
            }
            LookupKeySource::Hash(LookupHashKey::from_value(source_value)?)
        } else {
            let source = source_value.as_text().ok_or_else(|| {
                DataContractError::InvalidContractStructure(
                    "refersTo findBy maps each property to \".\", \"$ownerId\", a property path \
                     of the referring document type or a function"
                        .to_string(),
                )
            })?;
            LookupKeySource::from_wire_name(source)?
        };
        keys.insert(index_property, source);
    }

    let lookup = DocumentReferenceLookup::new(keys);

    // Without the reference's own value in the key, every document would
    // resolve to the same referenced document whatever the property holds: it
    // fills the key once, as a source or a param of the function. Beside a
    // function `"."` may be absent here: its params may name a property
    // carrying the reference by its path, and the writer or the creator may
    // leave the value out, which the referring-side rules check knowing where
    // the reference sits
    let reference_value_uses = lookup.reference_value_uses();
    let allowed_uses: &[usize] = if computed_keys == 1 { &[0, 1] } else { &[1] };
    if !allowed_uses.contains(&reference_value_uses) {
        return Err(DataContractError::InvalidContractStructure(format!(
            "refersTo findBy must read \".\", the reference's own value, exactly once, found \
             {reference_value_uses}"
        )));
    }

    // What the commitment a computed key finds must meet, its `constraints`,
    // and whether the create deletes it, declared beside `findBy`
    let constraints = match refers_to_map.get(property_names::CONSTRAINTS) {
        None => BTreeMap::new(),
        Some(value) => {
            let constraints = value.to_btree_ref_string_map().map_err(|_| {
                DataContractError::InvalidContractStructure(
                    "refersTo constraints is an object holding minimumAgeBlocks, chooseFirst or \
                     both"
                        .to_string(),
                )
            })?;
            if constraints.is_empty() {
                return Err(DataContractError::InvalidContractStructure(
                    "refersTo constraints holds minimumAgeBlocks, chooseFirst or both".to_string(),
                ));
            }
            if let Some(unknown) = constraints.keys().find(|key| {
                key.as_str() != property_names::MINIMUM_AGE_BLOCKS
                    && key.as_str() != property_names::CHOOSE_FIRST
            }) {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "refersTo constraints holds minimumAgeBlocks and chooseFirst only, not \
                     {unknown:?}"
                )));
            }
            constraints
        }
    };
    let minimum_age_blocks = constraints
        .get(property_names::MINIMUM_AGE_BLOCKS)
        .map(|value| {
            let blocks: u32 = value.to_integer().map_err(|_| {
                DataContractError::InvalidContractStructure(
                    "refersTo minimumAgeBlocks must be an integer from 1 to 4294967295".to_string(),
                )
            })?;
            if blocks == 0 {
                return Err(DataContractError::InvalidContractStructure(
                    "refersTo minimumAgeBlocks must be at least 1".to_string(),
                ));
            }
            Ok(blocks)
        })
        .transpose()?;
    let consume = match refers_to_map.get(property_names::CONSUME) {
        None => false,
        Some(value) if value.as_bool() == Some(true) => true,
        Some(_) => {
            return Err(DataContractError::InvalidContractStructure(
                "refersTo consume may only be declared true".to_string(),
            ))
        }
    };
    let choose_first = constraints
        .get(property_names::CHOOSE_FIRST)
        .map(|value| parse_choose_first(value))
        .transpose()?;
    Ok(DocumentReferenceLookup {
        minimum_age_blocks,
        consume,
        choose_first,
        ..lookup
    })
}

/// The `chooseFirst` in a document reference's `constraints`, `choose_first_value`:
/// `{ "among", "validWhen", "by" }`. `among` is a schema property of the referenced document
/// type, the one the `findBy` function fills; `validWhen` holds exactly one entry, a schema
/// property of the referenced type mapped to an owner-bound function of the shape of a
/// `findBy` function; `by` lists the properties the commitments are ordered by, each a name
/// (ascending) or `{ "<name>": "asc" | "desc" }`, at most [`MAX_CHOOSE_FIRST_ORDER`], each once,
/// `$id` only last and never `among` itself. What the functions may read and what they name
/// are checked once the document types are parsed, beside the `findBy` they complement
/// ([`DocumentReferenceLookup::referring_side_error`], on every parse), and against the
/// referenced type ([`DocumentReferenceLookup::referenced_side_error`]).
fn parse_choose_first(choose_first_value: &Value) -> Result<LookupChooseFirst, DataContractError> {
    let refused = |reason: String| {
        DataContractError::InvalidContractStructure(format!("refersTo chooseFirst {reason}"))
    };
    let schema_property = |value: Option<&&Value>| match value {
        Some(Value::Text(property))
            if !property.is_empty()
                && property.len() <= MAX_LOOKUP_PATH_LENGTH
                && !property.starts_with('$') =>
        {
            Some(property.clone())
        }
        _ => None,
    };
    let choose_first_map = choose_first_value.to_btree_ref_string_map()?;
    if let Some(unknown) = choose_first_map
        .keys()
        .find(|key| !matches!(key.as_str(), "among" | "validWhen" | "by"))
    {
        return Err(refused(format!(
            "holds \"among\", \"validWhen\" and \"by\" only, not {unknown:?}"
        )));
    }
    let Some(among) = schema_property(choose_first_map.get("among")) else {
        return Err(refused(format!(
            "names in \"among\" the schema property of the referenced document type, of 1 to \
             {MAX_LOOKUP_PATH_LENGTH} characters, that the findBy function fills"
        )));
    };
    let valid_when_map = match choose_first_map.get("validWhen") {
        Some(value @ Value::Map(_)) => value.to_btree_ref_string_map()?,
        _ => BTreeMap::new(),
    };
    let mut valid_when_entries = valid_when_map.into_iter();
    let (Some((valid_when_property, function @ Value::Map(_))), None) =
        (valid_when_entries.next(), valid_when_entries.next())
    else {
        return Err(refused(format!(
            "among \"{among}\" holds in \"validWhen\" exactly one entry, a schema property of the \
             referenced document type mapped to a function, {{ \"function\", \"params\" }}"
        )));
    };
    if schema_property(Some(&&Value::Text(valid_when_property.clone()))).is_none() {
        return Err(refused(format!(
            "among \"{among}\" names in \"validWhen\" a schema property of the referenced \
             document type of 1 to {MAX_LOOKUP_PATH_LENGTH} characters, not {valid_when_property:?}"
        )));
    }
    let Some(Value::Array(entries)) = choose_first_map.get("by") else {
        return Err(refused(format!(
            "among \"{among}\" lists in \"by\" the properties commitments are ordered by"
        )));
    };
    if entries.is_empty() || entries.len() > MAX_CHOOSE_FIRST_ORDER {
        return Err(refused(format!(
            "among \"{among}\" orders by 1 to {MAX_CHOOSE_FIRST_ORDER} properties"
        )));
    }
    let mut by: Vec<ChooseFirstOrder> = Vec::with_capacity(entries.len());
    for entry in entries {
        let order = match entry {
            Value::Text(name) => Some((name.clone(), true)),
            Value::Map(map) => match map.as_slice() {
                [(Value::Text(name), Value::Text(direction))] => match direction.as_str() {
                    "asc" => Some((name.clone(), true)),
                    "desc" => Some((name.clone(), false)),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        let Some((name, ascending)) =
            order.filter(|(name, _)| !name.is_empty() && name.len() <= MAX_LOOKUP_PATH_LENGTH)
        else {
            return Err(refused(format!(
                "among \"{among}\" orders by a property name, ascending, or by {{ \"<name>\": \
                 \"asc\" }} or {{ \"<name>\": \"desc\" }}"
            )));
        };
        if name == among || by.iter().any(|order| order.property == name) {
            return Err(refused(format!(
                "among \"{among}\" orders by \"{name}\" twice, or by the property it groups by"
            )));
        }
        if by.last().is_some_and(|order| order.property == ID) {
            return Err(refused(format!(
                "among \"{among}\" orders by {ID} last only: the entries of an index are kept by \
                 it after every one of its properties"
            )));
        }
        by.push(ChooseFirstOrder {
            property: name,
            ascending,
        });
    }
    // The index keeps the documents equal on its properties by id in the direction of its last
    // (ascending under an equal value alone), and the scan, cut at its candidate limit, reads
    // them that way: a last `$id` running the other way would leave out the ones before the
    // commitment found, read last
    if let Some((last, before)) = by.split_last() {
        let ids_ascending = before.last().is_none_or(|order| order.ascending);
        if last.property == ID && last.ascending != ids_ascending {
            let direction = |ascending: bool| if ascending { "asc" } else { "desc" };
            let after = match before.last() {
                Some(order) => format!(
                    "after \"{}\" {}",
                    order.property,
                    direction(order.ascending)
                ),
                None => "alone".to_string(),
            };
            return Err(refused(format!(
                "among \"{among}\" orders by {ID} {} {after}: an index keeps the documents equal \
                 on its properties by {ID} in the direction of its last, and the scan reads them \
                 that way, so a last {ID} runs in the direction of the property before it \
                 (asc alone)",
                direction(last.ascending)
            )));
        }
    }
    Ok(LookupChooseFirst {
        among,
        valid_when_property,
        valid_when: LookupHashKey::from_value_of(function, property_names::CHOOSE_FIRST)?,
        by,
    })
}

/// Checks the referring side of every `refersTo` `findBy` of a document type,
/// once all its properties are parsed: each property a key reads must exist,
/// be a stored, required, single value (with every object around it
/// required), and not be the reference property itself. See
/// [`DocumentReferenceLookup::referring_side_error`]. The `findBy` of the type's
/// `ownerRefersTo` or `creatorRefersTo` follows the same rules: the owner's
/// `"$ownerId"` sources pass the owner rule, since that type cannot change
/// owner (generation 3 refuses the keyword otherwise), and the creator's are
/// refused by it, since that type can.
///
/// Runs on every parse, validating or not, like the `encryptedFor` check: the
/// rule is a property of the document type, and the write-time lookup reads
/// the sources through the same flattened map. Generation 3 is the only
/// parser admitting `refersTo` at all.
pub(super) fn validate_reference_lookup_sources(
    document_type: DocumentTypeRef,
    document_type_name: &str,
) -> Result<(), DataContractError> {
    // On the writer or the creator, on an identifier property or on the
    // elements of a typed array: the key's other parts are the same for every
    // element, and the owner or creator reference's `"."` is that identity,
    // named by the path `$ownerId` or `$creatorId`, which no property source
    // can be
    // At most one `chooseFirst` on the type, so a create reads one set of competitors
    let mut choose_first_declared = false;
    for (holder, reference) in document_type.reference_declarations() {
        let Some(target) = reference.target() else {
            continue;
        };
        let on_elements = matches!(reference, PropertyReference::Elements { .. });
        // Each leaf of a reference expression reads its key as it would
        // alone, and the error names the leaf (`refersTo anyOf[1] findBy`)
        for (leaf_path, leaf) in target.leaves_with_paths() {
            let Some(declaration) = leaf.as_any_document_reference() else {
                continue;
            };
            let Some(lookup) = declaration.lookup else {
                continue;
            };
            let choose_first_error = lookup.choose_first.as_ref().and_then(|_| {
                let error = choose_first_declaration_error(
                    on_elements,
                    &leaf_path,
                    &declaration,
                    choose_first_declared,
                );
                choose_first_declared = true;
                error
            });
            // Where a `chooseFirst` sits is judged first: on the elements of a typed array its
            // findBy, which may read the element in no function, would be refused for that
            if let Some(reason) = choose_first_error
                .or_else(|| lookup.referring_side_error(document_type, holder.path()))
                .or_else(|| create_only_leaf_error(document_type, &leaf_path, lookup, &declaration))
                .or_else(|| age_or_consume_error(document_type, lookup))
            {
                let at = if leaf_path.is_empty() {
                    String::new()
                } else {
                    format!(" {leaf_path}")
                };
                return Err(DataContractError::InvalidContractStructure(format!(
                    "document type \"{document_type_name}\" {}{at} findBy: {reason}",
                    holder.describe()
                )));
            }
        }
    }
    Ok(())
}

/// Why a `chooseFirst` cannot sit where it is declared; `None` when it can. A create reads
/// the commitments a `chooseFirst` precedes once per value it judges, up to
/// `SystemLimits::max_choose_first_candidates` documents, so a document type declares at most
/// one (`already_declared` is whether an earlier declaration of the type held one), on a
/// single reference: not on the elements of a typed array (`on_elements`), each element of
/// which would read them again, nor on an operand of a reference expression (`leaf_path`
/// not empty). A `where` beside it compares nothing but the writer with the commitment's
/// owner (the declaration's `property_agreement`) and holds no other condition
/// (`where_conditions`): a competitor is judged on its key alone, so a commitment differing
/// from the create's in a `where` value, or failing a condition the found one meets, could
/// precede it.
fn choose_first_declaration_error(
    on_elements: bool,
    leaf_path: &str,
    declaration: &DocumentReferenceDeclaration,
    already_declared: bool,
) -> Option<String> {
    if already_declared {
        return Some(
            "a document type declares at most one chooseFirst: a create reads the competitors \
             of each"
                .to_string(),
        );
    }
    if on_elements {
        return Some(
            "chooseFirst may not sit on the elements of a typed array: every element would read \
             the competitors of its commitment again"
                .to_string(),
        );
    }
    if !leaf_path.is_empty() {
        return Some(
            "chooseFirst may not sit on an operand of a reference expression: it describes the \
             one commitment a reference reveals"
                .to_string(),
        );
    }
    if let Some(referenced) = declaration.where_conditions.keys().next() {
        return Some(format!(
            "where \"{referenced}\" holds a condition beside chooseFirst: a competitor is judged \
             on its key alone, so where may compare nothing but \"{OWNER_ID}\" with the writer"
        ));
    }
    declaration
        .property_agreement
        .iter()
        .find(|(referring, referenced)| {
            referring.as_str() != OWNER_ID || referenced.as_str() != OWNER_ID
        })
        .map(|(referring, referenced)| {
            format!(
                "where \"{referenced}\": \"{referring}\" beside chooseFirst: a competitor is \
                 judged on its key alone, so where may compare nothing but \"{OWNER_ID}\" with \
                 the writer"
            )
        })
}

/// Why a lookup declared on `document_type` may not declare `minimumAgeBlocks` or `consume`;
/// `None` when it declares neither or may. Both describe the document found when the
/// referring document is created, so the reference must be judged on the create alone
/// ([`DocumentReferenceLookup::is_checked_on_create_only`]): its key holds a `findBy` function,
/// or the type's documents are never replaced.
fn age_or_consume_error(
    document_type: DocumentTypeRef,
    lookup: &DocumentReferenceLookup,
) -> Option<String> {
    let declares = lookup.minimum_age_blocks.is_some() || lookup.consume;
    (declares && !lookup.is_checked_on_create_only(document_type.documents_mutable())).then(|| {
        "minimumAgeBlocks and consume describe the document found when the referring document \
         is created, so the reference must be judged on the create alone: give findBy a \
         function, or make the document type immutable (documentsMutable: false)"
            .to_string()
    })
}

/// Why a `refersTo` leaf whose key a `findBy` function computes, at
/// `leaf_path` of its declaration on `document_type`, cannot be
/// judged on the create alone; `None` when it can (or the lookup holds no
/// computed key). Such a leaf holds on a replace without a read, so:
///
/// * on a document type whose documents can be replaced it may not be an
///   operand of an `anyOf`, at any depth: the `anyOf` would hold on every
///   replace whichever operand held on the create, and the operands a replace
///   asks again would never be asked;
/// * every `where` entry beside the function reads a referring property that
///   is transient or fixed once written, since no replace checks the entry
///   again, the property an equality compares and the one a condition is
///   bounded by alike. `"$ownerId"` is exempt: it names the writer of the
///   create, whose own commitment the reveal is.
fn create_only_leaf_error(
    document_type: DocumentTypeRef,
    leaf_path: &str,
    lookup: &DocumentReferenceLookup,
    declaration: &DocumentReferenceDeclaration,
) -> Option<String> {
    if !lookup.is_checked_on_create_only(document_type.documents_mutable()) {
        return None;
    }
    if document_type.documents_mutable()
        && leaf_path.contains(ReferenceCombinator::AnyOf.wire_name())
    {
        return Some(
            "a reference whose key a findBy function computes is judged when the document is \
             created only and holds on a replace without a read, so on a document type whose \
             documents can be replaced it cannot be an operand of an anyOf: the anyOf would hold \
             on every replace, whichever operand held on the create"
                .to_string(),
        );
    }
    declaration
        .where_referring_properties()
        .filter(|referring| *referring != OWNER_ID)
        .find(|referring| {
            !is_transient(document_type, referring)
                && (!schema_property_is_fixed_once_written(document_type, referring)
                    || clearable_once_its_document_is_deleted(document_type, referring))
        })
        .map(|referring| {
            format!(
                "where reads \"{referring}\" beside a findBy function, so it is judged when the \
                 document is created only: \"{referring}\" must be fixed once written (make the \
                 type immutable or list the property under `immutable` without a condition, and \
                 make a `deletableDocument` reference required, since a replace can clear an \
                 optional one once its document is deleted) or transient"
            )
        })
        .or_else(|| {
            // A time or height a later write changes, which no replace reads
            // again here; the writer, `$ownerId`, is admitted as above
            declaration
                .where_conditions
                .values()
                .find_map(|condition| match condition.bound_read() {
                    Some(WhereBoundRead::System(property))
                        if WhereBoundRead::System(property).changes_without_a_write() =>
                    {
                        Some(property)
                    }
                    _ => None,
                })
                .map(|property| {
                    format!(
                        "where is bounded by \"{}\" beside a findBy function, so it is judged \
                         when the document is created only, while a later write changes \"{}\": \
                         bound it by a time or height of the create ($createdAt or one of its \
                         heights)",
                        property.name(),
                        property.name()
                    )
                })
        })
}

/// Reads a property's `encryptedFor` declaration: how the bytes of a byte
/// array property were encrypted. Non-byte-array properties, identifiers
/// among them, cannot carry it.
///
/// Versioned on `apply_encrypted_for` in the platform version's document type
/// schema versions. `None` selects the behavior of the versions that predate
/// the keyword: it is ignored entirely, so their parses stay byte-for-byte
/// identical to what they always produced.
fn apply_encrypted_for(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: &DocumentPropertyType,
    platform_version: &PlatformVersion,
) -> Result<Option<EncryptedFor>, DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_encrypted_for
    {
        None => Ok(None),
        Some(0) => apply_encrypted_for_v0(inner_properties, property_type),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "apply_encrypted_for version {version} is not supported"
        ))),
    }
}

fn apply_encrypted_for_v0(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: &DocumentPropertyType,
) -> Result<Option<EncryptedFor>, DataContractError> {
    let Some(encrypted_for_value) = inner_properties.get(property_names::ENCRYPTED_FOR) else {
        return Ok(None);
    };

    match property_type {
        DocumentPropertyType::ByteArray(_) => {}
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
            return Err(DataContractError::InvalidContractStructure(
                "encryptedFor is not allowed on identifier properties, only on byte arrays"
                    .to_string(),
            ));
        }
        _ => {
            return Err(DataContractError::InvalidContractStructure(
                "encryptedFor is only allowed on byte array properties".to_string(),
            ));
        }
    }

    let encrypted_for_map = encrypted_for_value.to_btree_ref_string_map()?;

    for key in encrypted_for_map.keys() {
        if !matches!(
            key.as_str(),
            property_names::RECIPIENT
                | property_names::RECIPIENT_KEY
                | property_names::SENDER_KEY
                | property_names::SCHEME
        ) {
            return Err(DataContractError::InvalidContractStructure(format!(
                "encryptedFor {key:?} is unknown, expected recipient, recipientKey, senderKey \
                 and scheme"
            )));
        }
    }

    let path = |key: &'static str| -> Result<&str, DataContractError> {
        let Some(value) = encrypted_for_map.get(key) else {
            return Err(DataContractError::InvalidContractStructure(format!(
                "encryptedFor must declare {key}"
            )));
        };
        let path = value.as_text().ok_or_else(|| {
            DataContractError::InvalidContractStructure(format!(
                "encryptedFor {key} must be a property path (a string)"
            ))
        })?;
        if path.is_empty() || path.len() > MAX_PROPERTY_PATH_LENGTH {
            return Err(DataContractError::InvalidContractStructure(format!(
                "encryptedFor {key} must be between 1 and {MAX_PROPERTY_PATH_LENGTH} characters"
            )));
        }
        Ok(path)
    };

    let recipient = EncryptedForRecipient::from_path(path(property_names::RECIPIENT)?);
    if recipient
        .property_path()
        .is_some_and(|recipient_path| recipient_path.starts_with('$'))
    {
        return Err(DataContractError::InvalidContractStructure(
            "encryptedFor recipient must name an identifier property of the document type or \
             its $ownerId"
                .to_string(),
        ));
    }

    let mut key_paths = [String::new(), String::new()];
    for (key, slot) in [property_names::RECIPIENT_KEY, property_names::SENDER_KEY]
        .into_iter()
        .zip(key_paths.iter_mut())
    {
        let key_path = path(key)?;
        if key_path.starts_with('$') {
            return Err(DataContractError::InvalidContractStructure(format!(
                "encryptedFor {key} must name an integer property of the document type, not a \
                 system property"
            )));
        }
        *slot = key_path.to_string();
    }
    let [recipient_key, sender_key] = key_paths;

    let scheme_value = encrypted_for_map
        .get(property_names::SCHEME)
        .ok_or_else(|| {
            DataContractError::InvalidContractStructure(
                "encryptedFor must declare scheme".to_string(),
            )
        })?;
    let scheme_name = scheme_value.as_text().ok_or_else(|| {
        DataContractError::InvalidContractStructure(
            "encryptedFor scheme must be a string".to_string(),
        )
    })?;
    let scheme = EncryptionScheme::from_wire_name(scheme_name).ok_or_else(|| {
        DataContractError::InvalidContractStructure(format!(
            "encryptedFor scheme {scheme_name:?} is unknown, expected one of {}",
            EncryptionScheme::ALL
                .iter()
                .map(|scheme| format!("{:?}", scheme.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    Ok(Some(EncryptedFor {
        recipient,
        recipient_key,
        sender_key,
        scheme,
    }))
}

/// Checks every `encryptedFor` declaration of a document type against the
/// properties it names, once all of them are parsed: the recipient must be an
/// identifier property (or `$ownerId`), the two key properties integers whose
/// schema declares `minimum` at least 0 and `maximum` at most 4294967295 (read
/// from the schema itself, so the rule holds whatever `sizedIntegerTypes` the
/// contract sets), none of them may be `transient` or sit inside a transient
/// object (a transient value is stripped before storage, which would leave
/// the stored ciphertext without its recipe), and the byte array's own
/// `maxItems` must hold the scheme's shortest ciphertext. Paths are looked up
/// among the flattened properties, so a nested property is named by its
/// dotted path. A key property's schema reached through a `$ref` is read from
/// `schema_defs`, the contract's `$defs`, as the core parse reads it.
///
/// Owned by parser generation 3: the only generation that admits the keyword.
pub(super) fn validate_encrypted_for_declarations(
    document_type: &DocumentTypeV2,
    schema_defs: Option<&BTreeMap<String, Value>>,
    document_type_name: &str,
) -> Result<(), DataContractError> {
    let flattened_properties = &document_type.flattened_properties;
    for (path, property) in flattened_properties {
        let Some(encrypted_for) = &property.encrypted_for else {
            continue;
        };
        let structure_error = |message: String| {
            DataContractError::InvalidContractStructure(format!(
                "document type \"{document_type_name}\" property \"{path}\" encryptedFor {message}"
            ))
        };

        let shortest = encrypted_for.scheme.minimum_ciphertext_length();
        if let DocumentPropertyType::ByteArray(sizes) = &property.property_type {
            if let Some(max_size) = sizes.max_size.filter(|max| usize::from(*max) < shortest) {
                return Err(structure_error(format!(
                    "maxItems {max_size} is below the {shortest} bytes the {} scheme produces \
                     at least, so no document could ever carry it",
                    encrypted_for.scheme
                )));
            }
        }

        if let Some(recipient_path) = encrypted_for.recipient.property_path() {
            match flattened_properties
                .get(recipient_path)
                .map(|recipient| &recipient.property_type)
            {
                Some(
                    DocumentPropertyType::Identifier
                    | DocumentPropertyType::IdentifierWithReference(_),
                ) => {}
                Some(other) => {
                    return Err(structure_error(format!(
                        "recipient \"{recipient_path}\" has type {}, not identifier",
                        other.name()
                    )));
                }
                None => {
                    return Err(structure_error(format!(
                        "recipient \"{recipient_path}\" is not a property of the document type"
                    )));
                }
            }
            if is_transient(DocumentTypeRef::V2(document_type), recipient_path) {
                return Err(structure_error(format!(
                    "recipient \"{recipient_path}\" is transient or inside a transient object: \
                     a transient value is never stored, so a reader could not tell whom the \
                     bytes are for"
                )));
            }
        }

        for (key, key_path) in [
            (property_names::RECIPIENT_KEY, &encrypted_for.recipient_key),
            (property_names::SENDER_KEY, &encrypted_for.sender_key),
        ] {
            if !flattened_properties.contains_key(key_path) {
                return Err(structure_error(format!(
                    "{key} \"{key_path}\" is not a property of the document type"
                )));
            }
            if !is_key_id_schema(&document_type.schema, schema_defs, key_path)? {
                return Err(structure_error(format!(
                    "{key} \"{key_path}\" must be an integer property with minimum at least 0 \
                     and maximum at most {}, so that it carries a key id",
                    u32::MAX
                )));
            }
            if is_transient(DocumentTypeRef::V2(document_type), key_path) {
                return Err(structure_error(format!(
                    "{key} \"{key_path}\" is transient or inside a transient object: a \
                     transient value is never stored, so a reader could not tell which key \
                     decrypts the bytes"
                )));
            }
        }
    }
    Ok(())
}

/// Reads a `generatedFrom` declaration off a string property: the function
/// the platform generates the property's value with, and its parameters,
/// other properties of the same document type. Only a string property
/// carries it, the one kind the functions return: a typed array has no
/// parameters for its elements, so the keyword is refused on the array and on
/// its items alike.
///
/// Versioned on `apply_generated_from` in the platform version's document
/// type schema versions. `None` selects the behavior of the versions that
/// predate the keyword: it is ignored entirely, so their parses stay
/// byte-for-byte identical to what they always produced.
///
/// The parameters are checked against the rest of the document type once
/// every property is parsed, by [`validate_generated_from_declarations`].
fn apply_generated_from(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: &DocumentPropertyType,
    platform_version: &PlatformVersion,
) -> Result<Option<GeneratedFrom>, DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .apply_generated_from
    {
        None => Ok(None),
        Some(0) => apply_generated_from_v0(inner_properties, property_type),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "apply_generated_from version {version} is not supported"
        ))),
    }
}

fn apply_generated_from_v0(
    inner_properties: &BTreeMap<String, &Value>,
    property_type: &DocumentPropertyType,
) -> Result<Option<GeneratedFrom>, DataContractError> {
    if let DocumentPropertyType::TypedArray(_) = property_type {
        // On the array itself first: the shared items lookup would refuse it there as
        // belonging on the items, and the keyword belongs on neither
        if inner_properties.contains_key(property_names::GENERATED_FROM)
            || typed_array_items_keyword(
                inner_properties,
                property_names::GENERATED_FROM,
                "has no parameters",
            )?
            .is_some()
        {
            return Err(DataContractError::InvalidContractStructure(
                "generatedFrom is only allowed on string properties, not on a typed array or \
                 its items"
                    .to_string(),
            ));
        }
        return Ok(None);
    }

    let Some(generated_from_value) = inner_properties.get(property_names::GENERATED_FROM) else {
        return Ok(None);
    };

    if !matches!(property_type, DocumentPropertyType::String(_)) {
        return Err(DataContractError::InvalidContractStructure(
            "generatedFrom is only allowed on string properties".to_string(),
        ));
    }

    let shape_error = || {
        DataContractError::InvalidContractStructure(
            "generatedFrom must be an object with a function (its name) and params (the paths \
             of the properties of the same document type it reads)"
                .to_string(),
        )
    };
    let generated_from_map = generated_from_value
        .to_btree_ref_string_map()
        .map_err(|_| shape_error())?;

    for key in generated_from_map.keys() {
        if !matches!(
            key.as_str(),
            property_names::FUNCTION | property_names::PARAMS
        ) {
            return Err(DataContractError::InvalidContractStructure(format!(
                "generatedFrom {key:?} is unknown, expected function and params"
            )));
        }
    }

    let function_name = generated_from_map
        .get(property_names::FUNCTION)
        .and_then(|value| value.as_text())
        .ok_or_else(shape_error)?;
    let function = SystemFunction::from_wire_name(function_name).ok_or_else(|| {
        DataContractError::InvalidContractStructure(format!(
            "generatedFrom function {function_name:?} is unknown, expected one of {}",
            SystemFunction::ALL
                .iter()
                .filter(|function| matches!(function, SystemFunction::StringTransformation(_)))
                .map(|function| format!("{:?}", function.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    // A property is generated with a string transformation; the other
    // namespaces (`sys.hash`) belong to other keywords
    let Some(parameter_count) = function
        .parameter_count()
        .filter(|_| matches!(function, SystemFunction::StringTransformation(_)))
    else {
        return Err(DataContractError::InvalidContractStructure(format!(
            "generatedFrom function {function} does not generate a string: a property is \
             generated with a sys.stringTransformations function"
        )));
    };

    let params = generated_from_map
        .get(property_names::PARAMS)
        .and_then(|value| value.as_array())
        .ok_or_else(shape_error)?;
    if params.len() != parameter_count {
        return Err(DataContractError::InvalidContractStructure(format!(
            "generatedFrom function {function} takes {parameter_count} parameter(s), but params \
             lists {}",
            params.len()
        )));
    }
    let params = params
        .iter()
        .map(|param| {
            let path = param.as_text().ok_or_else(|| {
                DataContractError::InvalidContractStructure(
                    "generatedFrom params must be property paths (strings)".to_string(),
                )
            })?;
            if path.is_empty() || path.len() > MAX_PROPERTY_PATH_LENGTH {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "generatedFrom params must be between 1 and {MAX_PROPERTY_PATH_LENGTH} \
                     characters"
                )));
            }
            if path.starts_with('$') {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "generatedFrom params must name properties of the document type, not \
                     system property \"{path}\""
                )));
            }
            Ok(GenerationParam::Property(path.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(GeneratedFrom { function, params }))
}

/// Checks every `generatedFrom` declaration of a document type against the
/// properties its parameters name, once all of them are parsed. Each
/// parameter:
///
/// * must be a string property of the type (the one kind the functions read)
///   other than the declaring one, a nested one named by its dotted path, as
///   the flattened map names it;
/// * may not be generated itself, so the platform generates every left-out
///   value from values the client sent, in any order;
/// * may not be transient or sit inside a transient object, nor may the
///   declaring property: a transient parameter is dropped before storage, so a
///   replace would have to supply it again or lose the generated value, and a
///   transient generated property is never stored at all;
/// * must sit inside every object that holds the declaring property (a
///   top-level property may take any parameter), so a document supplying the
///   parameters always holds the object the platform writes the value into.
///
/// Owned by parser generation 3: the only generation that admits the keyword.
pub(super) fn validate_generated_from_declarations(
    document_type: &DocumentTypeV2,
    document_type_name: &str,
) -> Result<(), DataContractError> {
    let flattened_properties = &document_type.flattened_properties;
    for (path, property) in flattened_properties {
        let Some(generated_from) = &property.generated_from else {
            continue;
        };
        let structure_error = |message: String| {
            DataContractError::InvalidContractStructure(format!(
                "document type \"{document_type_name}\" property \"{path}\" generatedFrom \
                 {message}"
            ))
        };

        if is_transient(DocumentTypeRef::V2(document_type), path) {
            return Err(structure_error(
                "is on a property that is transient or inside a transient object: a transient \
                 value is never stored, so the generated value would be lost"
                    .to_string(),
            ));
        }

        for param in generated_from.property_params() {
            if param == path {
                return Err(structure_error(
                    "reads the property itself: name other string properties of the document \
                     type"
                        .to_string(),
                ));
            }
            let Some(param_property) = flattened_properties.get(param) else {
                if names_an_object(flattened_properties, param) {
                    return Err(structure_error(format!(
                        "param \"{param}\" is an object, not a string property: name one of its \
                         string members"
                    )));
                }
                return Err(structure_error(format!(
                    "param \"{param}\" is not a property of the document type"
                )));
            };
            if !matches!(
                param_property.property_type,
                DocumentPropertyType::String(_)
            ) {
                return Err(structure_error(format!(
                    "param \"{param}\" has type {}, not string",
                    param_property.property_type.name()
                )));
            }
            if param_property.generated_from.is_some() {
                return Err(structure_error(format!(
                    "param \"{param}\" is generated itself: name the properties it is generated \
                     from instead"
                )));
            }
            if is_transient(DocumentTypeRef::V2(document_type), param) {
                return Err(structure_error(format!(
                    "param \"{param}\" is transient or inside a transient object: a transient \
                     value is never stored, so a replace could not keep the generated value \
                     without sending it again"
                )));
            }
            if let Some((target_object, _)) = path.rsplit_once('.') {
                let inside_target_object = param
                    .strip_prefix(target_object)
                    .is_some_and(|rest| rest.starts_with('.'));
                if !inside_target_object {
                    return Err(structure_error(format!(
                        "param \"{param}\" is outside \"{target_object}\": every param must sit \
                         inside every object that holds the generated property, so a document \
                         supplying the params always has the object the value is written into"
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Reads the `propertyConstraints` keyword onto the document type and checks
/// every property its rules read: by its value, an integer or boolean
/// property of the type (a nested one named by its dotted path, as the
/// flattened map names it); by its `length` or `byteLength`, a string
/// property; by its `count`, an array or byte array property; by its presence,
/// a property of any type, an object included; any way one that is neither
/// transient nor inside a transient object. A system time or height a rule
/// reads (`$createdAt`, ...) must be one the type records, listed in
/// `required`, and an indexOnly type reads none, nor `$ownerId`. A
/// transient value is never stored, so a stored document could not be held to
/// a rule reading one. The declaration's shape ([`parse_property_constraints`]) and these reads are
/// checked on every parse; under full validation, the limits too: at most
/// `SystemLimits::max_property_constraints` rules, each of at most
/// `max_property_constraint_nodes` nodes, and no `anyOf` or `allOf` listing
/// the same condition twice. The `enum` a constant or a default is checked
/// against is read from the property's schema, through a `$ref` into
/// `schema_defs`, the contract's `$defs`, as the core parse reads it.
///
/// Only parser generation 3 calls it, once the core parse has run the
/// meta-schema, so under full validation a malformed declaration is the
/// meta-schema's to report. Version 0 also reads `deleteConstraints`
/// ([`apply_delete_constraints_v0`]). Versioned on `parse_property_constraints`
/// in the platform version's document type schema versions: `None` selects the
/// behavior of the versions that predate the keywords, which ignore them
/// entirely.
pub(super) fn apply_property_constraints(
    document_type: &mut DocumentTypeV2,
    schema_defs: Option<&BTreeMap<String, Value>>,
    document_type_name: &str,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<(), DataContractError> {
    match platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .parse_property_constraints
    {
        None => Ok(()),
        Some(0) => apply_property_constraints_v0(
            document_type,
            schema_defs,
            document_type_name,
            full_validation,
            platform_version,
        ),
        Some(version) => Err(DataContractError::Unsupported(format!(
            "parse_property_constraints version {version} is not supported"
        ))),
    }
}

fn apply_property_constraints_v0(
    document_type: &mut DocumentTypeV2,
    schema_defs: Option<&BTreeMap<String, Value>>,
    document_type_name: &str,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<(), DataContractError> {
    let property_kind =
        |path: &str| property_equality_kind(&document_type.flattened_properties, path);
    let constraints =
        parse_property_constraints(&document_type.schema, document_type_name, &property_kind)?;
    let structure_error = |message: String| {
        DataContractError::InvalidContractStructure(format!(
            "document type \"{document_type_name}\" propertyConstraints {message}"
        ))
    };

    for (name, constraint) in &constraints {
        validate_property_constraint_reads(
            document_type,
            schema_defs,
            &format!("rule \"{name}\""),
            constraint,
            true,
            true,
            &structure_error,
        )?;
    }

    if full_validation {
        validate_rule_limits(&constraints, &structure_error, platform_version)?;
    }

    document_type.property_constraints = constraints;
    // The rules gating the owner's delete, in the same grammar and under the same version
    apply_delete_constraints_v0(
        document_type,
        schema_defs,
        document_type_name,
        full_validation,
        platform_version,
    )
}

/// Checks one keyword's rules against the limits of `SystemLimits`: at most
/// `max_property_constraints` rules, reading at most
/// `max_property_constraint_aggregates` distinct `countOf` and `sumOf` totals,
/// each rule of at most `max_property_constraint_nodes` nodes, and no `anyOf`
/// or `allOf` listing the same condition twice. `propertyConstraints` and
/// `deleteConstraints` are each held to them on their own.
fn validate_rule_limits(
    constraints: &BTreeMap<String, PropertyConstraint>,
    structure_error: &dyn Fn(String) -> DataContractError,
    platform_version: &PlatformVersion,
) -> Result<(), DataContractError> {
    let limits = &platform_version.system_limits;
    let max_constraints = limits.max_property_constraints;
    if constraints.len() > usize::from(max_constraints) {
        return Err(structure_error(format!(
            "declares {} rules, above the maximum of {max_constraints}",
            constraints.len()
        )));
    }
    let max_aggregates = limits.max_property_constraint_aggregates;
    let aggregates = constraints
        .values()
        .flat_map(PropertyConstraint::aggregate_reads)
        .collect::<BTreeSet<_>>()
        .len();
    if aggregates > usize::from(max_aggregates) {
        return Err(structure_error(format!(
            "reads {aggregates} distinct countOf and sumOf totals, above the maximum of \
             {max_aggregates}"
        )));
    }
    let max_nodes = limits.max_property_constraint_nodes;
    for (name, constraint) in constraints {
        let nodes = constraint.node_count();
        if nodes > usize::from(max_nodes) {
            return Err(structure_error(format!(
                "rule \"{name}\" has {nodes} nodes, above the maximum of {max_nodes}"
            )));
        }
        if let Some((repeat, earlier)) = constraint.repeated_condition() {
            return Err(structure_error(format!(
                "rule \"{name}\" at {repeat} repeats the condition at {earlier}"
            )));
        }
    }
    Ok(())
}

/// Reads the `deleteConstraints` keyword onto the document type: the rules the
/// stored document must meet for its owner to delete it
/// ([`parse_delete_constraints`]). A rule reads what a `propertyConstraints`
/// rule may read, checked the same way ([`validate_property_constraint_reads`]),
/// except two refused there: `$old.`, since a delete rule is judged on the
/// stored document itself and its plain paths already read it ([`stored_path`]),
/// and a document a reference points at, which a delete does not check; under
/// full validation the keyword is held to the same limits, counted
/// apart from `propertyConstraints`. Which type a `countOf` or `sumOf` totals,
/// and by which keys, is checked with the contract's other types
/// ([`validate_property_constraint_aggregates`]).
///
/// On every parse the type must be one whose owner deletes its documents: not
/// `canBeDeleted: false` or `"onlyWhenConsumed"`, where there is no delete to
/// gate, and not indexOnly, whose delete carries the row's values instead of
/// naming a stored document. The keyword arrives with meta-schema v3 and
/// protocol version 14, so no stored contract breaks either rule. Called by
/// `apply_property_constraints_v0`, so one dispatch on `parse_property_constraints`
/// parses both keywords: `None` ignores this one too, which no earlier
/// meta-schema admits.
fn apply_delete_constraints_v0(
    document_type: &mut DocumentTypeV2,
    schema_defs: Option<&BTreeMap<String, Value>>,
    document_type_name: &str,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<(), DataContractError> {
    let property_kind =
        |path: &str| property_equality_kind(&document_type.flattened_properties, path);
    let constraints =
        parse_delete_constraints(&document_type.schema, document_type_name, &property_kind)?;
    if constraints.is_empty() {
        return Ok(());
    }
    let keyword = property_names::DELETE_CONSTRAINTS;
    let structure_error = |message: String| {
        DataContractError::InvalidContractStructure(format!(
            "document type \"{document_type_name}\" {keyword} {message}"
        ))
    };
    if document_type.documents_deleted_only_when_consumed {
        return Err(structure_error(
            "gate the delete of a document by its owner, but the type sets `canBeDeleted: \
             \"onlyWhenConsumed\"`: its owner never deletes one, and a consume deletes it \
             without the rules"
                .to_string(),
        ));
    }
    if !document_type.documents_can_be_deleted {
        return Err(structure_error(
            "gate the delete of a document by its owner, but the type sets `canBeDeleted: \
             false`, so there is no delete to gate"
                .to_string(),
        ));
    }
    if document_type.index_only {
        return Err(structure_error(
            "gate the delete of a stored document, but the type is indexOnly: its delete \
             carries the row's values, judged by the type's propertyConstraints as a create's"
                .to_string(),
        ));
    }

    for (name, constraint) in &constraints {
        validate_property_constraint_reads(
            document_type,
            schema_defs,
            &format!("rule \"{name}\""),
            constraint,
            false,
            false,
            &structure_error,
        )?;
    }

    if full_validation {
        validate_rule_limits(&constraints, &structure_error, platform_version)?;
    }

    document_type.delete_constraints = constraints;
    Ok(())
}

/// How a condition compares the property at the dotted `path` of
/// `flattened_properties` in an `equal`, `notEqual` or `in`: as a string or as
/// an identifier, `None` as an integer (or not at all). A typed array compares
/// as its elements do, in a `contains`; anywhere else the reads refuse an array
/// where a string or an identifier belongs. A byte array property is
/// `EqualityKind::Bytes`, which only a `startsWith` or an `endsWith` tests; a
/// typed array of byte arrays compares as nothing does.
pub(super) fn property_equality_kind(
    flattened_properties: &IndexMap<String, DocumentProperty>,
    path: &str,
) -> Option<EqualityKind> {
    let equality_kind = |property_type: &DocumentPropertyType| match property_type {
        DocumentPropertyType::String(_) => Some(EqualityKind::Text),
        property_type if property_type.is_identifier() => Some(EqualityKind::Identifier),
        _ => None,
    };
    match flattened_properties
        .get(path)
        .map(|property| &property.property_type)
    {
        Some(DocumentPropertyType::TypedArray(array)) => equality_kind(&array.item_type),
        Some(DocumentPropertyType::ByteArray(_)) => Some(EqualityKind::Bytes),
        Some(property_type) => equality_kind(property_type),
        None => None,
    }
}

/// Checks the reads of the type's `propertyConstraints` and `deleteConstraints`
/// rules that depend on which writes the type has, once its moderator abilities
/// are parsed: a rule reading `$revision`, of either version, needs a type that
/// keeps a revision, and a `propertyConstraints` rule reading the stored
/// version through `$old.` needs a write that replaces one (a
/// `deleteConstraints` rule reads none: [`stored_path`] refused it). Both are
/// the types whose documents are mutable, transferable, sold at a price their
/// owner sets, or keep fields for their moderators (`requires_revision`): on
/// any other a create is the only write a rule judges, and it has no stored
/// version and the first revision. On every parse; the keywords arrive with
/// protocol version 14, so no stored contract breaks it.
pub(super) fn validate_stored_version_reads(
    document_type: &DocumentTypeV2,
    document_type_name: &str,
) -> Result<(), DataContractError> {
    if document_type.requires_revision() {
        return Ok(());
    }
    let has_no_later_write = "its documents are immutable, not transferable, not sold at a \
                              price their owner sets, and keep no fields for their moderators";
    for (keyword, rules) in [
        (
            property_names::PROPERTY_CONSTRAINTS,
            &document_type.property_constraints,
        ),
        (
            property_names::DELETE_CONSTRAINTS,
            &document_type.delete_constraints,
        ),
    ] {
        for (name, rule) in rules {
            if let Some(read) = rule
                .system_reads()
                .into_iter()
                .find(|read| read.value == SystemValue::Revision)
            {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "document type \"{document_type_name}\" {keyword} rule \"{name}\" reads {}, \
                     but the type keeps no revision: {has_no_later_write}",
                    read.name()
                )));
            }
        }
    }
    if let Some(name) = document_type
        .property_constraints
        .iter()
        .find_map(|(name, rule)| rule.reads_stored().then_some(name))
    {
        return Err(DataContractError::InvalidContractStructure(format!(
            "document type \"{document_type_name}\" {} rule \"{name}\" reads the stored \
             version through {STORED_DOCUMENT_PREFIX}, but no write of the type replaces one: \
             {has_no_later_write}, so a create, which has none, is the only write a rule judges",
            property_names::PROPERTY_CONSTRAINTS
        )));
    }
    Ok(())
}

/// A path a condition reads, as the document type names it: `path` itself,
/// or, with `allow_stored_reads`, what follows the `$old.` a rule of
/// `propertyConstraints`, an `immutable` entry's condition or `retractedWhen`
/// reads the stored version through ([`STORED_DOCUMENT_PREFIX`]): a property,
/// since `$old.$ownerId` and the stored system values are no property paths.
/// A rule of `deleteConstraints` is judged on the stored document itself, so
/// its plain paths read it and `$old.` is refused there.
fn stored_path<'a>(
    path: &'a str,
    subject: &str,
    allow_stored_reads: bool,
) -> Result<&'a str, String> {
    match stored_version_path(path) {
        Some(_) if !allow_stored_reads => Err(stored_read_in_delete_rule(subject, path)),
        Some(stored) => Ok(stored),
        None => Ok(path),
    }
}

/// The refusal of a `deleteConstraints` rule, `subject`, reading `path` through
/// `$old.`: such a rule is judged on the stored document itself.
fn stored_read_in_delete_rule(subject: &str, path: &str) -> String {
    format!(
        "{subject} reads \"{path}\", but a deleteConstraints rule is judged on the stored \
         document itself, so its plain paths read it: name it without \
         `{STORED_DOCUMENT_PREFIX}`"
    )
}

/// Why `document_type` has no property at `lookup` a rule `subject` can read
/// as `read`, the message naming it `path`; `None` when it has one: of the kind
/// the read needs, neither transient nor inside a transient object. Shared by
/// the reads of the type's own properties and those of a referenced document.
pub(super) fn property_read_error(
    document_type: DocumentTypeRef,
    subject: &str,
    path: &str,
    lookup: &str,
    read: PropertyRead,
) -> Option<String> {
    let reads = match read {
        PropertyRead::Value => "reads",
        PropertyRead::Presence => "tests the presence of",
        PropertyRead::Text | PropertyRead::Identifier => "compares",
        PropertyRead::Length => "measures",
        PropertyRead::Count => "counts the items of",
        PropertyRead::Elements(_) => "looks in",
        PropertyRead::Bytes { .. } => "reads the bytes of",
    };
    match read {
        PropertyRead::Value => match document_type
            .flattened_properties()
            .get(lookup)
            .map(|property| &property.property_type)
        {
            // `is_integer` leaves out the 128-bit types, which the arithmetic holds
            // too; a boolean reads as 1 for true and 0 for false
            Some(property_type)
                if property_type.is_integer()
                    || matches!(
                        property_type,
                        DocumentPropertyType::U128
                            | DocumentPropertyType::I128
                            | DocumentPropertyType::Boolean
                    ) => {}
            // A string is compared with constants, never read as a number
            Some(DocumentPropertyType::String(_)) => {
                return Some(format!(
                    "{subject} reads \"{path}\", which has type string, not integer \
                     or boolean: a string property is compared, by equal or notEqual, \
                     with a {{ \"const\": ... }} or another string property, or with the \
                     strings an in lists"
                ));
            }
            // An identifier is compared with identifiers, never read as a number
            Some(property_type) if property_type.is_identifier() => {
                return Some(format!(
                    "{subject} reads \"{path}\", which has type identifier, not \
                     integer or boolean: an identifier property is compared, by equal or \
                     notEqual, with a {{ \"const\": base58 }} or another identifier \
                     property, or with the identifiers an in lists"
                ));
            }
            // A byte array is read a byte at a time
            Some(DocumentPropertyType::ByteArray(_)) => {
                return Some(format!(
                    "{subject} reads \"{path}\", which has type byteArray, not integer \
                     or boolean: byteAt reads one of its bytes, and count how many it holds"
                ));
            }
            Some(other) => {
                return Some(format!(
                    "{subject} reads \"{path}\", which has type {}, not integer or \
                     boolean",
                    other.name()
                ));
            }
            // An object is not in the flattened map either: only its members hold values
            None => {
                return Some(format!(
                    "{subject} reads \"{path}\", which is not an integer or boolean \
                     property of the document type (a nested one is named by its dotted \
                     path)"
                ));
            }
        },
        PropertyRead::Presence => {
            if property_at_path(document_type.properties(), lookup).is_none() {
                return Some(format!(
                    "{subject} tests the presence of \"{path}\", which is not a \
                     property of the document type (a nested one is named by its dotted \
                     path)"
                ));
            }
        }
        PropertyRead::Text => match document_type
            .flattened_properties()
            .get(lookup)
            .map(|property| &property.property_type)
        {
            Some(DocumentPropertyType::String(_)) => {}
            Some(other) => {
                return Some(format!(
                    "{subject} compares \"{path}\" with a string, but it has type \
                     {}, not string",
                    other.name()
                ));
            }
            None => {
                return Some(format!(
                    "{subject} compares \"{path}\" with a string, but it is not a \
                     string property of the document type (a nested one is named by its \
                     dotted path)"
                ));
            }
        },
        PropertyRead::Length => match document_type
            .flattened_properties()
            .get(lookup)
            .map(|property| &property.property_type)
        {
            Some(DocumentPropertyType::String(_)) => {}
            Some(other) => {
                return Some(format!(
                    "{subject} measures the length of \"{path}\", which has type {}, \
                     not string: count gives the items of an array or byte array",
                    other.name()
                ));
            }
            None => {
                return Some(format!(
                    "{subject} measures the length of \"{path}\", which is not a \
                     string property of the document type (a nested one is named by its \
                     dotted path)"
                ));
            }
        },
        PropertyRead::Count => match document_type
            .flattened_properties()
            .get(lookup)
            .map(|property| &property.property_type)
        {
            Some(
                DocumentPropertyType::TypedArray(_)
                | DocumentPropertyType::ByteArray(_)
                | DocumentPropertyType::Array(_)
                | DocumentPropertyType::VariableTypeArray(_),
            ) => {}
            Some(DocumentPropertyType::String(_)) => {
                return Some(format!(
                    "{subject} counts the items of \"{path}\", which has type \
                     string, not array: length or byteLength gives the size of a string"
                ));
            }
            Some(other) => {
                return Some(format!(
                    "{subject} counts the items of \"{path}\", which has type {}, \
                     not array or byteArray",
                    other.name()
                ));
            }
            None => {
                return Some(format!(
                    "{subject} counts the items of \"{path}\", which is not an array \
                     or byte array property of the document type (a nested one is named \
                     by its dotted path)"
                ));
            }
        },
        PropertyRead::Elements(kind) => match document_type
            .flattened_properties()
            .get(lookup)
            .map(|property| &property.property_type)
        {
            Some(DocumentPropertyType::TypedArray(array)) => {
                let element_type = array.item_type.as_ref();
                let (holds, kind_name) = match kind {
                    ElementKind::Integer => (
                        element_type.is_integer()
                            || matches!(
                                element_type,
                                DocumentPropertyType::U128 | DocumentPropertyType::I128
                            ),
                        "an integer",
                    ),
                    ElementKind::Text => (
                        matches!(element_type, DocumentPropertyType::String(_)),
                        "a string",
                    ),
                    ElementKind::Identifier => (element_type.is_identifier(), "an identifier"),
                };
                if !holds {
                    return Some(format!(
                        "{subject} looks in \"{path}\" for {kind_name}, but its \
                         elements have type {}: contains looks for an integer, a string \
                         or an identifier among elements of that type",
                        element_type.name()
                    ));
                }
            }
            Some(other) => {
                return Some(format!(
                    "{subject} looks in \"{path}\", which has type {}, not an \
                     array with items",
                    other.name()
                ));
            }
            None => {
                return Some(format!(
                    "{subject} looks in \"{path}\", which is not an array property \
                     of the document type (a nested one is named by its dotted path)"
                ));
            }
        },
        PropertyRead::Bytes { extent } => match document_type
            .flattened_properties()
            .get(lookup)
            .map(|property| &property.property_type)
        {
            // A byte past the most the array may hold is never there
            Some(DocumentPropertyType::ByteArray(sizes)) => {
                if let Some(max_size) = sizes
                    .max_size
                    .filter(|max_size| extent > usize::from(*max_size))
                {
                    return Some(format!(
                        "{subject} reads \"{path}\" as at least {extent} bytes long, but \
                         its maxItems is {max_size}: the byte a byteAt reads, or the \
                         constant it must start or end with, is never there"
                    ));
                }
            }
            Some(property_type) if property_type.is_identifier() => {
                return Some(format!(
                    "{subject} reads the bytes of \"{path}\", which has type identifier, \
                     not byteArray: an identifier property is compared whole, by equal, \
                     notEqual or in"
                ));
            }
            Some(other) => {
                return Some(format!(
                    "{subject} reads the bytes of \"{path}\", which has type {}, not \
                     byteArray",
                    other.name()
                ));
            }
            None => {
                return Some(format!(
                    "{subject} reads the bytes of \"{path}\", which is not a byte array \
                     property of the document type (a nested one is named by its dotted \
                     path)"
                ));
            }
        },
        PropertyRead::Identifier => match document_type
            .flattened_properties()
            .get(lookup)
            .map(|property| &property.property_type)
        {
            Some(property_type) if property_type.is_identifier() => {}
            Some(other) => {
                return Some(format!(
                    "{subject} compares \"{path}\" with an identifier, but it has \
                     type {}, not identifier",
                    other.name()
                ));
            }
            None => {
                return Some(format!(
                    "{subject} compares \"{path}\" with an identifier, but it is \
                     not an identifier property of the document type (a nested one is \
                     named by its dotted path)"
                ));
            }
        },
    }
    if is_transient(document_type, lookup) {
        return Some(format!(
            "{subject} {reads} \"{path}\", which is transient or inside a \
             transient object: a transient value is never stored, so a stored document \
             could not be held to the rule"
        ));
    }
    None
}

/// Checks every property `constraint` reads against `document_type`, as
/// [`apply_property_constraints`] describes for a rule: each read names a
/// property of the kind it reads, neither transient nor inside a transient
/// object, a system time or height the type records, and a constant or a
/// default its string property's `enum` admits. `subject` names the condition
/// in the errors (`rule "fee"`), and `structure_error` wraps them. With
/// `allow_stored_reads` (a rule of `propertyConstraints`, the condition of an
/// `immutable` entry or `retractedWhen`) a path may read the stored version
/// through `$old.` and is judged as the path after it, and a stored system
/// value as the value after it; without it (a rule of `deleteConstraints`)
/// neither is admitted. Whether the type keeps the revision `$revision` reads,
/// and has a write replacing a stored version at all, is checked once its
/// moderator abilities are parsed ([`validate_stored_version_reads`]).
/// With `allow_referenced_reads` (a `propertyConstraints` rule) a
/// `referenced` operand may read a document one of the type's references
/// points at: what it reads is checked against the referenced type once it is
/// in hand ([`referenced_reads_error`]), and which reference it reads through
/// by [`validate_referencing_rules`].
pub(super) fn validate_property_constraint_reads(
    document_type: &DocumentTypeV2,
    schema_defs: Option<&BTreeMap<String, Value>>,
    subject: &str,
    constraint: &PropertyConstraint,
    allow_stored_reads: bool,
    allow_referenced_reads: bool,
    structure_error: &dyn Fn(String) -> DataContractError,
) -> Result<(), DataContractError> {
    // What a rule reads of a referenced document is checked with its reference
    if !allow_referenced_reads && constraint.reads_referenced() {
        return Err(structure_error(format!(
            "{subject} reads a value of the document a reference points at, which only a \
             propertyConstraints rule does: its writes are the ones that check the references"
        )));
    }
    for (path, read) in constraint.own_property_reads() {
        let lookup = stored_path(path, subject, allow_stored_reads).map_err(structure_error)?;
        if let Some(error) = property_read_error(
            DocumentTypeRef::V2(document_type),
            subject,
            path,
            lookup,
            read,
        ) {
            return Err(structure_error(error));
        }
    }
    // `$old.$ownerId` is no property path, so its refusal in a delete rule is here
    if !allow_stored_reads && constraint.reads_stored() {
        let path = constraint
            .system_reads()
            .into_iter()
            .find(|read| read.stored)
            .map_or_else(|| STORED_OWNER_ID.to_string(), SystemRead::name);
        return Err(structure_error(stored_read_in_delete_rule(subject, &path)));
    }
    // An indexOnly type keeps no stored version: its documents are never
    // replaced. Checked first, so that a rule reading `$old.$ownerId` is told so
    if document_type.index_only && constraint.reads_stored() {
        return Err(structure_error(format!(
            "{subject} reads the stored version through {STORED_DOCUMENT_PREFIX}, which an \
             indexOnly type does not keep: its documents are never replaced"
        )));
    }
    // Its delete carries its row's values but not its owner, so a rule reading
    // the owner could not be judged there
    if document_type.index_only && constraint.reads_owner() {
        return Err(structure_error(format!(
            "{subject} compares $ownerId, which a delete of an indexOnly document \
             does not carry"
        )));
    }
    for read in constraint.system_reads() {
        let system_name = read.name();
        // Nor its times, heights and revision
        if document_type.index_only {
            return Err(structure_error(format!(
                "{subject} reads {system_name}, which a delete of an indexOnly \
                 document does not carry"
            )));
        }
        // A stored document holds only the times and heights its type requires;
        // whether it keeps a revision is known once its moderator abilities are
        if let SystemValue::Property(property) = read.value {
            if !document_type.required_fields.contains(property.name()) {
                let listed = if read.stored { property.name() } else { "it" };
                return Err(structure_error(format!(
                    "{subject} reads {system_name}, which the document type does not \
                     record: list {listed} in required"
                )));
            }
        }
    }
    // Which type an aggregate totals, and by which of its keys, is checked once
    // every document type of the contract is parsed
    for read in constraint.aggregate_reads() {
        let operator = read.wire_name();
        // Nor a total read from state, which its delete is not given
        if document_type.index_only {
            return Err(structure_error(format!(
                "{subject} reads a {operator}, which a delete of an indexOnly \
                 document is not given"
            )));
        }
        // The value a key is matched by is always there, so that every write reads
        // the total of the documents matching it: a property the document could leave
        // out, or whose enclosing object it could, would match nothing
        for binding in read.filter.values() {
            let AggregateBinding::Property { path, .. } = binding else {
                continue;
            };
            let mut prefix = String::new();
            for segment in path.split('.') {
                if !prefix.is_empty() {
                    prefix.push('.');
                }
                prefix.push_str(segment);
                if !document_type.required_fields.contains(&prefix) {
                    return Err(structure_error(format!(
                        "{subject} matches a {operator} by \"{path}\", but the \
                         document type does not require \"{prefix}\": a value a \
                         {operator} matches by must always be there, so list it in required"
                    )));
                }
            }
        }
    }
    // A constant or a default a string property's `enum` does not list is a
    // typo: the property could never hold it. The enums of a referenced type
    // are checked with what is read of it
    let lookup = |path: &str| -> Result<Option<String>, DataContractError> {
        if path.starts_with(REFERENCED_DOCUMENT_PREFIX) {
            return Ok(None);
        }
        stored_path(path, subject, allow_stored_reads)
            .map(|lookup| Some(lookup.to_string()))
            .map_err(structure_error)
    };
    if let Some(reason) = enum_miss(
        constraint,
        subject,
        &document_type.schema,
        schema_defs,
        &lookup,
        &|path| path.to_string(),
    )? {
        return Err(structure_error(reason));
    }
    Ok(())
}

/// Checks the declaring side of every `propertyConstraints` rule of
/// `document_type`, named `name`, that reads a document one of its references
/// points at (`{ "referenced": [reference, field] }`), once its properties,
/// references and moderator fields are parsed:
///
/// * the reference is a top-level property of the type, not transient, holding
///   one reference to one document: `permanentDocument` or
///   `deletableDocument` by id or by a `findBy` without a function, or
///   `moderatedDocument`, not the elements of a typed array, an `anyOf` or
///   `allOf`, an `inList` or a `findBy` function, which is judged on the
///   create alone ([`referencing_property_error`]);
/// * the type is not indexOnly: the delete of such a document is judged by
///   the rules but checks no reference;
/// * on a type whose documents can be transferred or traded, the rule does not
///   read what a transfer or a purchase changes, nor, on a type whose prices
///   can be set, what a price update changes: those writes check no reference,
///   so they could not judge it;
/// * on a type keeping fields for its moderators to change, it reads nothing
///   their change can break, which their change is judged against
///   ([`PropertyConstraint::reads_changed_fields`]): such a field, of either
///   version, `$revision`, or any total, which may have moved since the
///   document was last written; their change checks no reference either;
/// * on a type whose moderators' deletions keep records, it reads no total: a
///   moderator's restore is judged against every rule reading one
///   ([`PropertyConstraint::reads_change`]), the totals having perhaps moved
///   while the document was gone, and checks no reference, so it could not
///   judge the rule.
///
/// What it reads of the referenced type is checked once that type is in hand
/// ([`referenced_reads_error`]). Registration only, like the rest of a rule's
/// limits: a stored contract passed it.
pub(super) fn validate_referencing_rules(
    document_type: DocumentTypeRef,
    name: &str,
) -> Result<(), DataContractError> {
    let can_change_owner = owner_can_change(document_type);
    let can_change_price = document_type.trade_mode() != TradeMode::None;
    let can_restore = document_type.moderator_deletions_keep_records();
    for (rule, constraint) in document_type.property_constraints() {
        let reads = constraint.referenced_reads();
        if reads.is_empty() {
            continue;
        }
        let error = |message: String| {
            DataContractError::InvalidContractStructure(format!(
                "document type \"{name}\" {} rule \"{rule}\" {message}",
                property_names::PROPERTY_CONSTRAINTS
            ))
        };
        if document_type.index_only() {
            return Err(error(
                "reads a document a reference points at, but the type is indexOnly: the delete \
                 of one of its documents is judged by its rules and checks no reference"
                    .to_string(),
            ));
        }
        let references: BTreeSet<&str> = reads.iter().map(|(reference, _, _)| *reference).collect();
        for reference in references {
            if let Some(reason) = referencing_property_error(document_type, reference) {
                return Err(error(format!("reads through \"{reference}\", {reason}")));
            }
        }
        if can_change_owner && constraint.reads_change(SystemChange::Transfer) {
            return Err(error(
                "reads a document a reference points at and what a transfer or a purchase \
                 changes ($ownerId, $transferredAt and its heights or $revision, of either \
                 version, or a total depending on the owner): a transfer or a purchase checks \
                 no reference, so it could not judge the rule"
                    .to_string(),
            ));
        }
        if can_change_price && constraint.reads_change(SystemChange::PriceUpdate) {
            return Err(error(
                "reads a document a reference points at and what a price update changes \
                 ($updatedAt and its heights or $revision, of either version): a price update \
                 checks no reference, so it could not judge the rule"
                    .to_string(),
            ));
        }
        // What a moderator's change of the fields the type keeps for them is
        // judged against, the reason named as precisely as the rule allows: a
        // field it reads, of either version, or keys a total over the type's
        // own documents by, `$revision`, or a total
        let moderator_fields = document_type.moderator_changeable_fields();
        if !moderator_fields.is_empty() && constraint.reads_changed_fields(moderator_fields) {
            let moderated_field = constraint
                .own_property_reads()
                .into_iter()
                .map(|(path, _)| either_version_path(path))
                .chain(
                    constraint
                        .aggregate_reads()
                        .into_iter()
                        .flat_map(AggregateRead::own_type_keys),
                )
                .find(|path| is_path_listed(moderator_fields, path));
            let reads_revision = constraint
                .system_reads()
                .into_iter()
                .any(|read| read.value == SystemValue::Revision);
            let reason = match (moderated_field, reads_revision) {
                (Some(field), _) => format!(
                    "depends on \"{field}\" (a property it reads, of either version, or a key \
                     of a total over the type's own documents), which the type keeps for its \
                     moderators to change"
                ),
                (None, true) => "$revision, which a moderator's change of the fields the type \
                                 keeps for them moves"
                    .to_string(),
                (None, false) => "a total, which a moderator's change of the fields the type \
                                  keeps for them is judged against, since it may have moved \
                                  since the document was last written"
                    .to_string(),
            };
            return Err(error(format!(
                "reads a document a reference points at and {reason}: their change checks no \
                 reference, so it could not judge the rule"
            )));
        }
        if can_restore && constraint.reads_change(SystemChange::Restore) {
            return Err(error(
                "reads a document a reference points at and a total, which a moderator's \
                 restore is judged against, since it may have moved while the document was \
                 gone: a restore checks no reference, so it could not judge the rule"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

/// What the property `reference` of `document_type` refers to, as every
/// reference check reads it ([`DocumentTypeRef::reference_declarations`]): its
/// value's target, its elements', a key id's, or the one its value reveals
/// into a `findBy` key. `None` for a property holding no reference.
fn property_reference<'a>(
    document_type: DocumentTypeRef<'a>,
    reference: &str,
) -> Option<PropertyReference<'a>> {
    document_type
        .reference_declarations()
        .find_map(|(holder, property_reference)| {
            (holder.path() == reference).then_some(property_reference)
        })
}

/// Why a rule of `document_type` may not read a document through its property
/// `reference`, `None` when it may: it must be a top-level property of the
/// type, not transient, holding one reference to one document it finds by id
/// or by a `findBy` without a function (a string or byte array property whose
/// value `findBy` reads included), so the reference check reads exactly that
/// document whenever the rule is judged.
fn referencing_property_error(document_type: DocumentTypeRef, reference: &str) -> Option<String> {
    if !document_type.flattened_properties().contains_key(reference) {
        return Some(
            "which is not a property of the document type holding a reference: a \
             reference read through is a top-level property"
                .to_string(),
        );
    }
    if is_transient(document_type, reference) {
        return Some(
            "which is transient: the reference a stored document was judged through would \
             not be stored"
                .to_string(),
        );
    }
    let target = match property_reference(document_type, reference) {
        Some(PropertyReference::Value(target) | PropertyReference::Revealed(target)) => target,
        Some(PropertyReference::Elements { .. }) => {
            return Some(
                "whose refersTo is on the elements of a typed array, which point at several \
                 documents"
                    .to_string(),
            )
        }
        Some(PropertyReference::KeyId(_)) => {
            return Some("whose refersTo names an identity key, not a document".to_string())
        }
        None => return Some("which carries no refersTo".to_string()),
    };
    if target.combinator().is_some() {
        return Some(
            "whose refersTo is an anyOf or allOf, which may hold through one document or \
             another"
                .to_string(),
        );
    }
    let Some(declaration) = target.as_any_document_reference() else {
        return Some(
            "whose refersTo names an identity, a contract, a token or a key, not a document"
                .to_string(),
        );
    };
    if declaration.in_list.is_some() {
        return Some(
            "whose refersTo has inList: its value is an element of a list, not a document"
                .to_string(),
        );
    }
    if declaration
        .lookup
        .is_some_and(|lookup| lookup.hash_key().is_some())
    {
        return Some(
            "whose findBy computes its key with a function: such a reference is judged on the \
             create alone, and a replace, which every rule judges, reads no document through it"
                .to_string(),
        );
    }
    None
}

/// Why a `propertyConstraints` rule, `subject` in the messages, can not read
/// what it reads through its reference `reference` of a document of
/// `referenced`, a type the reference declares of `kind`; `None` when it can.
/// `referenced_schema_defs` are the `$defs` of the referenced type's contract,
/// which resolve the `enum`s its constants are checked against, and
/// `records_creator_id` whether the referenced type records `$creatorId`.
///
/// * `$id`, `$ownerId` and `$creatorId` are identifiers, compared by `equal`,
///   `notEqual`, `in` or `contains`; `$creatorId` only of a type that records
///   it.
/// * A time or height is an integer, of a type that lists it in `required`.
/// * A schema property is of the kind the rule reads it as, stored (not
///   `transient` nor inside a transient object), and a string constant or
///   default compared with it, or one it must start or end with, fits its
///   `enum`.
/// * Through a `moderatedDocument`, which may resolve to the record of its
///   removal: `$id`, `$ownerId` or a field the referenced type keeps under
///   `moderatorAbilities.deleteKeepsFields` (or one inside a kept object),
///   which every record holds, since a rule is judged on every replace.
///
/// Checked by the contract parse for a type of the same contract
/// (`InvalidContractStructure`, 10231), and at registration against the
/// stored contract for another contract's (`ReferencedDocumentRuleReadInvalidError`,
/// 40148).
pub fn referenced_reads_error(
    rule: &PropertyConstraint,
    subject: &str,
    reference: &str,
    kind: DocumentReferenceKind,
    referenced: DocumentTypeRef,
    referenced_schema_defs: Option<&BTreeMap<String, Value>>,
    records_creator_id: bool,
) -> Result<Option<String>, DataContractError> {
    let label = |field: &str| format!("{reference}.{field}");
    for (read_reference, field, read) in rule.referenced_reads() {
        if read_reference != reference {
            continue;
        }
        if kind == DocumentReferenceKind::Moderated
            && field != ID
            && field != OWNER_ID
            && !is_path_listed(referenced.moderator_deletion_kept_fields(), field)
        {
            return Ok(Some(format!(
                "{subject} reads \"{}\" through a moderatedDocument reference: once a \
                 moderator removes the document only its record is left, which keeps $id, \
                 $ownerId and the fields its type lists under deleteKeepsFields, and a rule is \
                 judged on every replace",
                label(field)
            )));
        }
        if is_referenced_system_agreement_property(field) {
            if read != PropertyRead::Identifier {
                return Ok(Some(format!(
                    "{subject} reads \"{}\", an identifier, which a rule compares by equal, \
                     notEqual or in, or looks for with contains",
                    label(field)
                )));
            }
            if field == CREATOR_ID && !records_creator_id {
                return Ok(Some(format!(
                    "{subject} reads \"{}\", but the referenced document type does not \
                     record $creatorId: only a transferable or tradeable document type of a \
                     format-1 contract does",
                    label(field)
                )));
            }
            continue;
        }
        if let Some(system) = SystemProperty::from_name(field) {
            if read != PropertyRead::Value {
                return Ok(Some(format!(
                    "{subject} reads \"{}\", a time or height, which a rule reads as an \
                     integer",
                    label(field)
                )));
            }
            if !referenced.required_fields().contains(system.name()) {
                return Ok(Some(format!(
                    "{subject} reads \"{}\", which the referenced document type does not \
                     record: it must list it in required",
                    label(field)
                )));
            }
            continue;
        }
        // A field the parse compares as a string, not knowing the referenced
        // type, that holds something else: read as what it holds
        match referenced_textual_read(rule, subject, referenced, reference, field, read, &label) {
            Some(Ok(())) => continue,
            Some(Err(reason)) => return Ok(Some(reason)),
            None => {}
        }
        if let Some(reason) = property_read_error(referenced, subject, &label(field), field, read) {
            return Ok(Some(reason));
        }
    }
    // A constant or a default the referenced property's `enum` does not list
    let lookup = |path: &str| -> Result<Option<String>, DataContractError> {
        Ok(referenced_read(path).and_then(|(read_reference, field)| {
            (read_reference == reference && !field.starts_with('$')).then(|| field.to_string())
        }))
    };
    if let Some(reason) = enum_miss(
        rule,
        subject,
        referenced.schema(),
        referenced_schema_defs,
        &lookup,
        &|path| referenced_read(path).map_or_else(|| path.to_string(), |(_, field)| label(field)),
    )? {
        return Ok(Some(reason));
    }
    Ok(None)
}

/// How a rule reads `field` of `referenced` through `reference` where its
/// parse, which does not know the referenced type, compares it as a string
/// (`read`: a [`PropertyRead::Text`] read, or the string elements of an array
/// a `contains` looks in) though it holds something else. `None` when the
/// field holds strings, or the read is no such one, for the ordinary checks.
/// Otherwise the field is read as what it holds, which is sound when every
/// string constant compared with it spells a value of its type (base58 for an
/// identifier, hex of a size it takes for a byte array, which alone has a
/// start and an end a rule tests), no string default is given it, and every
/// property compared with it is another field of a referenced document
/// holding the same kind ([`referenced_peer_kind_error`] for one of another
/// reference): `Some(Ok(()))`, and `Some(Err(reason))` when not.
fn referenced_textual_read(
    rule: &PropertyConstraint,
    subject: &str,
    referenced: DocumentTypeRef,
    reference: &str,
    field: &str,
    read: PropertyRead,
    label: &dyn Fn(&str) -> String,
) -> Option<Result<(), String>> {
    let property_type = &referenced.flattened_properties().get(field)?.property_type;
    let (kind, element_type) = match (read, property_type) {
        (PropertyRead::Text, property_type) => {
            (WhereLiteralKind::of(property_type)?, property_type)
        }
        (PropertyRead::Elements(ElementKind::Text), DocumentPropertyType::TypedArray(array)) => {
            let item_type = array.item_type.as_ref();
            (WhereLiteralKind::of(item_type)?, item_type)
        }
        _ => return None,
    };
    if kind == WhereLiteralKind::Text {
        return None;
    }
    let path = referenced_path(reference, field);
    let named = label(field);
    let held = match kind {
        WhereLiteralKind::Integer => "an integer",
        WhereLiteralKind::Boolean => "a boolean",
        WhereLiteralKind::Identifier => "an identifier",
        WhereLiteralKind::Bytes => "a byte array",
        WhereLiteralKind::Text => "a string",
    };
    let refuse = |reason: String| Some(Err(format!("{subject} {reason}")));
    if rule
        .text_defaults()
        .iter()
        .any(|(read_path, _)| *read_path == path)
    {
        return refuse(format!(
            "gives \"{named}\" a string default, but it holds {held}"
        ));
    }
    for (read_path, affix, _) in rule.text_affixes() {
        if read_path != path {
            continue;
        }
        if kind != WhereLiteralKind::Bytes {
            return refuse(format!(
                "tests whether \"{named}\" starts or ends with \"{affix}\", but it holds \
                 {held}, which has no start or end a rule tests"
            ));
        }
        if hex::decode(affix).is_err() {
            return refuse(format!(
                "tests whether \"{named}\" starts or ends with \"{affix}\", which is not hex, \
                 as a byte array is written"
            ));
        }
    }
    for (read_path, constant) in rule.text_constants() {
        if read_path != path {
            continue;
        }
        let spells = match kind {
            WhereLiteralKind::Identifier => {
                Identifier::from_string(constant, Encoding::Base58).is_ok()
            }
            WhereLiteralKind::Bytes => hex::decode(constant).is_ok_and(|bytes| {
                let DocumentPropertyType::ByteArray(sizes) = element_type else {
                    return true;
                };
                !sizes
                    .min_size
                    .is_some_and(|minimum| bytes.len() < usize::from(minimum))
                    && !sizes
                        .max_size
                        .is_some_and(|maximum| bytes.len() > usize::from(maximum))
            }),
            WhereLiteralKind::Integer | WhereLiteralKind::Boolean | WhereLiteralKind::Text => false,
        };
        if !spells {
            return refuse(format!(
                "compares \"{named}\", which holds {held}, with \"{constant}\", which spells \
                 none: an identifier is written base58, a byte array hex of a size it takes, \
                 and an integer or a boolean is compared with no string"
            ));
        }
    }
    for (left, right) in rule.text_peers() {
        let other = if left == path {
            right
        } else if right == path {
            left
        } else {
            continue;
        };
        let Some((other_reference, other_field)) = referenced_read(other) else {
            return refuse(format!(
                "compares \"{named}\", which holds {held}, with \"{other}\", a string \
                 property of the type"
            ));
        };
        if other_reference == reference
            && WhereLiteralKind::of_referenced(referenced, other_field) != Some(kind)
        {
            return refuse(format!(
                "compares \"{named}\", which holds {held}, with \"{}\", which does not",
                label(other_field)
            ));
        }
    }
    Some(Ok(()))
}

/// Why `rule`, `subject` in the messages, compares two fields of documents
/// different references point at that hold different kinds of value (a string
/// with an identifier, say), so that the comparison could never hold; `None`
/// when it does not. `kind_of` gives what the field of the document a
/// reference points at holds, `None` when the referenced type is not in hand
/// (another contract's, at the contract parse), which leaves the pair to
/// registration. A pair through one reference is checked with its other reads
/// ([`referenced_reads_error`]).
pub fn referenced_peer_kind_error(
    rule: &PropertyConstraint,
    subject: &str,
    kind_of: &dyn Fn(&str, &str) -> Option<WhereLiteralKind>,
) -> Option<String> {
    referenced_peer_kind_mismatch(rule, subject, kind_of).map(|(_, reason)| reason)
}

/// [`referenced_peer_kind_error`] with the reference of the pair's first
/// side, which the registration error names.
pub fn referenced_peer_kind_mismatch<'a>(
    rule: &'a PropertyConstraint,
    subject: &str,
    kind_of: &dyn Fn(&str, &str) -> Option<WhereLiteralKind>,
) -> Option<(&'a str, String)> {
    rule.text_peers().into_iter().find_map(|(left, right)| {
        let (left_reference, left_field) = referenced_read(left)?;
        let (right_reference, right_field) = referenced_read(right)?;
        if left_reference == right_reference {
            return None;
        }
        let left_kind = kind_of(left_reference, left_field)?;
        let right_kind = kind_of(right_reference, right_field)?;
        (left_kind != right_kind).then(|| {
            (
                left_reference,
                format!(
                    "{subject} compares \"{left_reference}.{left_field}\" with \
                     \"{right_reference}.{right_field}\", which hold different kinds of value, \
                     so the two never agree"
                ),
            )
        })
    })
}

/// Why the literals of a `where` entry keyed by `property` of `referenced`,
/// whose value reads as `kind`, leave the entry unable to hold, `None` when
/// they do not: each literal reads as `kind` ([`WhereLiteral::value_for`]) and
/// is a value the property can hold ([`where_literal_error`]), an `in` lists
/// values distinct once read (`"cafe"` and `"CAFE"` are one byte array), and
/// an ordering's integer leaves a value the property can hold on its side.
/// Registration only, with the referenced type in hand: a stored contract
/// passed it.
pub fn where_condition_literal_error(
    referenced: DocumentTypeRef,
    referenced_schema_defs: Option<&BTreeMap<String, Value>>,
    property: &str,
    kind: WhereLiteralKind,
    condition: &WhereCondition,
) -> Result<Option<String>, DataContractError> {
    match condition {
        WhereCondition::Constant(literal)
        | WhereCondition::NotEqual(WhereNotEqualBound::Constant(literal)) => {
            where_literal_error(referenced, referenced_schema_defs, property, kind, literal)
        }
        WhereCondition::In(literals) => {
            let mut read: Vec<Value> = Vec::with_capacity(literals.len());
            for literal in literals {
                if let Some(reason) = where_literal_error(
                    referenced,
                    referenced_schema_defs,
                    property,
                    kind,
                    literal,
                )? {
                    return Ok(Some(reason));
                }
                let Some(value) = literal.value_for(kind) else {
                    continue;
                };
                if read.iter().any(|earlier| earlier.same_scalar_data(&value)) {
                    return Ok(Some(format!(
                        "the literal {literal} reads as a value an earlier literal of the in \
                         already is"
                    )));
                }
                read.push(value);
            }
            Ok(None)
        }
        WhereCondition::Ordering {
            ordering,
            bound: WhereOrderingBound::Constant(bound),
        } => {
            let Some((minimum, maximum)) =
                integer_range(referenced, referenced_schema_defs, property)?
            else {
                return Ok(None);
            };
            let can_hold = match ordering {
                WhereOrdering::GreaterThan => maximum > *bound,
                WhereOrdering::GreaterThanOrEqual => maximum >= *bound,
                WhereOrdering::LessThan => minimum < *bound,
                WhereOrdering::LessThanOrEqual => minimum <= *bound,
            };
            Ok((!can_hold).then(|| {
                format!(
                    "no value the referenced property holds, from {minimum} to {maximum}, is \
                     {} {bound}",
                    ordering.wire_name()
                )
            }))
        }
        _ => Ok(None),
    }
}

/// Why `literal`, compared with `property` of `referenced` read as `kind`,
/// can never equal a value the property holds, `None` when it can: it reads as
/// `kind` and, for a schema property, is one of its `enum` values, of a length
/// (a string) or a size (a byte array) it takes, and inside its range (an
/// integer).
fn where_literal_error(
    referenced: DocumentTypeRef,
    referenced_schema_defs: Option<&BTreeMap<String, Value>>,
    property: &str,
    kind: WhereLiteralKind,
    literal: &WhereLiteral,
) -> Result<Option<String>, DataContractError> {
    let Some(value) = literal.value_for(kind) else {
        return Ok(Some(format!(
            "the literal {literal} is not {}, as the referenced property holds",
            kind.describe()
        )));
    };
    if is_referenced_system_agreement_property(property) {
        return Ok(None);
    }
    let Some(declared) = referenced.flattened_properties().get(property) else {
        return Ok(None);
    };
    let reason = match (&declared.property_type, &value) {
        (DocumentPropertyType::String(sizes), Value::Text(text)) => {
            let length = text.chars().count();
            if sizes
                .min_length
                .is_some_and(|minimum| length < usize::from(minimum))
                || sizes
                    .max_length
                    .is_some_and(|maximum| length > usize::from(maximum))
                || sizes
                    .max_bytes
                    .is_some_and(|maximum| text.len() > usize::from(maximum))
            {
                Some("is not of a length the referenced property takes".to_string())
            } else if !enum_admits(referenced.schema(), referenced_schema_defs, property, text)? {
                Some("is not one of the referenced property's enum values".to_string())
            } else {
                None
            }
        }
        (DocumentPropertyType::ByteArray(sizes), Value::Bytes(bytes)) => {
            let size = bytes.len();
            (sizes
                .min_size
                .is_some_and(|minimum| size < usize::from(minimum))
                || sizes
                    .max_size
                    .is_some_and(|maximum| size > usize::from(maximum)))
            .then(|| "is not of a size the referenced property takes".to_string())
        }
        (_, value) if kind == WhereLiteralKind::Integer => {
            match (
                integer_range(referenced, referenced_schema_defs, property)?,
                value.to_integer::<i128>().ok(),
            ) {
                (Some((minimum, maximum)), Some(integer))
                    if integer < minimum || integer > maximum =>
                {
                    Some(format!(
                        "is outside the values the referenced property holds, {minimum} to \
                         {maximum}"
                    ))
                }
                _ => None,
            }
        }
        _ => None,
    };
    Ok(reason.map(|reason| format!("the literal {literal} {reason}")))
}

/// The integers the property `property` of `referenced` can hold: those of
/// its type, inside its schema's `minimum` and `maximum`. `None` for a
/// property that is no integer.
fn integer_range(
    referenced: DocumentTypeRef,
    referenced_schema_defs: Option<&BTreeMap<String, Value>>,
    property: &str,
) -> Result<Option<(i128, i128)>, DataContractError> {
    let Some(declared) = referenced.flattened_properties().get(property) else {
        return Ok(None);
    };
    let (mut minimum, mut maximum): (i128, i128) = match declared.property_type {
        DocumentPropertyType::U8 => (0, u8::MAX.into()),
        DocumentPropertyType::I8 => (i8::MIN.into(), i8::MAX.into()),
        DocumentPropertyType::U16 => (0, u16::MAX.into()),
        DocumentPropertyType::I16 => (i16::MIN.into(), i16::MAX.into()),
        DocumentPropertyType::U32 => (0, u32::MAX.into()),
        DocumentPropertyType::I32 => (i32::MIN.into(), i32::MAX.into()),
        DocumentPropertyType::U64 => (0, u64::MAX.into()),
        DocumentPropertyType::I64 => (i64::MIN.into(), i64::MAX.into()),
        DocumentPropertyType::U128 => (0, i128::MAX),
        DocumentPropertyType::I128 => (i128::MIN, i128::MAX),
        _ => return Ok(None),
    };
    if let Some(schema) = schema_at_path(referenced.schema(), referenced_schema_defs, property)? {
        if let Some(declared_minimum) = schema
            .get(property_names::MINIMUM)
            .and_then(|value| whole_integer(value))
        {
            minimum = minimum.max(declared_minimum);
        }
        if let Some(declared_maximum) = schema
            .get(property_names::MAXIMUM)
            .and_then(|value| whole_integer(value))
        {
            maximum = maximum.min(declared_maximum);
        }
    }
    Ok(Some((minimum, maximum)))
}

/// Checks what every `propertyConstraints` rule of `document_types`, one
/// contract's, reads of a document of the same contract one of its references
/// points at ([`referenced_reads_error`]), once all of them are parsed. A
/// reference into a type the contract does not have, or of a kind the type
/// does not admit, is left to the reference checks, which report it, and one
/// into another contract to registration, which reads that contract.
/// `contract_id`, `contract_system_version` and `contract_config_version`
/// describe the contract, whose types record `$creatorId` as they say.
pub(in crate::data_contract::document_type::class_methods) fn validate_property_constraint_referenced_reads(
    document_types: &BTreeMap<String, DocumentType>,
    contract_id: Identifier,
    contract_system_version: u16,
    contract_config_version: u16,
    schema_defs: Option<&BTreeMap<String, Value>>,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    for (type_name, document_type) in document_types {
        let declaring = document_type.as_ref();
        for (rule, constraint) in declaring.property_constraints() {
            let references: BTreeSet<&str> = constraint
                .referenced_reads()
                .into_iter()
                .map(|(reference, _, _)| reference)
                .collect();
            // The referenced type of the same contract a reference points at
            let same_contract = |reference: &str| {
                let declaration = property_reference(declaring, reference)?
                    .target()?
                    .as_any_document_reference()?;
                if declaration
                    .contract_id
                    .is_some_and(|referenced_contract| referenced_contract != contract_id)
                {
                    return None;
                }
                let referenced = document_types.get(declaration.document_type_name)?.as_ref();
                (declaration.kind == referenced.document_reference_kind())
                    .then_some((declaration, referenced))
            };
            if let Some(reason) = referenced_peer_kind_error(
                constraint,
                &format!("rule \"{rule}\""),
                &|reference, field| {
                    same_contract(reference).and_then(|(_, referenced)| {
                        WhereLiteralKind::of_referenced(referenced, field)
                    })
                },
            ) {
                return Err(consensus_or_protocol_data_contract_error(
                    DataContractError::InvalidContractStructure(format!(
                        "document type \"{type_name}\" {} {reason}",
                        property_names::PROPERTY_CONSTRAINTS
                    )),
                ));
            }
            for reference in references {
                let Some((declaration, referenced)) = same_contract(reference) else {
                    continue;
                };
                let records_creator_id = referenced.should_use_creator_id(
                    contract_system_version,
                    contract_config_version,
                    platform_version,
                )?;
                let reason = referenced_reads_error(
                    constraint,
                    &format!("rule \"{rule}\""),
                    reference,
                    declaration.kind,
                    referenced,
                    schema_defs,
                    records_creator_id,
                )
                .map_err(consensus_or_protocol_data_contract_error)?;
                if let Some(reason) = reason {
                    return Err(consensus_or_protocol_data_contract_error(
                        DataContractError::InvalidContractStructure(format!(
                            "document type \"{type_name}\" {} {reason}",
                            property_names::PROPERTY_CONSTRAINTS
                        )),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// How the key of an aggregate's filter, and the value it is matched against,
/// compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AggregateKeyKind {
    Integer,
    Text,
    Identifier,
}

impl AggregateKeyKind {
    /// How a property of `property_type` compares as a key or a bound value:
    /// `None` for one that cannot (a boolean, a byte array, an object, ...).
    fn of(property_type: &DocumentPropertyType) -> Option<Self> {
        match property_type {
            DocumentPropertyType::String(_) => Some(AggregateKeyKind::Text),
            property_type if property_type.is_identifier() => Some(AggregateKeyKind::Identifier),
            property_type
                if property_type.is_integer()
                    || matches!(
                        property_type,
                        DocumentPropertyType::U128 | DocumentPropertyType::I128
                    ) =>
            {
                Some(AggregateKeyKind::Integer)
            }
            _ => None,
        }
    }

    fn describe(self) -> &'static str {
        match self {
            AggregateKeyKind::Integer => "an integer",
            AggregateKeyKind::Text => "a string",
            AggregateKeyKind::Identifier => "an identifier",
        }
    }
}

/// Checks every `countOf` and `sumOf` the `propertyConstraints` and
/// `deleteConstraints` rules of `document_types`, one contract's, read, once all
/// of them are parsed: the type it totals is one of them, and not an indexOnly
/// one, nor, for a rule judging writes, the declaring type itself when it has
/// a contested index; a key of its filter is `$ownerId` or an integer, string
/// or identifier property of that type, and the value matched against it is
/// of the same kind: a property of the declaring type, `$ownerId` or `$id`, an
/// integer, a string the key's `enum` lists, or a base58 identifier; and a tree
/// of that type keeps the total ([`AggregateRead::whole_type_kept`],
/// [`AggregateRead::answering_index`]), so that a rule never reads a total a
/// count or sum tree does not keep. Registration only: the index and property
/// settings it relies on do not change on a contract update. `schema_defs`,
/// the contract's `$defs`, resolves a filter key declared through a `$ref`.
pub(in crate::data_contract::document_type::class_methods) fn validate_property_constraint_aggregates(
    document_types: &BTreeMap<String, DocumentType>,
    schema_defs: Option<&BTreeMap<String, Value>>,
) -> Result<(), DataContractError> {
    for (type_name, document_type) in document_types {
        let declaring = document_type.as_ref();
        // A delete rule judges a total as it stands at the delete, which a document
        // a contest awarded is in like any other, so it may total a contested type's
        // own documents
        let rules = declaring
            .property_constraints()
            .iter()
            .map(|rule| (property_names::PROPERTY_CONSTRAINTS, true, rule))
            .chain(
                declaring
                    .delete_constraints()
                    .iter()
                    .map(|rule| (property_names::DELETE_CONSTRAINTS, false, rule)),
            );
        for (keyword, judges_writes, (rule, constraint)) in rules {
            let error = |message: String| {
                DataContractError::InvalidContractStructure(format!(
                    "document type \"{type_name}\" {keyword} rule \"{rule}\" {message}"
                ))
            };
            for read in constraint.aggregate_reads() {
                let counted_name = &read.document_type;
                let totals = match &read.kind {
                    AggregateKind::Count => format!("counts \"{counted_name}\""),
                    AggregateKind::Sum { property } => {
                        format!("totals \"{property}\" of \"{counted_name}\"")
                    }
                };
                let Some(counted) = document_types.get(counted_name) else {
                    return Err(error(format!(
                        "{totals}, which is no document type of this contract"
                    )));
                };
                let counted = counted.as_ref();
                if counted.index_only() {
                    return Err(error(format!(
                        "{totals}, an indexOnly type, whose rows a countOf or sumOf does not \
                         total"
                    )));
                }
                // A document of the type a contest is opened for waits in the contest's
                // storage, outside the count and sum trees, and the one a contest awards
                // is stored without any rule judged, so a total of the type's own
                // documents could pass the rule
                if judges_writes
                    && read.of_own_type
                    && counted
                        .indexes()
                        .values()
                        .any(|index| index.contested_index.is_some())
                {
                    return Err(error(format!(
                        "{totals}, its own type, which has a contested index: a document a \
                         contest awards is stored without the rules being judged, so the total \
                         could pass the rule"
                    )));
                }
                if read.filter.is_empty() {
                    if !read.whole_type_kept(counted) {
                        return Err(error(match &read.kind {
                            AggregateKind::Count => format!(
                                "counts every \"{counted_name}\" document, which needs \
                                 documentsCountable on \"{counted_name}\""
                            ),
                            AggregateKind::Sum { property } => format!(
                                "totals \"{property}\" over every \"{counted_name}\" document, \
                                 which needs documentsSummable: \"{property}\" on \
                                 \"{counted_name}\""
                            ),
                        }));
                    }
                    continue;
                }
                for (key, binding) in &read.filter {
                    let key_kind = if key == OWNER_ID {
                        AggregateKeyKind::Identifier
                    } else {
                        let Some(property) = counted.flattened_properties().get(key) else {
                            return Err(error(format!(
                                "{totals} by \"{key}\", which is not a property of \
                                 \"{counted_name}\" (a nested one is named by its dotted path)"
                            )));
                        };
                        let Some(kind) = AggregateKeyKind::of(&property.property_type) else {
                            return Err(error(format!(
                                "{totals} by \"{key}\", which has type {}: a key is an integer, \
                                 string or identifier property, or $ownerId",
                                property.property_type.name()
                            )));
                        };
                        kind
                    };
                    let (bound, bound_kind) = match binding {
                        AggregateBinding::Owner => {
                            (OWNER_ID.to_string(), AggregateKeyKind::Identifier)
                        }
                        AggregateBinding::Id => (ID.to_string(), AggregateKeyKind::Identifier),
                        AggregateBinding::Integer(integer) => {
                            (integer.to_string(), AggregateKeyKind::Integer)
                        }
                        AggregateBinding::Constant(constant) => {
                            let kind = match key_kind {
                                AggregateKeyKind::Integer => {
                                    return Err(error(format!(
                                        "{totals} with \"{key}\" at the constant \
                                         \"{constant}\", but \"{key}\" is an integer: write \
                                         the integer bare"
                                    )));
                                }
                                AggregateKeyKind::Identifier => {
                                    if Identifier::from_string(constant, Encoding::Base58).is_err()
                                    {
                                        return Err(error(format!(
                                            "{totals} with \"{key}\" at \"{constant}\", which is \
                                             not a base58 identifier"
                                        )));
                                    }
                                    AggregateKeyKind::Identifier
                                }
                                AggregateKeyKind::Text => {
                                    // A constant the key's `enum` does not list is a typo:
                                    // no document could match it. `counted.schema()` is the
                                    // schema as sent, which may spell a property with a type
                                    // shorthand; only its `enum` is read here, which the
                                    // expansion never changes
                                    if !enum_admits(counted.schema(), schema_defs, key, constant)? {
                                        return Err(error(format!(
                                            "{totals} with \"{key}\" at \"{constant}\", which is \
                                             not one of its enum values"
                                        )));
                                    }
                                    AggregateKeyKind::Text
                                }
                            };
                            (format!("the constant \"{constant}\""), kind)
                        }
                        AggregateBinding::Property { path, .. } => {
                            let property_type = declaring
                                .flattened_properties()
                                .get(path)
                                .map(|property| &property.property_type);
                            let Some(kind) = property_type.and_then(AggregateKeyKind::of) else {
                                return Err(error(format!(
                                    "{totals} with \"{key}\" at \"{path}\", which has type {}: \
                                     a value matched is an integer, string or identifier \
                                     property, $ownerId, $id, an integer or a \
                                     {{ \"const\": ... }}",
                                    property_type.map_or("none".to_string(), |property_type| {
                                        property_type.name()
                                    })
                                )));
                            };
                            (format!("\"{path}\""), kind)
                        }
                    };
                    if bound_kind != key_kind {
                        return Err(error(format!(
                            "{totals} with \"{key}\", {}, at {bound}, {}",
                            key_kind.describe(),
                            bound_kind.describe()
                        )));
                    }
                }
                if read.answering_index(&counted).is_none() {
                    let keys = read
                        .filter
                        .keys()
                        .map(|key| format!("\"{key}\""))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let keeping = match &read.kind {
                        AggregateKind::Count => "countable index".to_string(),
                        AggregateKind::Sum { property } => {
                            format!("index with summable: \"{property}\"")
                        }
                    };
                    return Err(error(format!(
                        "{totals} by {keys}, which no {keeping} of \"{counted_name}\" whose \
                         properties are exactly those keys answers (a unique, contested, ranked, \
                         time-range or indexOnly-terminal index answers none)"
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Why a string constant, affix or default `rule` (described by `subject`)
/// compares with a property of `schema` is none its `enum` admits, `None` when
/// each is: a constant or a default the `enum` does not list, or an affix none
/// of its values fits, is a typo the property could never meet. `lookup`
/// gives the property of `schema` a rule path reads, `None` for one it does
/// not check here, and `label` names a rule path in the message.
fn enum_miss(
    rule: &PropertyConstraint,
    subject: &str,
    schema: &Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    lookup: &dyn Fn(&str) -> Result<Option<String>, DataContractError>,
    label: &dyn Fn(&str) -> String,
) -> Result<Option<String>, DataContractError> {
    for (path, constant) in rule.text_constants() {
        let Some(property) = lookup(path)? else {
            continue;
        };
        if !enum_admits(schema, schema_defs, &property, constant)? {
            return Ok(Some(format!(
                "{subject} compares \"{}\" with \"{constant}\", which is not one of its enum \
                 values",
                label(path)
            )));
        }
    }
    for (path, affix, position) in rule.text_affixes() {
        let Some(property) = lookup(path)? else {
            continue;
        };
        if !enum_any(schema, schema_defs, &property, |member| {
            position.holds(member, affix)
        })? {
            let tests = match position {
                AffixPosition::Start => "starts with",
                AffixPosition::End => "ends with",
            };
            return Ok(Some(format!(
                "{subject} tests whether \"{}\" {tests} \"{affix}\", which none of its enum \
                 values does",
                label(path)
            )));
        }
    }
    for (path, default) in rule.text_defaults() {
        let Some(property) = lookup(path)? else {
            continue;
        };
        if !enum_admits(schema, schema_defs, &property, default)? {
            return Ok(Some(format!(
                "{subject} gives \"{}\" the default \"{default}\", which is not one of its \
                 enum values",
                label(path)
            )));
        }
    }
    Ok(None)
}

/// Whether the string property at the dotted `path` of `schema`, a document
/// type's, may hold `value`: always, unless it declares an `enum` that does not
/// list it. `$ref`s are followed into `schema_defs`, the contract's `$defs`.
fn enum_admits(
    schema: &Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    path: &str,
    value: &str,
) -> Result<bool, DataContractError> {
    enum_any(schema, schema_defs, path, |member| member == value)
}

/// Whether the string property at the dotted `path` of `schema` (or the
/// elements of the typed array there) may hold a value `admits`: always,
/// unless it declares an `enum`, one of whose values must then pass. `$ref`s
/// are followed into `schema_defs`, the contract's `$defs`.
fn enum_any(
    schema: &Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    path: &str,
    admits: impl Fn(&str) -> bool,
) -> Result<bool, DataContractError> {
    let Some(property_schema) = schema_at_path(schema, schema_defs, path)? else {
        return Ok(true);
    };
    // A value a `contains` looks for in an array is one of its elements
    let property_schema = match property_schema.get(property_names::ITEMS) {
        Some(items) => resolve_schema(schema, schema_defs, items)?,
        None => property_schema,
    };
    let Some(Value::Array(members)) = property_schema.get(property_names::ENUM) else {
        return Ok(true);
    };
    Ok(members
        .iter()
        .any(|member| member.as_text().is_some_and(&admits)))
}

/// The schema of the property at the dotted `path` of `schema`, a document
/// type's, `None` when the path names none. `$ref`s are followed, into
/// `schema_defs`, the contract's `$defs`, for `#/$defs/...`.
fn schema_at_path<'a>(
    schema: &'a Value,
    schema_defs: Option<&'a BTreeMap<String, Value>>,
    path: &str,
) -> Result<Option<BTreeMap<String, &'a Value>>, DataContractError> {
    let mut current = resolve_schema(schema, schema_defs, schema)?;
    for segment in path.split('.') {
        let Some(properties) = current.get(property_names::PROPERTIES) else {
            return Ok(None);
        };
        let Some(next) = properties.to_btree_ref_string_map()?.get(segment).copied() else {
            return Ok(None);
        };
        current = resolve_schema(schema, schema_defs, next)?;
    }
    Ok(Some(current))
}

/// The schema `value` is within the document type schema `schema`, its `$ref`
/// followed ([`resolve_schema_ref`]).
fn resolve_schema<'a>(
    schema: &'a Value,
    schema_defs: Option<&'a BTreeMap<String, Value>>,
    value: &'a Value,
) -> Result<BTreeMap<String, &'a Value>, DataContractError> {
    let map = value.to_btree_ref_string_map()?;
    match map.get_optional_str(property_names::REF)? {
        Some(schema_ref) => {
            Ok(resolve_schema_ref(schema, schema_defs, schema_ref)?.to_btree_ref_string_map()?)
        }
        None => Ok(map),
    }
}

/// The value the `$ref` `uri` of the document type schema `schema` names,
/// found where the core parse finds it, in the schema with the contract's
/// `$defs` added ([`DocumentType::enrich_with_base_schema`]): a
/// `#/$defs/<name>` reference, and a path below one, in `schema_defs`; any
/// other in `schema`, which holds no `$defs` of its own. Every document
/// meta-schema names a definition with letters, digits, `-` and `_` only, so
/// the name ends at the next `/`.
fn resolve_schema_ref<'a>(
    schema: &'a Value,
    schema_defs: Option<&'a BTreeMap<String, Value>>,
    uri: &str,
) -> Result<&'a Value, DataContractError> {
    let Some(definition_path) = uri.strip_prefix(DEFINITIONS_REF_PREFIX) else {
        return resolve_uri(schema, uri);
    };
    let (name, below) = match definition_path.split_once('/') {
        Some((name, below)) => (name, Some(below)),
        None => (definition_path, None),
    };
    let definition = schema_defs
        .and_then(|definitions| definitions.get(name))
        .ok_or_else(|| {
            DataContractError::InvalidURI(format!(
                "{uri} names no definition in the contract's $defs"
            ))
        })?;
    match below {
        Some(below) => resolve_uri(definition, &format!("#/{below}")),
        None => Ok(definition),
    }
}

/// Whether the property at the dotted `path` of `schema` is declared as an
/// integer with `minimum` at least 0 and `maximum` at most `u32::MAX`, read
/// from the schema rather than from the parsed type so that the answer does
/// not depend on the contract's `sizedIntegerTypes`. `$ref`s are followed into
/// `schema_defs`, the contract's `$defs`.
fn is_key_id_schema(
    schema: &Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    path: &str,
) -> Result<bool, DataContractError> {
    let Some(current) = schema_at_path(schema, schema_defs, path)? else {
        return Ok(false);
    };
    let is_integer = current.get_optional_str(property_names::TYPE)? == Some("integer");
    let minimum = current.get_optional_integer::<i64>(property_names::MINIMUM)?;
    let maximum = current.get_optional_integer::<i64>(property_names::MAXIMUM)?;
    Ok(is_integer
        && minimum.is_some_and(|minimum| minimum >= 0)
        && maximum.is_some_and(|maximum| maximum <= i64::from(u32::MAX)))
}

/// The `contractRequirements` of a `contract` reference: each key an aspect of the referenced
/// contract with a closed set of values (`moderation`, `owner`, the config flags), or a bound
/// on it (`minimumAgeSeconds`), at least one when the object is given at all.
fn parse_contract_reference_requirements(
    refers_to_map: &BTreeMap<String, &Value>,
) -> Result<ContractReferenceRequirements, DataContractError> {
    let Some(fields_value) = refers_to_map.get(property_names::CONTRACT_REQUIREMENTS) else {
        return Ok(ContractReferenceRequirements::default());
    };
    let fields_map = fields_value.to_btree_ref_string_map()?;
    if fields_map.is_empty() {
        return Err(DataContractError::InvalidContractStructure(
            "contract refersTo contractRequirements must declare at least one requirement"
                .to_string(),
        ));
    }
    let mut fields = ContractReferenceRequirements::default();
    for (field, value) in fields_map {
        match field.as_str() {
            property_names::MODERATION => {
                let name = value.as_text().ok_or_else(|| {
                    DataContractError::InvalidContractStructure(
                        "contract refersTo contractRequirements moderation must be a string"
                            .to_string(),
                    )
                })?;
                fields.moderation = Some(ContractReferenceModeration::from_wire_name(name).ok_or_else(|| {
                    DataContractError::InvalidContractStructure(format!(
                        "contract refersTo contractRequirements moderation {name:?} is unknown, expected one of {:?}",
                        ContractReferenceModeration::WIRE_NAMES
                    ))
                })?);
            }
            property_names::MINIMUM_AGE_SECONDS => {
                fields.minimum_age_seconds = Some(parse_contract_reference_seconds(&field, value)?);
            }
            property_names::MINIMUM_SECONDS_SINCE_UPDATE => {
                fields.minimum_seconds_since_update =
                    Some(parse_contract_reference_seconds(&field, value)?);
            }
            property_names::OWNER => {
                let name = value.as_text().ok_or_else(|| {
                    DataContractError::InvalidContractStructure(
                        "contract refersTo contractRequirements owner must be a string".to_string(),
                    )
                })?;
                fields.owner = Some(ContractReferenceOwner::from_wire_name(name).ok_or_else(|| {
                    DataContractError::InvalidContractStructure(format!(
                        "contract refersTo contractRequirements owner {name:?} is unknown, expected \"self\" or \"other\""
                    ))
                })?);
            }
            property_names::READONLY => {
                fields.readonly = Some(parse_contract_reference_true(&field, value)?);
            }
            property_names::KEEPS_HISTORY => {
                fields.keeps_history = Some(parse_contract_reference_true(&field, value)?);
            }
            property_names::OWNER_PROTECTED => {
                fields.owner_protected = Some(parse_contract_reference_bool(&field, value)?);
            }
            other => {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "contract refersTo contractRequirements {other:?} is unknown"
                )));
            }
        }
    }
    Ok(fields)
}

/// The `keyRequirements` of an `identityPublicKey` reference: each key an aspect of the
/// referenced key with a closed set of values (`purpose`) or a document type name of the
/// declaring contract (`boundTo`), at least one when the object is given at all. That `boundTo`
/// names a document type the contract has is checked once every document type is parsed, in
/// `create_document_types_from_document_schemas`.
fn parse_identity_key_reference_requirements(
    refers_to_map: &BTreeMap<String, &Value>,
) -> Result<IdentityKeyReferenceRequirements, DataContractError> {
    let Some(fields_value) = refers_to_map.get(property_names::KEY_REQUIREMENTS) else {
        return Ok(IdentityKeyReferenceRequirements::default());
    };
    let fields_map = fields_value.to_btree_ref_string_map()?;
    if fields_map.is_empty() {
        return Err(DataContractError::InvalidContractStructure(
            "identityPublicKey refersTo keyRequirements must declare at least one requirement"
                .to_string(),
        ));
    }
    let mut fields = IdentityKeyReferenceRequirements::default();
    for (field, value) in fields_map {
        match field.as_str() {
            property_names::PURPOSE => {
                let name = value.as_text().ok_or_else(|| {
                    DataContractError::InvalidContractStructure(
                        "identityPublicKey refersTo keyRequirements purpose must be a string"
                            .to_string(),
                    )
                })?;
                // The purposes a user's key can carry: every one but SYSTEM
                let purpose = Purpose::from_wire_name(name)
                    .filter(|purpose| Purpose::full_range().contains(purpose))
                    .ok_or_else(|| {
                        DataContractError::InvalidContractStructure(format!(
                            "identityPublicKey refersTo keyRequirements purpose {name:?} is unknown, expected one of {:?}",
                            Purpose::full_range().map(|purpose| purpose.wire_name())
                        ))
                    })?;
                fields.purpose = Some(purpose);
            }
            property_names::BOUND_TO => {
                let document_type_name = value.as_text().ok_or_else(|| {
                    DataContractError::InvalidContractStructure(
                        "identityPublicKey refersTo keyRequirements boundTo must be a string"
                            .to_string(),
                    )
                })?;
                if document_type_name.is_empty() || document_type_name.len() > 64 {
                    return Err(DataContractError::InvalidContractStructure(
                        "identityPublicKey refersTo keyRequirements boundTo must be between 1 \
                         and 64 characters"
                            .to_string(),
                    ));
                }
                fields.bound_to = Some(document_type_name.to_string());
            }
            other => {
                return Err(DataContractError::InvalidContractStructure(format!(
                    "identityPublicKey refersTo keyRequirements {other:?} is unknown"
                )));
            }
        }
    }
    Ok(fields)
}

/// A duration requirement of a `contract` reference (`minimumAgeSeconds`,
/// `minimumSecondsSinceUpdate`): a whole number of seconds from 1 to `u32::MAX`.
fn parse_contract_reference_seconds(field: &str, value: &Value) -> Result<u32, DataContractError> {
    let seconds: u32 = value.to_integer().map_err(|_| {
        DataContractError::InvalidContractStructure(format!(
            "contract refersTo contractRequirements {field} must be an integer from 1 to 4294967295"
        ))
    })?;
    if seconds == 0 {
        return Err(DataContractError::InvalidContractStructure(format!(
            "contract refersTo contractRequirements {field} must be at least 1"
        )));
    }
    Ok(seconds)
}

/// A boolean requirement of a `contract` reference (`ownerProtected`): `true` or `false`.
fn parse_contract_reference_bool(field: &str, value: &Value) -> Result<bool, DataContractError> {
    value.as_bool().ok_or_else(|| {
        DataContractError::InvalidContractStructure(format!(
            "contract refersTo contractRequirements {field} must be a boolean"
        ))
    })
}

/// A flag requirement of a `contract` reference (`readonly`, `keepsHistory`): only `true`
/// requires anything, so `false` is refused rather than declared as a requirement that
/// requires nothing.
fn parse_contract_reference_true(field: &str, value: &Value) -> Result<bool, DataContractError> {
    if parse_contract_reference_bool(field, value)? {
        Ok(true)
    } else {
        Err(DataContractError::InvalidContractStructure(format!(
            "contract refersTo contractRequirements {field} must be true"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use crate::data_contract::document_type::index::{Index, IntegerRangeKeyType};
    use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
    use crate::data_contract::document_type::validate_required_since_within_contract_version;
    use crate::data_contract::DataContract;
    use crate::serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
        PlatformSerializableWithPlatformVersion,
    };
    use assert_matches::assert_matches;
    use platform_value::string_encoding::Encoding;
    use serde_json::json;

    fn try_document_type_from_schema(
        schema: serde_json::Value,
    ) -> Result<DocumentType, ProtocolError> {
        try_document_type_from_schema_on_version(schema, PlatformVersion::latest())
    }

    /// Same as [`try_document_type_from_schema`] but with `full_validation`
    /// on — the index validations (the timeRange source rules among them)
    /// only run on the validating parse.
    fn try_document_type_from_schema_full_validation(
        schema: serde_json::Value,
    ) -> Result<DocumentType, ProtocolError> {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let value = platform_value::to_value(schema).expect("schema should convert");

        DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            platform_version,
        )
    }

    fn try_document_type_from_schema_on_version(
        schema: serde_json::Value,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentType, ProtocolError> {
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let value = platform_value::to_value(schema).expect("schema should convert");

        DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
    }

    #[test]
    fn should_parse_refers_to_on_identifier_property() {
        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "toUserId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "identity"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect("should parse");

        let property_type = document_type
            .as_ref()
            .flattened_properties()
            .get("toUserId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        assert!(matches!(
            property_type,
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Identity
            )
        ));
    }

    #[test]
    fn should_reject_refers_to_on_non_identifier_property() {
        let err = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0,
                    "refersTo": { "type": "identity" }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("refersTo is only allowed on identifier properties"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn should_parse_property_agreement_on_permanent_document_refers_to() {
        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "hashtag": { "type": "string", "position": 0, "maxLength": 63 },
                "postId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 1,
                    "refersTo": {
                        "type": "permanentDocument",
                        "documentType": "post",
                        "where": {
                            "hashtag": "hashtag"
                        }
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect("should parse");

        let property_type = document_type
            .as_ref()
            .flattened_properties()
            .get("postId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        let DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::PermanentDocument {
                property_agreement, ..
            },
        ) = property_type
        else {
            panic!("expected a permanentDocument reference");
        };
        assert_eq!(
            property_agreement,
            BTreeMap::from([("hashtag".to_string(), "hashtag".to_string())])
        );
    }

    #[test]
    fn should_reject_property_agreement_on_non_document_reference() {
        let err = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "toUserId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "identity",
                        "where": {
                            "hashtag": "hashtag"
                        }
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("identity refersTo does not take where"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn should_reject_empty_property_agreement() {
        let err = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "postId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "documentType": "post",
                        "where": {}
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("where must compare between 1 and 10 properties"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn should_reject_non_string_property_agreement_values() {
        let err = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "postId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "documentType": "post",
                        "where": { "hashtag": 7 }
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("where values must name a property of the referring document type"),
            "unexpected error: {message}"
        );
    }

    fn system_agreement_schema(agreement: serde_json::Value) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "authorId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0
                },
                "postId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 1,
                    "refersTo": {
                        "type": "permanentDocument",
                        "documentType": "post",
                        "where": agreement
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        })
    }

    #[test]
    fn should_parse_referenced_system_identifiers_in_property_agreement() {
        for referenced in ["$ownerId", "$creatorId"] {
            let document_type = try_document_type_from_schema(system_agreement_schema(json!({
                referenced: "authorId"
            })))
            .expect("a referenced-side system identifier should parse");

            let property_type = document_type
                .as_ref()
                .flattened_properties()
                .get("postId")
                .map(|p| p.property_type.clone())
                .expect("property should be present");

            let DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::PermanentDocument {
                    property_agreement, ..
                },
            ) = property_type
            else {
                panic!("expected a permanentDocument reference");
            };
            assert_eq!(
                property_agreement,
                BTreeMap::from([("authorId".to_string(), referenced.to_string())])
            );
        }
    }

    #[test]
    fn should_reject_other_system_properties_on_the_referring_side_of_an_agreement() {
        for referring in ["$creatorId", "$id", "$createdAt"] {
            let err = try_document_type_from_schema(system_agreement_schema(json!({
                "$ownerId": referring
            })))
            .expect_err("only $ownerId may be the referring side");

            let message = err.to_string();
            assert!(
                message.contains("referring document type or its $ownerId"),
                "unexpected error for {referring}: {message}"
            );
        }
    }

    #[test]
    fn should_parse_writer_owner_id_on_the_referring_side_of_an_agreement() {
        for referenced in ["$ownerId", "$creatorId", "authorId"] {
            let document_type = try_document_type_from_schema(system_agreement_schema(json!({
                referenced: "$ownerId"
            })))
            .expect("the writer's $ownerId should parse as the referring side");

            let property_type = document_type
                .as_ref()
                .flattened_properties()
                .get("postId")
                .map(|p| p.property_type.clone())
                .expect("property should be present");

            let DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::PermanentDocument {
                    property_agreement, ..
                },
            ) = property_type
            else {
                panic!("expected a permanentDocument reference");
            };
            assert_eq!(
                property_agreement,
                BTreeMap::from([("$ownerId".to_string(), referenced.to_string())])
            );
        }
    }

    #[test]
    fn should_reject_other_system_properties_on_the_referenced_side_of_an_agreement() {
        for referenced in ["$createdAt", "$revision", "$owner"] {
            let err = try_document_type_from_schema(system_agreement_schema(json!({
                referenced: "authorId"
            })))
            .expect_err("only $ownerId, $creatorId and $id may be referenced");

            let message = err.to_string();
            assert!(
                message.contains("$ownerId, $creatorId and $id system properties"),
                "unexpected error for {referenced}: {message}"
            );
        }
        // `$id`, the referenced document's own id, is one of them
        try_document_type_from_schema(system_agreement_schema(json!({ "$id": "authorId" })))
            .expect("$id may be referenced");
    }

    #[test]
    fn should_parse_permanent_document_refers_to() {
        let contract_id = Identifier::from([7u8; 32]);

        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "parentNoteId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "contractId": contract_id.to_string(Encoding::Base58),
                        "documentType": "note"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect("should parse");

        let property_type = document_type
            .as_ref()
            .flattened_properties()
            .get("parentNoteId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        assert_eq!(
            property_type,
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id: Some(contract_id),
                    document_type_name: "note".to_string(),
                    where_conditions: Default::default(),
                    property_agreement: Default::default(),
                }
            )
        );
    }

    #[test]
    fn should_parse_permanent_document_refers_to_without_contract_id_as_own_contract() {
        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "parentNoteId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "documentType": "note"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect("should parse");

        let property_type = document_type
            .as_ref()
            .flattened_properties()
            .get("parentNoteId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        assert_eq!(
            property_type,
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id: None,
                    document_type_name: "note".to_string(),
                    where_conditions: Default::default(),
                    property_agreement: Default::default(),
                }
            )
        );
    }

    fn contract_reference_schema(refers_to: serde_json::Value) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "targetContractId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": refers_to
                }
            },
            "required": [],
            "additionalProperties": false
        })
    }

    fn contract_reference_target(refers_to: serde_json::Value) -> DocumentPropertyType {
        try_document_type_from_schema(contract_reference_schema(refers_to))
            .expect("should parse")
            .as_ref()
            .flattened_properties()
            .get("targetContractId")
            .map(|p| p.property_type.clone())
            .expect("property should be present")
    }

    #[test]
    fn should_parse_contract_refers_to_without_contract_requirements_as_no_requirement() {
        assert_eq!(
            contract_reference_target(json!({ "type": "contract" })),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Contract {
                    contract_requirements: ContractReferenceRequirements::default(),
                }
            )
        );
    }

    #[test]
    fn should_parse_contract_refers_to_requiring_elected_moderation() {
        assert_eq!(
            contract_reference_target(json!({
                "type": "contract",
                "contractRequirements": { "moderation": "elected" }
            })),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Contract {
                    contract_requirements: ContractReferenceRequirements {
                        moderation: Some(ContractReferenceModeration::Elected),
                        minimum_age_seconds: None,
                        minimum_seconds_since_update: None,
                        owner: None,
                        readonly: None,
                        keeps_history: None,
                        owner_protected: None,
                    },
                }
            )
        );
    }

    #[test]
    fn should_parse_contract_refers_to_requiring_the_moderation_election_open() {
        assert_eq!(
            contract_reference_target(json!({
                "type": "contract",
                "contractRequirements": { "moderation": "electionOpen" }
            })),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Contract {
                    contract_requirements: ContractReferenceRequirements {
                        moderation: Some(ContractReferenceModeration::ElectionOpen),
                        minimum_age_seconds: None,
                        minimum_seconds_since_update: None,
                        owner: None,
                        readonly: None,
                        keeps_history: None,
                        owner_protected: None,
                    },
                }
            )
        );
    }

    #[test]
    fn should_parse_contract_refers_to_requiring_a_minimum_age_or_time_since_update() {
        assert_eq!(
            contract_reference_target(json!({
                "type": "contract",
                "contractRequirements": { "minimumAgeSeconds": 604800 }
            })),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Contract {
                    contract_requirements: ContractReferenceRequirements {
                        moderation: None,
                        minimum_age_seconds: Some(604_800),
                        minimum_seconds_since_update: None,
                        owner: None,
                        readonly: None,
                        keeps_history: None,
                        owner_protected: None,
                    },
                }
            )
        );
        assert_eq!(
            contract_reference_target(json!({
                "type": "contract",
                "contractRequirements": { "minimumSecondsSinceUpdate": 86400 }
            })),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Contract {
                    contract_requirements: ContractReferenceRequirements {
                        moderation: None,
                        minimum_age_seconds: None,
                        minimum_seconds_since_update: Some(86_400),
                        owner: None,
                        readonly: None,
                        keeps_history: None,
                        owner_protected: None,
                    },
                }
            )
        );
        assert_eq!(
            contract_reference_target(json!({
                "type": "contract",
                "contractRequirements": {
                    "moderation": "elected",
                    "minimumAgeSeconds": u32::MAX,
                    "minimumSecondsSinceUpdate": 1
                }
            })),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Contract {
                    contract_requirements: ContractReferenceRequirements {
                        moderation: Some(ContractReferenceModeration::Elected),
                        minimum_age_seconds: Some(u32::MAX),
                        minimum_seconds_since_update: Some(1),
                        owner: None,
                        readonly: None,
                        keeps_history: None,
                        owner_protected: None,
                    },
                }
            )
        );
    }

    #[test]
    fn should_parse_contract_refers_to_requiring_an_owner_relation() {
        for (name, owner) in [
            ("self", ContractReferenceOwner::Writer),
            ("other", ContractReferenceOwner::Other),
        ] {
            assert_eq!(
                contract_reference_target(json!({
                    "type": "contract",
                    "contractRequirements": { "owner": name }
                })),
                DocumentPropertyType::IdentifierWithReference(
                    DocumentPropertyReferenceTarget::Contract {
                        contract_requirements: ContractReferenceRequirements {
                            owner: Some(owner),
                            ..Default::default()
                        },
                    }
                )
            );
        }
        assert_eq!(
            contract_reference_target(json!({
                "type": "contract",
                "contractRequirements": { "moderation": "elected", "owner": "other" }
            })),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Contract {
                    contract_requirements: ContractReferenceRequirements {
                        moderation: Some(ContractReferenceModeration::Elected),
                        owner: Some(ContractReferenceOwner::Other),
                        ..Default::default()
                    },
                }
            )
        );
    }

    #[test]
    fn should_parse_contract_refers_to_requiring_config_flags() {
        for (requirements, expected) in [
            (
                json!({ "readonly": true }),
                ContractReferenceRequirements {
                    readonly: Some(true),
                    ..Default::default()
                },
            ),
            (
                json!({ "keepsHistory": true }),
                ContractReferenceRequirements {
                    keeps_history: Some(true),
                    ..Default::default()
                },
            ),
            (
                json!({ "ownerProtected": true }),
                ContractReferenceRequirements {
                    owner_protected: Some(true),
                    ..Default::default()
                },
            ),
            (
                json!({ "ownerProtected": false }),
                ContractReferenceRequirements {
                    owner_protected: Some(false),
                    ..Default::default()
                },
            ),
            (
                json!({ "moderation": "elected", "readonly": true, "keepsHistory": true, "ownerProtected": false }),
                ContractReferenceRequirements {
                    moderation: Some(ContractReferenceModeration::Elected),
                    readonly: Some(true),
                    keeps_history: Some(true),
                    owner_protected: Some(false),
                    ..Default::default()
                },
            ),
        ] {
            assert_eq!(
                contract_reference_target(json!({
                    "type": "contract",
                    "contractRequirements": requirements
                })),
                DocumentPropertyType::IdentifierWithReference(
                    DocumentPropertyReferenceTarget::Contract {
                        contract_requirements: expected,
                    }
                ),
                "{requirements}"
            );
        }
    }

    #[test]
    fn should_reject_a_config_flag_requirement_that_is_not_a_boolean_or_requires_nothing() {
        for (field, value, fragment) in [
            ("readonly", json!(false), "readonly must be true"),
            ("readonly", json!("true"), "readonly must be a boolean"),
            ("readonly", json!(1), "readonly must be a boolean"),
            ("keepsHistory", json!(false), "keepsHistory must be true"),
            (
                "keepsHistory",
                json!("true"),
                "keepsHistory must be a boolean",
            ),
            (
                "ownerProtected",
                json!("true"),
                "ownerProtected must be a boolean",
            ),
            (
                "ownerProtected",
                json!(0),
                "ownerProtected must be a boolean",
            ),
            (
                "ownerProtected",
                json!(null),
                "ownerProtected must be a boolean",
            ),
        ] {
            let refers_to = json!({
                "type": "contract",
                "contractRequirements": { field: value }
            });
            let err = try_document_type_from_schema(contract_reference_schema(refers_to.clone()))
                .expect_err("should be refused");
            assert!(
                err.to_string().contains(fragment),
                "{refers_to}: expected {fragment:?}, got {err}"
            );
        }
    }

    #[test]
    fn should_reject_a_duration_requirement_that_is_zero_negative_too_large_or_not_an_integer() {
        for field in ["minimumAgeSeconds", "minimumSecondsSinceUpdate"] {
            for (seconds, fragment) in [
                (json!(0), "must be at least 1"),
                (json!(-1), "must be an integer"),
                (json!(u64::from(u32::MAX) + 1), "must be an integer"),
                (json!(1.5), "must be an integer"),
                (json!("3600"), "must be an integer"),
            ] {
                let refers_to = json!({
                    "type": "contract",
                    "contractRequirements": { field: seconds }
                });
                let err =
                    try_document_type_from_schema(contract_reference_schema(refers_to.clone()))
                        .expect_err("should be refused");
                assert!(
                    err.to_string().contains(fragment) && err.to_string().contains(field),
                    "{refers_to}: expected {fragment:?} naming {field}, got {err}"
                );
            }
        }
    }

    #[test]
    fn should_reject_contract_requirements_that_are_empty_unknown_or_on_another_type() {
        for (refers_to, fragment) in [
            (
                json!({ "type": "contract", "contractRequirements": {} }),
                "at least one requirement",
            ),
            (
                json!({ "type": "contract", "contractRequirements": { "moderation": "appointed" } }),
                "is unknown",
            ),
            (
                json!({ "type": "contract", "contractRequirements": { "moderation": 1 } }),
                "must be a string",
            ),
            (
                json!({ "type": "contract", "contractRequirements": { "tokens": "any" } }),
                "is unknown",
            ),
            (
                json!({ "type": "contract", "contractRequirements": { "owner": "anyone" } }),
                "owner \"anyone\" is unknown, expected \"self\" or \"other\"",
            ),
            (
                json!({ "type": "contract", "contractRequirements": { "owner": true } }),
                "owner must be a string",
            ),
            (
                json!({ "type": "identity", "contractRequirements": { "moderation": "elected" } }),
                "does not take contractRequirements",
            ),
        ] {
            let err = try_document_type_from_schema(contract_reference_schema(refers_to.clone()))
                .expect_err("should be refused");
            assert!(
                err.to_string().contains(fragment),
                "{refers_to}: expected {fragment:?}, got {err}"
            );
        }
    }

    fn contract_identifier(refers_to: serde_json::Value) -> serde_json::Value {
        json!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "refersTo": refers_to
        })
    }

    /// A mutable document type holding `reference` (a single identifier, an object or a typed
    /// array) at `link`, with `owner_change` merged in (`transferable`, `tradeMode`) and
    /// `link` listed as immutable when `immutable` is set
    fn contract_reference_holder_schema(
        owner_change: serde_json::Value,
        mut reference: serde_json::Value,
        immutable: bool,
    ) -> serde_json::Value {
        reference["position"] = json!(0);
        let mut schema = json!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "link": reference,
                "note": { "type": "string", "maxLength": 64, "position": 1 }
            },
            "required": [],
            "additionalProperties": false
        });
        if immutable {
            schema["immutable"] = json!(["link"]);
        }
        if let (Some(schema), Some(owner_change)) =
            (schema.as_object_mut(), owner_change.as_object())
        {
            schema.extend(owner_change.clone());
        }
        schema
    }

    fn owner_requirement(owner: &str) -> serde_json::Value {
        json!({ "type": "contract", "contractRequirements": { "owner": owner } })
    }

    #[test]
    fn should_refuse_an_immutable_contract_owner_requirement_on_a_type_whose_documents_change_owner(
    ) {
        for owner_change in [json!({ "transferable": 1 }), json!({ "tradeMode": 1 })] {
            for owner in ["self", "other"] {
                let schema = contract_reference_holder_schema(
                    owner_change.clone(),
                    contract_identifier(owner_requirement(owner)),
                    true,
                );
                let err = try_document_type_from_schema_full_validation(schema.clone())
                    .expect_err("should be refused");
                assert!(
                    err.to_string()
                        .contains("is a contract reference with an `owner` requirement"),
                    "{owner_change} {owner}: got {err}"
                );
                // A stored contract is parsed without the registration rules and stays readable
                try_document_type_from_schema(schema).expect("should parse without validation");
            }
        }
    }

    #[test]
    fn should_refuse_a_contract_owner_requirement_inside_an_immutable_object_or_typed_array() {
        let in_object = json!({
            "type": "object",
            "properties": {
                "contractId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": owner_requirement("self")
                }
            },
            "required": [],
            "additionalProperties": false
        });
        let in_typed_array = json!({
            "type": "array",
            "minItems": 0,
            "maxItems": 4,
            "items": contract_identifier(owner_requirement("self"))
        });
        for (reference, path) in [(in_object, "link.contractId"), (in_typed_array, "link")] {
            let err = try_document_type_from_schema_full_validation(
                contract_reference_holder_schema(json!({ "transferable": 1 }), reference, true),
            )
            .expect_err("should be refused");
            assert!(
                err.to_string().contains(&format!(
                    "\"{path}\" is a contract reference with an `owner` requirement"
                )),
                "{path}: got {err}"
            );
        }
    }

    #[test]
    fn should_admit_an_immutable_contract_owner_requirement_where_documents_cannot_change_owner() {
        for owner in ["self", "other"] {
            try_document_type_from_schema_full_validation(contract_reference_holder_schema(
                json!({}),
                contract_identifier(owner_requirement(owner)),
                true,
            ))
            .expect("the writer never changes, so the requirement is never re-checked");
        }
    }

    #[test]
    fn should_admit_a_mutable_owner_requirement_and_other_immutable_contract_requirements() {
        // Mutable: a new owner can repoint the reference at a contract that meets it
        try_document_type_from_schema_full_validation(contract_reference_holder_schema(
            json!({ "transferable": 1 }),
            contract_identifier(owner_requirement("self")),
            false,
        ))
        .expect("a mutable reference can be repointed");
        // The other requirements are facts about the referenced contract, never re-checked
        try_document_type_from_schema_full_validation(contract_reference_holder_schema(
            json!({ "transferable": 1 }),
            contract_identifier(
                json!({ "type": "contract", "contractRequirements": { "readonly": true } }),
            ),
            true,
        ))
        .expect("only the owner requirement is re-checked on replace");
    }

    #[test]
    fn should_reject_permanent_document_refers_to_with_invalid_contract_id() {
        let err = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "parentNoteId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "contractId": "not-a-valid-identifier",
                        "documentType": "note"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(message.contains("base 58"), "unexpected error: {message}");
    }

    #[test]
    fn should_reject_permanent_document_refers_to_with_oversized_document_type_name() {
        let err = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "parentNoteId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "contractId": Identifier::from([7u8; 32]).to_string(Encoding::Base58),
                        "documentType": "a".repeat(65)
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("between 1 and 64 characters"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn should_reject_permanent_document_refers_to_without_document_type() {
        try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "parentNoteId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "permanentDocument",
                        "contractId": Identifier::from([7u8; 32]).to_string(Encoding::Base58)
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");
    }

    #[test]
    fn should_parse_deletable_document_refers_to() {
        let contract_id = Identifier::from([7u8; 32]);
        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "draftId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "deletableDocument",
                        "contractId": contract_id.to_string(Encoding::Base58),
                        "documentType": "draft",
                        "where": { "topic": "topic", "$ownerId": "$ownerId" }
                    }
                },
                "ownDraftId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 1,
                    "refersTo": {
                        "type": "deletableDocument",
                        "documentType": "draft"
                    }
                },
                "topic": {
                    "type": "string",
                    "maxLength": 63,
                    "position": 2
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect("should parse");

        let property_type = |name: &str| {
            document_type
                .as_ref()
                .flattened_properties()
                .get(name)
                .map(|p| p.property_type.clone())
                .expect("property should be present")
        };

        assert_eq!(
            property_type("draftId"),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::DeletableDocument {
                    contract_id: Some(contract_id),
                    document_type_name: "draft".to_string(),
                    where_conditions: Default::default(),
                    property_agreement: [
                        ("topic".to_string(), "topic".to_string()),
                        ("$ownerId".to_string(), "$ownerId".to_string()),
                    ]
                    .into(),
                }
            )
        );
        assert_eq!(
            property_type("ownDraftId"),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::DeletableDocument {
                    contract_id: None,
                    document_type_name: "draft".to_string(),
                    where_conditions: Default::default(),
                    property_agreement: Default::default(),
                }
            )
        );
    }

    #[test]
    fn should_parse_moderated_document_refers_to() {
        let contract_id = Identifier::from([7u8; 32]);
        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "postId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "moderatedDocument",
                        "contractId": contract_id.to_string(Encoding::Base58),
                        "documentType": "post",
                        "where": { "topic": "topic", "$ownerId": "$ownerId" }
                    }
                },
                "ownPostId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 1,
                    "refersTo": {
                        "type": "moderatedDocument",
                        "documentType": "post"
                    }
                },
                "topic": {
                    "type": "string",
                    "maxLength": 63,
                    "position": 2
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect("should parse");

        let property_type = |name: &str| {
            document_type
                .as_ref()
                .flattened_properties()
                .get(name)
                .map(|p| p.property_type.clone())
                .expect("property should be present")
        };

        assert_eq!(
            property_type("postId"),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::ModeratedDocument {
                    contract_id: Some(contract_id),
                    document_type_name: "post".to_string(),
                    where_conditions: Default::default(),
                    property_agreement: [
                        ("topic".to_string(), "topic".to_string()),
                        ("$ownerId".to_string(), "$ownerId".to_string()),
                    ]
                    .into(),
                }
            )
        );
        assert_eq!(
            property_type("ownPostId"),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::ModeratedDocument {
                    contract_id: None,
                    document_type_name: "post".to_string(),
                    where_conditions: Default::default(),
                    property_agreement: Default::default(),
                }
            )
        );
    }

    /// A moderated document is referred to by its id alone: `findBy` (a removal frees the
    /// keys it finds by), `inList` (a list on a document that may leave state) and an operand
    /// of a reference expression (whose other operands know nothing of removal records) are
    /// each refused.
    #[test]
    fn should_refuse_a_moderated_document_refers_to_not_by_its_id() {
        for refers_to in [
            json!({
                "type": "moderatedDocument",
                "documentType": "post",
                "findBy": { "slug": "." }
            }),
            json!({
                "type": "moderatedDocument",
                "documentType": "post",
                "findBy": { "$id": "parentId" },
                "inList": "members"
            }),
            json!({
                "anyOf": [
                    { "type": "identity" },
                    { "type": "moderatedDocument", "documentType": "post" }
                ]
            }),
            json!({ "type": "moderatedDocument" }),
        ] {
            let schema = json!({
                "type": "object",
                "properties": {
                    "postId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 0,
                        "refersTo": refers_to.clone()
                    },
                    "parentId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 1
                    }
                },
                "required": [],
                "additionalProperties": false
            });
            // By the parser alone, as a stored contract is read, and with the meta-schema
            try_document_type_from_schema(schema.clone())
                .expect_err(&format!("{refers_to} must be refused"));
            try_document_type_from_schema_full_validation(schema).expect_err(&format!(
                "{refers_to} must be refused under full validation"
            ));
        }
    }

    /// A writer gate on a moderatedDocument reference is asked about on every replace, and
    /// only a removed document's owner and id are sure to be in its record: a gate on anything
    /// else is refused, one on `$ownerId` or `$id` parses.
    #[test]
    fn should_refuse_a_moderated_document_writer_gate_the_removal_record_can_not_answer() {
        let schema = |where_entries: serde_json::Value| {
            json!({
                "type": "object",
                "properties": {
                    "postId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 0,
                        "refersTo": {
                            "type": "moderatedDocument",
                            "documentType": "post",
                            "where": where_entries
                        }
                    },
                    "authorId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 1
                    }
                },
                "required": [],
                "additionalProperties": false
            })
        };
        for refused in [
            json!({ "$creatorId": "$ownerId" }),
            json!({ "authorId": "$ownerId" }),
        ] {
            let error = try_document_type_from_schema(schema(refused.clone()))
                .expect_err(&format!("{refused} must be refused"));
            assert!(
                error.to_string().contains("compares the writer"),
                "unexpected error: {error}"
            );
        }
        for admitted in [
            json!({ "$ownerId": "$ownerId" }),
            json!({ "$creatorId": "authorId" }),
        ] {
            try_document_type_from_schema(schema(admitted.clone()))
                .unwrap_or_else(|error| panic!("{admitted} must parse: {error}"));
        }
    }

    #[test]
    fn should_reject_deletable_document_refers_to_without_document_type() {
        let error = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "draftId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "deletableDocument"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");
        assert!(!error.to_string().is_empty());

        let error = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "draftId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "deletableDocument",
                        "documentType": "d".repeat(65)
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");
        assert!(
            error
                .to_string()
                .contains("deletableDocument refersTo documentType must be between"),
            "{error}"
        );
    }

    #[test]
    fn should_parse_identity_public_key_refers_to() {
        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "toUserId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "identityPublicKey",
                        "keyIdProperty": "toKeyIndex"
                    }
                },
                "toKeyIndex": {
                    "type": "integer",
                    "position": 1
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect("should parse");

        let property_type = document_type
            .as_ref()
            .flattened_properties()
            .get("toUserId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        assert_eq!(
            property_type,
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::IdentityPublicKey {
                    key_id_property: "toKeyIndex".to_string(),
                    key_requirements: IdentityKeyReferenceRequirements::default(),
                }
            )
        );
    }

    fn identity_key_reference_schema(refers_to: serde_json::Value) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "recipientId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": refers_to
                },
                "recipientKeyId": {
                    "type": "integer",
                    "position": 1
                }
            },
            "required": [],
            "additionalProperties": false
        })
    }

    fn identity_key_reference_target(refers_to: serde_json::Value) -> DocumentPropertyType {
        try_document_type_from_schema(identity_key_reference_schema(refers_to))
            .expect("should parse")
            .as_ref()
            .flattened_properties()
            .get("recipientId")
            .map(|p| p.property_type.clone())
            .expect("property should be present")
    }

    #[test]
    fn should_parse_identity_public_key_refers_to_with_key_requirements() {
        assert_eq!(
            identity_key_reference_target(json!({
                "type": "identityPublicKey",
                "keyIdProperty": "recipientKeyId",
                "keyRequirements": { "purpose": "decryption", "boundTo": "submittedCharter" }
            })),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::IdentityPublicKey {
                    key_id_property: "recipientKeyId".to_string(),
                    key_requirements: IdentityKeyReferenceRequirements {
                        purpose: Some(Purpose::DECRYPTION),
                        bound_to: Some("submittedCharter".to_string()),
                    },
                }
            )
        );
        for (name, purpose) in [
            ("authentication", Purpose::AUTHENTICATION),
            ("encryption", Purpose::ENCRYPTION),
            ("decryption", Purpose::DECRYPTION),
            ("transfer", Purpose::TRANSFER),
            ("voting", Purpose::VOTING),
            ("owner", Purpose::OWNER),
        ] {
            assert_eq!(
                identity_key_reference_target(json!({
                    "type": "identityPublicKey",
                    "keyIdProperty": "recipientKeyId",
                    "keyRequirements": { "purpose": name }
                })),
                DocumentPropertyType::IdentifierWithReference(
                    DocumentPropertyReferenceTarget::IdentityPublicKey {
                        key_id_property: "recipientKeyId".to_string(),
                        key_requirements: IdentityKeyReferenceRequirements {
                            purpose: Some(purpose),
                            bound_to: None,
                        },
                    }
                )
            );
        }
        assert_eq!(
            identity_key_reference_target(json!({
                "type": "identityPublicKey",
                "keyIdProperty": "recipientKeyId",
                "keyRequirements": { "boundTo": "joinRequest" }
            })),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::IdentityPublicKey {
                    key_id_property: "recipientKeyId".to_string(),
                    key_requirements: IdentityKeyReferenceRequirements {
                        purpose: None,
                        bound_to: Some("joinRequest".to_string()),
                    },
                }
            )
        );
    }

    #[test]
    fn should_reject_key_requirements_that_are_empty_unknown_or_on_another_type() {
        for (refers_to, fragment) in [
            (
                json!({ "type": "identityPublicKey", "keyIdProperty": "recipientKeyId", "keyRequirements": {} }),
                "at least one requirement",
            ),
            (
                json!({ "type": "identityPublicKey", "keyIdProperty": "recipientKeyId", "keyRequirements": { "purpose": "signing" } }),
                "is unknown",
            ),
            (
                json!({ "type": "identityPublicKey", "keyIdProperty": "recipientKeyId", "keyRequirements": { "purpose": "system" } }),
                "is unknown",
            ),
            (
                json!({ "type": "identityPublicKey", "keyIdProperty": "recipientKeyId", "keyRequirements": { "purpose": "DECRYPTION" } }),
                "is unknown",
            ),
            (
                json!({ "type": "identityPublicKey", "keyIdProperty": "recipientKeyId", "keyRequirements": { "purpose": 2 } }),
                "must be a string",
            ),
            (
                json!({ "type": "identityPublicKey", "keyIdProperty": "recipientKeyId", "keyRequirements": { "boundTo": "" } }),
                "between 1 and 64 characters",
            ),
            (
                json!({ "type": "identityPublicKey", "keyIdProperty": "recipientKeyId", "keyRequirements": { "boundTo": 1 } }),
                "must be a string",
            ),
            (
                json!({ "type": "identityPublicKey", "keyIdProperty": "recipientKeyId", "keyRequirements": { "securityLevel": "high" } }),
                "is unknown",
            ),
            (
                json!({ "type": "identity", "keyRequirements": { "purpose": "decryption" } }),
                "does not take keyRequirements",
            ),
            (
                json!({ "type": "contract", "keyRequirements": { "purpose": "decryption" } }),
                "does not take keyRequirements",
            ),
        ] {
            let err =
                try_document_type_from_schema(identity_key_reference_schema(refers_to.clone()))
                    .expect_err("should be refused");
            assert!(
                err.to_string().contains(fragment),
                "{refers_to}: expected {fragment:?}, got {err}"
            );
        }
    }

    #[test]
    fn should_reject_identity_public_key_refers_to_without_key_id_property() {
        try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "toUserId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "identityPublicKey"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect_err("should fail");
    }

    // ================================================================
    //  identityPublicKey on the key id property (identityProperty)
    // ================================================================

    /// A `message` document type whose `senderKeyId` carries `refers_to`, with
    /// the property's own keywords under `key_id_schema` (the `u32` key id
    /// range by default).
    fn key_id_reference_schema(
        key_id_schema: serde_json::Value,
        refers_to: serde_json::Value,
    ) -> serde_json::Value {
        let mut sender_key_id = key_id_schema;
        sender_key_id["position"] = json!(0);
        sender_key_id["refersTo"] = refers_to;
        json!({
            "type": "object",
            "properties": {
                "senderKeyId": sender_key_id,
                "note": { "type": "string", "maxLength": 64, "position": 1 }
            },
            "required": [],
            "additionalProperties": false
        })
    }

    fn u32_key_id_schema() -> serde_json::Value {
        json!({ "type": "integer", "minimum": 0, "maximum": 4294967295u64 })
    }

    fn owner_key_refers_to() -> serde_json::Value {
        json!({ "type": "identityPublicKey", "identityProperty": "$ownerId" })
    }

    fn sender_key_id_type(document_type: &DocumentType) -> DocumentPropertyType {
        document_type
            .as_ref()
            .flattened_properties()
            .get("senderKeyId")
            .map(|p| p.property_type.clone())
            .expect("property should be present")
    }

    #[test]
    fn should_parse_identity_public_key_refers_to_on_the_key_id_property() {
        // With and without the meta-schema: the v3 meta-schema admits the form
        for document_type in [
            try_document_type_from_schema(key_id_reference_schema(
                u32_key_id_schema(),
                owner_key_refers_to(),
            ))
            .expect("should parse"),
            try_document_type_from_schema_full_validation(key_id_reference_schema(
                u32_key_id_schema(),
                owner_key_refers_to(),
            ))
            .expect("should parse under the meta-schema"),
        ] {
            assert_eq!(
                sender_key_id_type(&document_type),
                DocumentPropertyType::KeyIdWithReference(KeyIdReference::new(
                    KeyReferenceIdentityProperty::OwnerId
                ))
            );
            // A key id property is still an integer to everything that asks
            assert!(sender_key_id_type(&document_type).is_integer());
        }
    }

    /// The form sits on a nested property too, under the meta-schema and in
    /// the flattened properties consensus walks, at its dotted path.
    #[test]
    fn should_parse_identity_public_key_refers_to_on_a_nested_key_id_property() {
        let mut sender_key_id = u32_key_id_schema();
        sender_key_id["position"] = json!(0);
        sender_key_id["refersTo"] = owner_key_refers_to();
        let schema = json!({
            "type": "object",
            "properties": {
                "meta": {
                    "type": "object",
                    "position": 0,
                    "properties": { "senderKeyId": sender_key_id },
                    "additionalProperties": false
                }
            },
            "required": [],
            "additionalProperties": false
        });

        let document_type = try_document_type_from_schema_full_validation(schema)
            .expect("should parse under the meta-schema");

        assert_eq!(
            document_type
                .as_ref()
                .flattened_properties()
                .get("meta.senderKeyId")
                .map(|p| p.property_type.clone()),
            Some(DocumentPropertyType::KeyIdWithReference(
                KeyIdReference::new(KeyReferenceIdentityProperty::OwnerId)
            ))
        );
    }

    /// `keyRequirements` sit on the key id form exactly as on the identifier
    /// form.
    #[test]
    fn should_parse_key_requirements_on_the_key_id_form() {
        let schema = key_id_reference_schema(
            u32_key_id_schema(),
            json!({
                "type": "identityPublicKey",
                "identityProperty": "$ownerId",
                "keyRequirements": { "purpose": "encryption" }
            }),
        );
        for document_type in [
            try_document_type_from_schema(schema.clone()).expect("should parse"),
            try_document_type_from_schema_full_validation(schema)
                .expect("should parse under the meta-schema"),
        ] {
            assert_eq!(
                sender_key_id_type(&document_type),
                DocumentPropertyType::KeyIdWithReference(KeyIdReference {
                    identity_property: KeyReferenceIdentityProperty::OwnerId,
                    key_requirements: IdentityKeyReferenceRequirements {
                        purpose: Some(Purpose::ENCRYPTION),
                        bound_to: None,
                    },
                })
            );
        }
    }

    #[test]
    fn should_reject_identity_property_on_an_identifier_property() {
        let schema = key_id_reference_schema(
            json!({
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier"
            }),
            owner_key_refers_to(),
        );
        let err = try_document_type_from_schema(schema.clone()).expect_err("should fail");
        assert!(
            err.to_string()
                .contains("only allowed on a key id property"),
            "unexpected error: {err}"
        );
        try_document_type_from_schema_full_validation(schema)
            .expect_err("the meta-schema should refuse the form on an identifier");
    }

    #[test]
    fn should_reject_identity_property_on_an_integer_without_the_u32_range() {
        for key_id_schema in [
            // Narrower than a key id
            json!({ "type": "integer", "minimum": 0, "maximum": 255 }),
            // Wider than a key id
            json!({ "type": "integer", "minimum": 0 }),
            json!({ "type": "integer", "minimum": -1, "maximum": 4294967295u64 }),
            // Narrower at the bottom: the rule is the exact range, as the
            // meta-schema pins it, not the inferred u32
            json!({ "type": "integer", "minimum": 1, "maximum": 4294967295u64 }),
            // Not an integer at all
            json!({ "type": "string", "maxLength": 10 }),
        ] {
            let schema = key_id_reference_schema(key_id_schema.clone(), owner_key_refers_to());
            let err = try_document_type_from_schema(schema.clone()).expect_err("should fail");
            assert!(
                err.to_string()
                    .contains("only allowed on a key id property"),
                "{key_id_schema}: unexpected error: {err}"
            );
            try_document_type_from_schema_full_validation(schema)
                .expect_err("the meta-schema should refuse a property outside the u32 range");
        }
    }

    #[test]
    fn should_reject_identity_property_together_with_key_id_property() {
        let schema = key_id_reference_schema(
            u32_key_id_schema(),
            json!({
                "type": "identityPublicKey",
                "identityProperty": "$ownerId",
                "keyIdProperty": "senderKeyId"
            }),
        );
        let err = try_document_type_from_schema(schema.clone()).expect_err("should fail");
        assert!(
            err.to_string().contains("not both"),
            "unexpected error: {err}"
        );
        try_document_type_from_schema_full_validation(schema)
            .expect_err("the meta-schema should refuse both keys on one declaration");
    }

    /// `identityProperty` admits `$creatorId` and a property path beside
    /// `$ownerId`; whether the path or the creator fits the document type is
    /// a registration check, not a parse
    #[test]
    fn should_parse_identity_public_key_refers_to_naming_the_creator_or_a_property() {
        for (identity_property, expected) in [
            ("$creatorId", KeyReferenceIdentityProperty::CreatorId),
            (
                "note",
                KeyReferenceIdentityProperty::Property("note".to_string()),
            ),
            (
                "meta.toUserId",
                KeyReferenceIdentityProperty::Property("meta.toUserId".to_string()),
            ),
        ] {
            let schema = key_id_reference_schema(
                u32_key_id_schema(),
                json!({ "type": "identityPublicKey", "identityProperty": identity_property }),
            );
            for document_type in [
                try_document_type_from_schema(schema.clone()).expect("should parse"),
                try_document_type_from_schema_full_validation(schema)
                    .expect("should parse under the meta-schema"),
            ] {
                assert_eq!(
                    sender_key_id_type(&document_type),
                    DocumentPropertyType::KeyIdWithReference(KeyIdReference::new(expected.clone())),
                    "{identity_property}"
                );
            }
        }
    }

    #[test]
    fn should_reject_identity_property_values_that_are_neither_a_system_name_nor_a_path() {
        for identity_property in [
            json!("$id"),
            json!("$creatorId2"),
            json!(""),
            json!("a".repeat(257)),
            json!("with-dash"),
            json!(5),
        ] {
            let schema = key_id_reference_schema(
                u32_key_id_schema(),
                json!({ "type": "identityPublicKey", "identityProperty": identity_property }),
            );
            // A hyphenated path is the meta-schema's refusal alone: the parser
            // reads stored contracts as they are
            if identity_property != json!("with-dash") {
                let err = try_document_type_from_schema(schema.clone()).expect_err("should fail");
                assert!(
                    err.to_string().contains("identityProperty"),
                    "{identity_property}: unexpected error: {err}"
                );
            }
            try_document_type_from_schema_full_validation(schema)
                .expect_err("the meta-schema should refuse the value");
        }
    }

    #[test]
    fn should_reject_identity_property_on_other_reference_types() {
        for reference_type in ["identity", "contract", "token", "deletableDocument"] {
            let schema = key_id_reference_schema(
                u32_key_id_schema(),
                json!({
                    "type": reference_type,
                    "documentType": "note",
                    "identityProperty": "$ownerId"
                }),
            );
            let err = try_document_type_from_schema(schema.clone()).expect_err("should fail");
            assert!(
                err.to_string().contains(&format!(
                    "{reference_type} refersTo does not take identityProperty"
                )),
                "{reference_type}: unexpected error: {err}"
            );
            try_document_type_from_schema_full_validation(schema)
                .expect_err("the meta-schema should refuse identityProperty on other types");
        }
    }

    #[test]
    fn should_reject_property_agreement_on_the_key_id_form() {
        let schema = key_id_reference_schema(
            u32_key_id_schema(),
            json!({
                "type": "identityPublicKey",
                "identityProperty": "$ownerId",
                "where": {
                    "note": "note"
                }
            }),
        );
        let err = try_document_type_from_schema(schema.clone()).expect_err("should fail");
        assert!(
            err.to_string()
                .contains("identityPublicKey refersTo does not take where"),
            "unexpected error: {err}"
        );
        try_document_type_from_schema_full_validation(schema)
            .expect_err("the meta-schema should refuse where here");
    }

    #[test]
    fn should_ignore_the_key_id_form_on_platform_versions_predating_it() {
        // Protocol version 13 predates `refersTo` entirely: parsed without the
        // meta-schema the keyword is ignored and the property stays the plain
        // u32 it always was; its meta-schema (v2) refuses the keyword outright
        let platform_version = PlatformVersion::get(13).expect("platform version 13 should exist");
        let schema = key_id_reference_schema(u32_key_id_schema(), owner_key_refers_to());

        let document_type =
            try_document_type_from_schema_on_version(schema.clone(), platform_version)
                .expect("should parse");
        assert_eq!(
            sender_key_id_type(&document_type),
            DocumentPropertyType::U32
        );

        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");
        DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            platform_value::to_value(schema).expect("schema should convert"),
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            platform_version,
        )
        .expect_err("the v2 meta-schema should refuse refersTo");
    }

    #[test]
    fn should_ignore_refers_to_on_platform_versions_predating_it() {
        // Platform versions whose tables carry `apply_property_reference: None`
        // predate the `refersTo` keyword: even if it appears in a schema they
        // parse (only possible without full validation — their meta-schemas
        // reject it), they must ignore it and keep producing the plain
        // identifier type they always produced.
        let platform_version = PlatformVersion::get(13).expect("platform version 13 should exist");

        let document_type = try_document_type_from_schema_on_version(
            json!({
                "type": "object",
                "properties": {
                    "toUserId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 0,
                        "refersTo": {
                            "type": "identity"
                        }
                    }
                },
                "required": [],
                "additionalProperties": false
            }),
            platform_version,
        )
        .expect("should parse");

        let property_type = document_type
            .as_ref()
            .flattened_properties()
            .get("toUserId")
            .map(|p| p.property_type.clone())
            .expect("property should be present");

        assert!(matches!(property_type, DocumentPropertyType::Identifier));
    }

    #[test]
    fn should_not_reject_refers_to_on_non_identifier_property_on_platform_versions_predating_it() {
        let platform_version = PlatformVersion::get(13).expect("platform version 13 should exist");

        try_document_type_from_schema_on_version(
            json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "position": 0,
                        "refersTo": { "type": "identity" }
                    }
                },
                "required": [],
                "additionalProperties": false
            }),
            platform_version,
        )
        .expect("a parse predating refersTo should ignore the keyword entirely");
    }

    // ================================================================
    //  distinctFrom
    // ================================================================

    /// An identifier property schema, with `distinct_from` as its `distinctFrom`
    /// when given.
    fn identifier_property(position: u32, distinct_from: Option<&str>) -> serde_json::Value {
        let mut property = json!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": position
        });
        if let Some(distinct_from) = distinct_from {
            property["distinctFrom"] = json!(distinct_from);
        }
        property
    }

    /// A document type with `delegateId` declaring `distinct_from`, next to the
    /// identifier `toUserId`, the string `note` and the nested object `meta`
    /// holding the identifier `meta.reviewerId` and the string `meta.tag`.
    fn distinct_from_schema(distinct_from: Option<&str>) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "delegateId": identifier_property(0, distinct_from),
                "toUserId": identifier_property(1, None),
                "note": {"type": "string", "maxLength": 32, "position": 2},
                "meta": {
                    "type": "object",
                    "position": 3,
                    "properties": {
                        "reviewerId": identifier_property(0, None),
                        "tag": {"type": "string", "maxLength": 32, "position": 1}
                    },
                    "additionalProperties": false
                }
            },
            "required": [],
            "additionalProperties": false
        })
    }

    fn distinct_from_of(document_type: &DocumentType, path: &str) -> Option<DistinctFrom> {
        document_type
            .as_ref()
            .flattened_properties()
            .get(path)
            .expect("property should be present")
            .distinct_from
            .clone()
    }

    #[test]
    fn should_parse_distinct_from_owner_id() {
        for full_validation in [true, false] {
            let document_type = if full_validation {
                try_document_type_from_schema_full_validation(distinct_from_schema(Some(
                    "$ownerId",
                )))
            } else {
                try_document_type_from_schema(distinct_from_schema(Some("$ownerId")))
            }
            .expect("should parse");

            assert_eq!(
                distinct_from_of(&document_type, "delegateId"),
                Some(DistinctFrom::OwnerId),
                "full_validation: {full_validation}"
            );
            assert_eq!(distinct_from_of(&document_type, "toUserId"), None);
        }
    }

    #[test]
    fn should_parse_distinct_from_property_path_on_top_level_and_nested_properties() {
        let document_type = try_document_type_from_schema_full_validation(json!({
            "type": "object",
            "properties": {
                "delegateId": identifier_property(0, Some("meta.reviewerId")),
                "meta": {
                    "type": "object",
                    "position": 1,
                    "properties": {
                        "reviewerId": identifier_property(0, Some("delegateId")),
                        "tag": {"type": "string", "maxLength": 32, "position": 1}
                    },
                    "additionalProperties": false
                }
            },
            "required": [],
            "additionalProperties": false
        }))
        .expect("should parse");

        assert_eq!(
            distinct_from_of(&document_type, "delegateId"),
            Some(DistinctFrom::Property("meta.reviewerId".to_string()))
        );
        assert_eq!(
            distinct_from_of(&document_type, "meta.reviewerId"),
            Some(DistinctFrom::Property("delegateId".to_string()))
        );
    }

    #[test]
    fn should_reject_distinct_from_on_a_non_identifier_property() {
        let schema = json!({
            "type": "object",
            "properties": {
                "note": {"type": "string", "maxLength": 32, "position": 0, "distinctFrom": "$ownerId"},
                "toUserId": identifier_property(1, None)
            },
            "required": [],
            "additionalProperties": false
        });

        // Without the meta-schema the parser refuses it itself
        let err = try_document_type_from_schema(schema.clone()).expect_err("should be refused");
        assert!(
            err.to_string()
                .contains("distinctFrom is only allowed on identifier properties"),
            "got {err}"
        );

        // With it the meta-schema's dependent schema refuses it first, before
        // the parser gets to run, so the parser's message must not be the one
        let err = try_document_type_from_schema_full_validation(schema)
            .expect_err("the meta-schema should refuse it");
        assert!(
            !err.to_string()
                .contains("distinctFrom is only allowed on identifier properties"),
            "the meta-schema, not the parser, must refuse it: {err}"
        );
    }

    #[test]
    fn should_reject_distinct_from_naming_a_property_that_does_not_exist() {
        for target in ["missing", "meta.missing", "note.deeper"] {
            let err = try_document_type_from_schema(distinct_from_schema(Some(target)))
                .expect_err("should be refused");
            assert!(
                err.to_string()
                    .contains(&format!("declares distinctFrom \"{target}\", but the document type has no property at that path")),
                "{target}: got {err}"
            );
        }
    }

    #[test]
    fn should_reject_distinct_from_naming_a_non_identifier_property() {
        for target in ["note", "meta.tag", "meta"] {
            let err = try_document_type_from_schema(distinct_from_schema(Some(target)))
                .expect_err("should be refused");
            // An object is not in the flattened map at all; its members are
            let expected = if target == "meta" {
                "which is an object, not an identifier property".to_string()
            } else {
                format!("declares distinctFrom \"{target}\", which is not an identifier property")
            };
            assert!(err.to_string().contains(&expected), "{target}: got {err}");
        }
    }

    #[test]
    fn should_reject_distinct_from_naming_itself() {
        let err = try_document_type_from_schema(distinct_from_schema(Some("delegateId")))
            .expect_err("should be refused");
        assert!(
            err.to_string().contains("declares distinctFrom itself"),
            "got {err}"
        );
    }

    #[test]
    fn should_reject_distinct_from_naming_a_system_property_other_than_owner_id() {
        for (target, fragment) in [
            ("$id", "not system property \"$id\""),
            ("$creatorId", "not system property \"$creatorId\""),
            ("", "between 1 and 256 characters"),
        ] {
            let err = try_document_type_from_schema(distinct_from_schema(Some(target)))
                .expect_err("should be refused");
            assert!(err.to_string().contains(fragment), "{target:?}: got {err}");
        }
    }

    #[test]
    fn should_reject_distinct_from_that_is_not_a_string() {
        let mut schema = distinct_from_schema(None);
        schema["properties"]["delegateId"]["distinctFrom"] = json!(["$ownerId"]);
        let err = try_document_type_from_schema(schema).expect_err("should be refused");
        assert!(
            err.to_string().contains("distinctFrom must be a string"),
            "got {err}"
        );
    }

    #[test]
    fn should_refuse_distinct_from_below_protocol_version_14_under_full_validation() {
        // Meta-schema v2 (protocol version 13) knows no such keyword, so a
        // registering parse refuses it.
        let platform_version = PlatformVersion::get(13).expect("platform version 13 should exist");
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");
        let value = platform_value::to_value(distinct_from_schema(Some("$ownerId")))
            .expect("schema should convert");

        DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            platform_version,
        )
        .expect_err("protocol version 13 should refuse the keyword");
    }

    #[test]
    fn should_ignore_distinct_from_below_protocol_version_14_without_full_validation() {
        // Platform versions whose tables carry `apply_distinct_from: None`
        // predate the keyword: without the meta-schema they must ignore it and
        // keep producing the plain property they always produced.
        let platform_version = PlatformVersion::get(13).expect("platform version 13 should exist");

        let document_type = try_document_type_from_schema_on_version(
            distinct_from_schema(Some("$ownerId")),
            platform_version,
        )
        .expect("should parse");

        assert_eq!(distinct_from_of(&document_type, "delegateId"), None);
    }

    #[test]
    fn should_accept_distinct_from_at_protocol_version_14() {
        let platform_version = PlatformVersion::get(14).expect("platform version 14 should exist");

        let document_type = try_document_type_from_schema_on_version(
            distinct_from_schema(Some("toUserId")),
            platform_version,
        )
        .expect("should parse");

        assert_eq!(
            distinct_from_of(&document_type, "delegateId"),
            Some(DistinctFrom::Property("toUserId".to_string()))
        );
    }

    #[test]
    fn should_parse_distinct_from_next_to_refers_to_and_judge_the_referencing_property() {
        let mut schema = distinct_from_schema(Some("toUserId"));
        schema["properties"]["delegateId"]["refersTo"] = json!({ "type": "identity" });
        let document_type =
            try_document_type_from_schema_full_validation(schema).expect("should parse");

        let property = document_type
            .as_ref()
            .flattened_properties()
            .get("delegateId")
            .expect("property should be present")
            .clone();
        assert_eq!(
            property.property_type,
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::Identity
            )
        );
        assert_eq!(
            property.distinct_from,
            Some(DistinctFrom::Property("toUserId".to_string()))
        );

        let data = BTreeMap::from([
            ("delegateId".to_string(), Value::Identifier([1; 32])),
            ("toUserId".to_string(), Value::Identifier([1; 32])),
        ]);
        let result = document_type
            .as_ref()
            .validate_distinct_from_properties(
                &data,
                Identifier::from([2; 32]),
                PlatformVersion::latest(),
            )
            .expect("the check should run");
        assert!(
            !result.is_valid(),
            "a referencing property is judged like any other identifier"
        );
    }

    #[test]
    fn should_not_judge_distinct_from_before_protocol_version_14() {
        // A document type parsed at 14 carries the declaration; judged through
        // the dispatcher at 13, whose table has no `validate_distinct_from`,
        // nothing is checked, as no property parsed there could declare it.
        let document_type = try_document_type_from_schema(distinct_from_schema(Some("$ownerId")))
            .expect("should parse");
        let owner_id = Identifier::from([1; 32]);
        let data = BTreeMap::from([("delegateId".to_string(), Value::Identifier([1; 32]))]);

        let before = document_type
            .as_ref()
            .validate_distinct_from_properties(
                &data,
                owner_id,
                PlatformVersion::get(13).expect("platform version 13 should exist"),
            )
            .expect("the check should run");
        assert!(before.is_valid(), "{:?}", before.errors);

        let at = document_type
            .as_ref()
            .validate_distinct_from_properties(&data, owner_id, PlatformVersion::latest())
            .expect("the check should run");
        assert!(!at.is_valid());
    }

    /// A typed array of identifiers whose items declare `distinct_from`.
    fn identifier_array_property(position: u32, distinct_from: Option<&str>) -> serde_json::Value {
        let mut items = json!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier"
        });
        if let Some(distinct_from) = distinct_from {
            items["distinctFrom"] = json!(distinct_from);
        }
        json!({ "type": "array", "maxItems": 8, "items": items, "position": position })
    }

    #[test]
    fn should_parse_distinct_from_on_the_items_of_an_identifier_array_and_judge_every_element() {
        let mut schema = distinct_from_schema(None);
        schema["properties"]["members"] = identifier_array_property(4, Some("$ownerId"));
        let document_type =
            try_document_type_from_schema_full_validation(schema).expect("should parse");

        assert_eq!(
            distinct_from_of(&document_type, "members"),
            Some(DistinctFrom::OwnerId)
        );

        let owner_id = Identifier::from([1; 32]);
        let judge = |members: Vec<Value>| {
            let data = BTreeMap::from([("members".to_string(), Value::Array(members))]);
            document_type
                .as_ref()
                .validate_distinct_from_properties(&data, owner_id, PlatformVersion::latest())
                .expect("the check should run")
        };

        assert!(judge(vec![]).is_valid());
        assert!(judge(vec![Value::Identifier([2; 32]), Value::Identifier([3; 32])]).is_valid());
        let refused = judge(vec![Value::Identifier([2; 32]), Value::Identifier([1; 32])]);
        assert_matches!(
            refused.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DocumentPropertyNotDistinctError(e))]
                if e.property() == "members" && e.distinct_from() == "$ownerId"
        );
    }

    #[test]
    fn should_reject_distinct_from_on_the_array_itself_or_on_non_identifier_items() {
        for (property, fragment) in [
            (
                json!({
                    "type": "array", "maxItems": 8, "position": 4, "distinctFrom": "$ownerId",
                    "items": {
                        "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier"
                    }
                }),
                "belongs on its items",
            ),
            (
                json!({
                    "type": "array", "maxItems": 8, "position": 4,
                    "items": { "type": "integer", "distinctFrom": "$ownerId" }
                }),
                "only allowed on identifier elements",
            ),
        ] {
            let mut schema = distinct_from_schema(None);
            schema["properties"]["members"] = property;
            let err = try_document_type_from_schema(schema.clone()).expect_err("should be refused");
            assert!(err.to_string().contains(fragment), "got {err}");
            try_document_type_from_schema_full_validation(schema)
                .expect_err("the meta-schema should refuse it too");
        }
    }

    #[test]
    fn should_round_trip_a_contract_through_platform_serialization_with_and_without_distinct_from()
    {
        let platform_version = PlatformVersion::latest();

        for distinct_from in [None, Some("$ownerId"), Some("toUserId")] {
            let contract_value = platform_value::to_value(json!({
                "$formatVersion": "1",
                "id": Identifier::from([7u8; 32]).to_string(Encoding::Base58),
                "ownerId": Identifier::from([8u8; 32]).to_string(Encoding::Base58),
                "version": 1,
                "documentSchemas": {
                    "message": distinct_from_schema(distinct_from)
                }
            }))
            .expect("contract should convert");
            let contract = DataContract::from_value(contract_value, true, platform_version)
                .expect("the contract should parse");

            let bytes = contract
                .serialize_to_bytes_with_platform_version(platform_version)
                .expect("the contract should serialize");
            let recovered =
                DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
                    .expect("the contract should deserialize");

            assert_eq!(contract, recovered, "distinctFrom {distinct_from:?}");
            let expected = distinct_from
                .map(|name| DistinctFrom::from_wire_name(name).expect("a valid declaration"));
            let message_type = recovered
                .document_type_for_name("message")
                .expect("the message type")
                .to_owned_document_type();
            assert_eq!(distinct_from_of(&message_type, "delegateId"), expected);
        }
    }

    // ================================================================
    //  encryptedFor
    // ================================================================

    /// The `encryptedFor` declaration every encryption test starts from.
    fn encrypted_for_declaration() -> serde_json::Value {
        json!({
            "recipient": "recipientId",
            "recipientKey": "recipientKeyId",
            "senderKey": "senderKeyId",
            "scheme": "ecdh-secp256k1-aes256-cbc"
        })
    }

    /// A document type with a recipient identifier, two bounded key ids, an
    /// unbounded integer, a string and a nested object next to the
    /// `encryptedMessage` byte array carrying `encrypted_for`.
    fn encrypted_schema(encrypted_for: serde_json::Value) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "recipientId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0
                },
                "recipientKeyId": { "type": "integer", "minimum": 0, "maximum": 4294967295_u64, "position": 1 },
                "senderKeyId": { "type": "integer", "minimum": 0, "maximum": 4294967295_u64, "position": 2 },
                "unboundedKeyId": { "type": "integer", "minimum": 0, "position": 3 },
                "note": { "type": "string", "maxLength": 63, "position": 4 },
                "maxOnlyKeyId": { "type": "integer", "maximum": 100, "position": 7 },
                "meta": {
                    "type": "object",
                    "position": 5,
                    "properties": {
                        "authorId": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 32,
                            "maxItems": 32,
                            "contentMediaType": "application/x.dash.dpp.identifier",
                            "position": 0
                        }
                    },
                    "additionalProperties": false
                },
                "encryptedMessage": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 1040,
                    "position": 6,
                    "encryptedFor": encrypted_for
                }
            },
            "required": [],
            "additionalProperties": false
        })
    }

    fn encrypted_for_of(document_type: &DocumentType, path: &str) -> Option<EncryptedFor> {
        document_type
            .as_ref()
            .flattened_properties()
            .get(path)
            .expect("property should be present")
            .encrypted_for
            .clone()
    }

    #[test]
    fn should_parse_encrypted_for_on_a_byte_array_property() {
        let document_type = try_document_type_from_schema_full_validation(encrypted_schema(
            encrypted_for_declaration(),
        ))
        .expect("should parse");

        let expected = EncryptedFor {
            recipient: EncryptedForRecipient::Property("recipientId".to_string()),
            recipient_key: "recipientKeyId".to_string(),
            sender_key: "senderKeyId".to_string(),
            scheme: EncryptionScheme::EcdhSecp256k1Aes256Cbc,
        };
        assert_eq!(
            encrypted_for_of(&document_type, "encryptedMessage"),
            Some(expected.clone())
        );
        assert_eq!(
            document_type.as_ref().encrypted_properties(),
            vec![(&"encryptedMessage".to_string(), &expected)]
        );
        assert_eq!(encrypted_for_of(&document_type, "note"), None);
    }

    #[test]
    fn should_parse_encrypted_for_with_the_owner_and_a_nested_identifier_as_recipient() {
        for (recipient, expected) in [
            ("$ownerId", EncryptedForRecipient::Owner),
            (
                "meta.authorId",
                EncryptedForRecipient::Property("meta.authorId".to_string()),
            ),
        ] {
            let mut declaration = encrypted_for_declaration();
            declaration["recipient"] = json!(recipient);
            let document_type =
                try_document_type_from_schema(encrypted_schema(declaration)).expect("should parse");
            assert_eq!(
                encrypted_for_of(&document_type, "encryptedMessage")
                    .expect("should be declared")
                    .recipient,
                expected
            );
        }
    }

    #[test]
    fn should_reject_encrypted_for_on_a_non_byte_array_or_identifier_property() {
        let mut on_string = encrypted_schema(encrypted_for_declaration());
        on_string["properties"]["note"]["encryptedFor"] = encrypted_for_declaration();
        on_string["properties"]["encryptedMessage"]
            .as_object_mut()
            .expect("object")
            .remove("encryptedFor");
        let err = try_document_type_from_schema(on_string).expect_err("should be refused");
        assert!(
            err.to_string()
                .contains("encryptedFor is only allowed on byte array properties"),
            "{err}"
        );

        let mut on_identifier = encrypted_schema(encrypted_for_declaration());
        on_identifier["properties"]["recipientId"]["encryptedFor"] = encrypted_for_declaration();
        on_identifier["properties"]["encryptedMessage"]
            .as_object_mut()
            .expect("object")
            .remove("encryptedFor");
        let err =
            try_document_type_from_schema(on_identifier.clone()).expect_err("should be refused");
        assert!(
            err.to_string()
                .contains("encryptedFor is not allowed on identifier properties"),
            "{err}"
        );
        // The meta-schema refuses it on an identifier too, before the parser sees it
        try_document_type_from_schema_full_validation(on_identifier)
            .expect_err("the meta-schema should refuse encryptedFor on an identifier");
    }

    #[test]
    fn should_reject_encrypted_for_with_a_missing_unknown_or_malformed_key_or_an_unknown_scheme() {
        for (mutate, fragment) in [
            (
                Box::new(|d: &mut serde_json::Value| {
                    d.as_object_mut().expect("object").remove("recipient");
                }) as Box<dyn Fn(&mut serde_json::Value)>,
                "must declare recipient",
            ),
            (
                Box::new(|d: &mut serde_json::Value| {
                    d.as_object_mut().expect("object").remove("scheme");
                }),
                "must declare scheme",
            ),
            (
                Box::new(|d: &mut serde_json::Value| d["scheme"] = json!("rsa-oaep")),
                "scheme \"rsa-oaep\" is unknown",
            ),
            (
                Box::new(|d: &mut serde_json::Value| d["iv"] = json!("ivProperty")),
                "\"iv\" is unknown",
            ),
            (
                Box::new(|d: &mut serde_json::Value| d["recipient"] = json!(7)),
                "recipient must be a property path",
            ),
            (
                Box::new(|d: &mut serde_json::Value| d["recipientKey"] = json!("")),
                "recipientKey must be between 1 and 256 characters",
            ),
            (
                Box::new(|d: &mut serde_json::Value| d["recipient"] = json!("$createdAt")),
                "recipient must name an identifier property",
            ),
            (
                Box::new(|d: &mut serde_json::Value| d["senderKey"] = json!("$ownerId")),
                "senderKey must name an integer property",
            ),
        ] {
            let mut declaration = encrypted_for_declaration();
            mutate(&mut declaration);
            let err = try_document_type_from_schema(encrypted_schema(declaration.clone()))
                .expect_err("should be refused");
            assert!(
                err.to_string().contains(fragment),
                "{declaration}: expected {fragment:?}, got {err}"
            );
        }
    }

    #[test]
    fn should_reject_encrypted_for_naming_a_property_that_is_missing_or_of_the_wrong_type() {
        for (key, path, fragment) in [
            ("recipient", "note", "has type string, not identifier"),
            (
                "recipient",
                "nowhere",
                "is not a property of the document type",
            ),
            (
                "recipientKey",
                "note",
                "must be an integer property with minimum at least 0",
            ),
            (
                "recipientKey",
                "unboundedKeyId",
                "must be an integer property with minimum at least 0",
            ),
            (
                "recipientKey",
                "maxOnlyKeyId",
                "must be an integer property with minimum at least 0",
            ),
            (
                "senderKey",
                "recipientId",
                "must be an integer property with minimum at least 0",
            ),
            (
                "senderKey",
                "nowhere",
                "is not a property of the document type",
            ),
        ] {
            let mut declaration = encrypted_for_declaration();
            declaration[key] = json!(path);
            let err = try_document_type_from_schema(encrypted_schema(declaration))
                .expect_err("should be refused");
            assert!(
                err.to_string().contains(fragment),
                "{key}={path}: expected {fragment:?}, got {err}"
            );
        }
    }

    /// The document type of `schema` parsed at the latest platform version
    /// with the contract's `$defs`, which a `$ref` in it resolves against.
    fn try_document_type_from_schema_with_defs(
        schema: serde_json::Value,
        schema_defs: &BTreeMap<String, Value>,
        full_validation: bool,
    ) -> Result<DocumentType, ProtocolError> {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let value = platform_value::to_value(schema).expect("schema should convert");

        DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            value,
            Some(schema_defs),
            &BTreeMap::new(),
            &config,
            full_validation,
            &mut vec![],
            platform_version,
        )
    }

    /// The contract's `$defs` from a JSON object of definitions.
    fn schema_defs_from(defs: serde_json::Value) -> BTreeMap<String, Value> {
        platform_value::to_value(defs)
            .expect("the definitions convert")
            .into_btree_string_map()
            .expect("the definitions are a map")
    }

    /// A document type with one property `p` that is a `$ref` to `target`.
    fn schema_with_ref_property(target: &str) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "p": { "$ref": target, "position": 0 } },
            "additionalProperties": false
        })
    }

    fn assert_ref_cycle_refused(defs: serde_json::Value) {
        let started = std::time::Instant::now();
        let err = try_document_type_from_schema_with_defs(
            schema_with_ref_property("#/$defs/a"),
            &schema_defs_from(defs),
            false,
        )
        .expect_err("a cyclic $ref should be refused");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(1),
            "took {:?}",
            started.elapsed()
        );
        assert!(
            err.to_string()
                .contains("the ref '#/$defs/a' contains cycles"),
            "got {err}"
        );
    }

    /// An object whose two members both `$ref` it back would expand without
    /// end on the non-validating parse; it is refused instead.
    #[test]
    fn should_refuse_a_branching_ref_cycle_without_full_validation() {
        assert_ref_cycle_refused(json!({
            "a": {
                "type": "object",
                "properties": {
                    "b": { "$ref": "#/$defs/a", "position": 0 },
                    "c": { "$ref": "#/$defs/a", "position": 1 }
                },
                "additionalProperties": false
            }
        }));
    }

    #[test]
    fn should_refuse_a_self_ref_without_full_validation() {
        assert_ref_cycle_refused(json!({
            "a": {
                "type": "object",
                "properties": { "b": { "$ref": "#/$defs/a", "position": 0 } },
                "additionalProperties": false
            }
        }));
    }

    /// Only a `$ref` back to a target on the property's own path is a cycle:
    /// two siblings may name the same definition.
    #[test]
    fn should_parse_sibling_refs_to_one_definition_without_full_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "x": { "$ref": "#/$defs/leaf", "position": 0 },
                "y": { "$ref": "#/$defs/leaf", "position": 1 }
            },
            "additionalProperties": false
        });
        let defs = schema_defs_from(json!({
            "leaf": {
                "type": "object",
                "properties": { "v": { "type": "integer", "position": 0 } },
                "additionalProperties": false
            }
        }));
        let document_type = try_document_type_from_schema_with_defs(schema, &defs, false)
            .expect("sibling refs should parse");
        let flattened = document_type.flattened_properties();
        assert!(flattened.contains_key("x.v"), "got {:?}", flattened.keys());
        assert!(flattened.contains_key("y.v"), "got {:?}", flattened.keys());
        let properties = document_type.properties();
        assert!(properties.contains_key("x") && properties.contains_key("y"));
    }

    #[test]
    fn should_parse_a_chain_of_distinct_refs_without_full_validation() {
        let defs = schema_defs_from(json!({
            "a": {
                "type": "object",
                "properties": { "b": { "$ref": "#/$defs/b", "position": 0 } },
                "additionalProperties": false
            },
            "b": {
                "type": "object",
                "properties": { "c": { "$ref": "#/$defs/c", "position": 0 } },
                "additionalProperties": false
            },
            "c": {
                "type": "object",
                "properties": { "v": { "type": "integer", "position": 0 } },
                "additionalProperties": false
            }
        }));
        let document_type = try_document_type_from_schema_with_defs(
            schema_with_ref_property("#/$defs/a"),
            &defs,
            false,
        )
        .expect("a chain of distinct refs should parse");
        assert!(
            document_type.flattened_properties().contains_key("p.b.c.v"),
            "got {:?}",
            document_type.flattened_properties().keys()
        );
    }

    /// An acyclic `$defs` graph reaching one definition along many paths is no
    /// cycle, so the non-validating parse expands it in full: here into 2^12
    /// properties. Refusing such a graph is left to the schema depth check,
    /// which `check_tx` runs first; keep the size small.
    #[test]
    fn should_expand_an_acyclic_ref_dag_without_full_validation() {
        const LEVELS: usize = 12;
        let mut defs = serde_json::Map::new();
        for i in 0..LEVELS {
            let next = format!("#/$defs/d{}", i + 1);
            defs.insert(
                format!("d{i}"),
                json!({
                    "type": "object",
                    "properties": {
                        "b": { "$ref": next, "position": 0 },
                        "c": { "$ref": next, "position": 1 }
                    },
                    "additionalProperties": false
                }),
            );
        }
        defs.insert(format!("d{LEVELS}"), json!({ "type": "integer" }));

        let document_type = try_document_type_from_schema_with_defs(
            schema_with_ref_property("#/$defs/d0"),
            &schema_defs_from(serde_json::Value::Object(defs)),
            false,
        )
        .expect("an acyclic ref graph should parse");
        assert_eq!(document_type.flattened_properties().len(), 1 << LEVELS);
    }

    /// A key id property whose schema is a `$ref` to one of the contract's
    /// `$defs` is read from the definition, as the core parse reads it: a
    /// bounded integer there registers, and an unbounded one is refused as
    /// it is inline. On both paths. Each key has a definition of its own:
    /// the schema depth check refuses two references to one definition.
    #[test]
    fn should_read_an_encrypted_for_key_id_through_a_ref() {
        let key_id = || {
            platform_value::to_value(json!({
                "type": "integer",
                "minimum": 0,
                "maximum": 4294967295_u64
            }))
            .expect("the definition converts")
        };
        let schema_defs = BTreeMap::from([
            ("recipientKey".to_string(), key_id()),
            ("senderKey".to_string(), key_id()),
            (
                "anyInteger".to_string(),
                platform_value::to_value(json!({ "type": "integer", "minimum": 0 }))
                    .expect("the definition converts"),
            ),
        ]);
        let mut schema = encrypted_schema(encrypted_for_declaration());
        schema["properties"]["recipientKeyId"] =
            json!({ "$ref": "#/$defs/recipientKey", "position": 1 });
        schema["properties"]["senderKeyId"] = json!({ "$ref": "#/$defs/senderKey", "position": 2 });
        for full_validation in [true, false] {
            let document_type = try_document_type_from_schema_with_defs(
                schema.clone(),
                &schema_defs,
                full_validation,
            )
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
            let encrypted_for =
                encrypted_for_of(&document_type, "encryptedMessage").expect("should be declared");
            assert_eq!(encrypted_for.recipient_key, "recipientKeyId");
            assert_eq!(encrypted_for.sender_key, "senderKeyId");
        }

        schema["properties"]["senderKeyId"] =
            json!({ "$ref": "#/$defs/anyInteger", "position": 2 });
        for full_validation in [true, false] {
            let err = try_document_type_from_schema_with_defs(
                schema.clone(),
                &schema_defs,
                full_validation,
            )
            .expect_err("should be refused");
            assert!(
                err.to_string().contains(
                    "senderKey \"senderKeyId\" must be an integer property with minimum at least 0"
                ),
                "full_validation {full_validation}: got {err}"
            );
        }
    }

    /// A `$ref` may name a schema below one of the contract's `$defs`, as
    /// `#/$defs/keys/properties/recipient` does, and a key id property is then
    /// read from the schema found there, as the core parse reads it: a bounded
    /// integer registers, and an unbounded one is refused as it is inline. On
    /// both paths. Each key names a schema of its own: the schema depth check
    /// refuses two references to one schema.
    #[test]
    fn should_read_an_encrypted_for_key_id_through_a_ref_below_a_definition() {
        let schema_defs = BTreeMap::from([(
            "keys".to_string(),
            platform_value::to_value(json!({
                "type": "object",
                "properties": {
                    "recipient": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 4294967295_u64,
                        "position": 0
                    },
                    "sender": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 4294967295_u64,
                        "position": 1
                    },
                    "unbounded": { "type": "integer", "minimum": 0, "position": 2 }
                },
                "additionalProperties": false
            }))
            .expect("the definition converts"),
        )]);
        let mut schema = encrypted_schema(encrypted_for_declaration());
        schema["properties"]["recipientKeyId"] =
            json!({ "$ref": "#/$defs/keys/properties/recipient", "position": 1 });
        schema["properties"]["senderKeyId"] =
            json!({ "$ref": "#/$defs/keys/properties/sender", "position": 2 });
        for full_validation in [true, false] {
            let document_type = try_document_type_from_schema_with_defs(
                schema.clone(),
                &schema_defs,
                full_validation,
            )
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
            let encrypted_for =
                encrypted_for_of(&document_type, "encryptedMessage").expect("should be declared");
            assert_eq!(encrypted_for.recipient_key, "recipientKeyId");
            assert_eq!(encrypted_for.sender_key, "senderKeyId");
        }

        schema["properties"]["senderKeyId"] =
            json!({ "$ref": "#/$defs/keys/properties/unbounded", "position": 2 });
        for full_validation in [true, false] {
            let err = try_document_type_from_schema_with_defs(
                schema.clone(),
                &schema_defs,
                full_validation,
            )
            .expect_err("should be refused");
            assert!(
                err.to_string().contains(
                    "senderKey \"senderKeyId\" must be an integer property with minimum at least 0"
                ),
                "full_validation {full_validation}: got {err}"
            );
        }
    }

    #[test]
    fn should_refuse_encrypted_for_below_platform_version_14_and_accept_it_at_14() {
        let schema = encrypted_schema(encrypted_for_declaration());
        let v13 = PlatformVersion::get(13).expect("platform version 13 should exist");

        // The v2 meta-schema does not know the keyword: a validating parse refuses it
        let config = DataContractConfig::default_for_version(v13).expect("config should build");
        let value = platform_value::to_value(schema.clone()).expect("schema should convert");
        DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            v13,
        )
        .expect_err("platform version 13 should refuse encryptedFor under full validation");

        // Without validation the tables carry `apply_encrypted_for: None`, so the keyword
        // is ignored and the property parses as the plain byte array it always was
        let document_type =
            try_document_type_from_schema_on_version(schema.clone(), v13).expect("should parse");
        assert_eq!(encrypted_for_of(&document_type, "encryptedMessage"), None);
        assert!(document_type.as_ref().encrypted_properties().is_empty());

        // At 14 both parses carry it
        let document_type = try_document_type_from_schema_full_validation(schema.clone())
            .expect("should parse at platform version 14");
        assert!(encrypted_for_of(&document_type, "encryptedMessage").is_some());
        let document_type = try_document_type_from_schema(schema).expect("should parse");
        assert!(encrypted_for_of(&document_type, "encryptedMessage").is_some());
    }

    #[test]
    fn should_reject_encrypted_for_naming_a_transient_recipient_or_key() {
        for (transient, fragment) in [
            ("recipientId", "recipient \"recipientId\" is transient"),
            ("senderKeyId", "senderKey \"senderKeyId\" is transient"),
        ] {
            let mut schema = encrypted_schema(encrypted_for_declaration());
            schema["transient"] = json!([transient]);
            let err = try_document_type_from_schema(schema).expect_err("should be refused");
            assert!(err.to_string().contains(fragment), "{transient}: got {err}");
        }
    }

    /// `transient` lists the object, not its leaves, and the whole object is
    /// stripped before storage: a recipient or key id inside it is gone from
    /// the stored document however required it is.
    #[test]
    fn should_reject_encrypted_for_naming_a_recipient_or_key_inside_a_transient_object() {
        for (key, path) in [
            ("recipient", "meta.authorId"),
            ("recipientKey", "meta.keyId"),
            ("senderKey", "meta.keyId"),
        ] {
            let mut declaration = encrypted_for_declaration();
            declaration[key] = json!(path);
            let mut schema = encrypted_schema(declaration);
            schema["properties"]["meta"]["properties"]["keyId"] = json!({
                "type": "integer",
                "minimum": 0,
                "maximum": 4294967295_u64,
                "position": 1
            });
            schema["properties"]["meta"]["required"] = json!(["authorId", "keyId"]);
            schema["required"] = json!(["meta"]);

            // Stored with the document, the nested path is accepted
            try_document_type_from_schema_full_validation(schema.clone())
                .unwrap_or_else(|err| panic!("{key}={path} should parse: {err}"));

            schema["transient"] = json!(["meta"]);
            let fragment = format!("{key} \"{path}\" is transient or inside a transient object");
            for err in [
                try_document_type_from_schema(schema.clone()).expect_err("should be refused"),
                try_document_type_from_schema_full_validation(schema.clone())
                    .expect_err("should be refused under full validation"),
            ] {
                assert!(
                    err.to_string().contains(&fragment),
                    "{key}={path}: expected {fragment:?}, got {err}"
                );
            }
        }
    }

    #[test]
    fn should_find_a_path_transient_through_itself_or_an_enclosing_object_only() {
        let mut schema = encrypted_schema(encrypted_for_declaration());
        schema["transient"] = json!(["meta", "note"]);
        let document_type = try_document_type_from_schema(schema).expect("should parse");
        let document_type = document_type.as_ref();

        assert!(is_transient(document_type, "note"));
        assert!(is_transient(document_type, "meta"));
        assert!(is_transient(document_type, "meta.authorId"));
        assert!(is_transient(document_type, "meta.inner.leaf"));
        // A prefix counts only up to a dot: "metadata" is not inside "meta"
        assert!(!is_transient(document_type, "metadata.authorId"));
        assert!(!is_transient(document_type, "recipientId"));
    }

    #[test]
    fn should_reject_encrypted_for_on_a_byte_array_too_short_for_the_scheme() {
        let mut schema = encrypted_schema(encrypted_for_declaration());
        schema["properties"]["encryptedMessage"]["minItems"] = json!(1);
        schema["properties"]["encryptedMessage"]["maxItems"] = json!(24);
        let err = try_document_type_from_schema(schema).expect_err("should be refused");
        assert!(
            err.to_string()
                .contains("maxItems 24 is below the 32 bytes the ecdh-secp256k1-aes256-cbc"),
            "{err}"
        );
    }

    /// The key-id bounds are read from the schema, so a contract whose
    /// integers are not sized (config V0, or `sizedIntegerTypes: false`)
    /// can still declare the keyword.
    #[test]
    fn should_accept_encrypted_for_key_ids_when_sized_integer_types_are_off() {
        use crate::data_contract::config::v0::DataContractConfigV0;

        let platform_version = PlatformVersion::latest();
        let config = DataContractConfig::V0(DataContractConfigV0::default());
        let value = platform_value::to_value(encrypted_schema(encrypted_for_declaration()))
            .expect("schema should convert");

        let document_type = DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("should parse with unsized integers");

        assert!(matches!(
            document_type
                .as_ref()
                .flattened_properties()
                .get("recipientKeyId")
                .map(|p| &p.property_type),
            Some(DocumentPropertyType::I64)
        ));
        assert!(encrypted_for_of(&document_type, "encryptedMessage").is_some());
    }

    // ================================================================
    //  requiredSince
    // ================================================================

    #[test]
    fn should_parse_required_since_on_top_level_required_property() {
        let document_type = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60},
                "b": {"type": "string", "position": 1, "maxLength": 60, "requiredSince": 3},
            },
            "required": ["a", "b"],
            "additionalProperties": false
        }))
        .expect("should parse");

        let properties = document_type.as_ref().flattened_properties().clone();
        assert_eq!(properties.get("a").unwrap().required_since, None);
        assert_eq!(properties.get("b").unwrap().required_since, Some(3));
        assert!(properties.get("b").unwrap().required);
    }

    #[test]
    fn should_reject_required_since_on_optional_property() {
        let result = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60, "requiredSince": 2},
            },
            "required": [],
            "additionalProperties": false
        }));

        assert!(
            result.is_err(),
            "requiredSince on a property not listed in required must be rejected"
        );
    }

    #[test]
    fn should_reject_required_since_on_nested_property() {
        let result = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "outer": {
                    "type": "object",
                    "position": 0,
                    "properties": {
                        "inner": {"type": "string", "position": 0, "maxLength": 60, "requiredSince": 2},
                    },
                    "required": ["inner"],
                    "additionalProperties": false
                },
            },
            "required": [],
            "additionalProperties": false
        }));

        assert!(
            result.is_err(),
            "requiredSince on a nested property must be rejected"
        );
    }

    #[test]
    fn should_reject_required_since_above_u32_max() {
        // The meta-schema caps the value at u32::MAX too; this pins the
        // parser-side rejection so it does not depend on meta-schema
        // coverage (parses without full validation skip the meta-schema)
        let result = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60, "requiredSince": 4_294_967_296_u64},
            },
            "required": ["a"],
            "additionalProperties": false
        }));

        assert!(
            result.is_err(),
            "requiredSince above u32::MAX must be rejected"
        );
    }

    #[test]
    fn should_reject_required_since_of_zero() {
        let result = try_document_type_from_schema(json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60, "requiredSince": 0},
            },
            "required": ["a"],
            "additionalProperties": false
        }));

        assert!(
            result.is_err(),
            "requiredSince of 0 must be rejected (contract versions start at 1)"
        );
    }

    #[test]
    fn should_parse_required_since_reached_through_a_ref() {
        // A `$ref`'d property resolves to its `$defs` entry before keywords
        // are read, so an annotation hidden behind a reference is parsed
        // exactly like a direct one — any validation that only scans raw
        // property JSON would miss it, which is why the
        // `requiredSince <= contract version` invariant is enforced on
        // parsed properties (validate_required_since_within_contract_version)
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let schema_defs: BTreeMap<String, Value> = [(
            "annotated".to_string(),
            platform_value::to_value(json!({
                "type": "string", "maxLength": 60, "requiredSince": 2
            }))
            .expect("defs should convert"),
        )]
        .into_iter()
        .collect();

        let schema = platform_value::to_value(json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60},
                "b": {"$ref": "#/$defs/annotated", "position": 1},
            },
            "required": ["a", "b"],
            "additionalProperties": false
        }))
        .expect("schema should convert");

        let document_type = DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            schema,
            Some(&schema_defs),
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("should parse");

        let properties = document_type.as_ref().flattened_properties().clone();
        assert_eq!(properties.get("b").unwrap().required_since, Some(2));

        // The parsed-property invariant check sees the annotation the raw
        // JSON hides: version 1 (too old for requiredSince 2) rejects,
        // version 2 accepts
        let mut document_types = BTreeMap::new();
        document_types.insert("msg".to_string(), document_type);

        assert!(
            validate_required_since_within_contract_version(&document_types, 1).is_err(),
            "requiredSince 2 must be rejected on a version 1 contract even through $ref"
        );
        assert!(validate_required_since_within_contract_version(&document_types, 2).is_ok());
    }

    #[test]
    fn should_ignore_required_since_on_platform_versions_predating_it() {
        // Platform versions whose tables carry `apply_required_since: None`
        // predate the keyword: even if it appears in a schema they parse
        // (only possible without full validation — their meta-schemas reject
        // it), they must ignore it and keep producing the plain required
        // property they always produced.
        let platform_version = PlatformVersion::get(13).expect("platform version 13 should exist");

        let document_type = try_document_type_from_schema_on_version(
            json!({
                "type": "object",
                "properties": {
                    "a": {"type": "string", "position": 0, "maxLength": 60, "requiredSince": 3},
                },
                "required": ["a"],
                "additionalProperties": false
            }),
            platform_version,
        )
        .expect("a parse predating requiredSince should ignore the keyword entirely");

        let properties = document_type.as_ref().flattened_properties().clone();
        assert_eq!(properties.get("a").unwrap().required_since, None);
        assert!(properties.get("a").unwrap().required);
    }
    #[test]
    fn should_reject_time_range_on_user_defined_property() {
        // No user property type parses to a millisecond timestamp — `type:
        // "string"` with `format: "date-time"` stays `String` — so a
        // user-defined time-range source must be rejected rather than
        // accepted as an index that could never bucket anything meaningful.
        let err = try_document_type_from_schema_full_validation(json!({
            "type": "object",
            "properties": {
                "eventAt": {
                    "type": "string",
                    "maxLength": 63,
                    "position": 0
                }
            },
            "indices": [
                {
                    "name": "byEventTime",
                    "properties": [{ "eventAt": "asc" }],
                    "timeRange": { "on": "eventAt", "range": 21_600u64, "step": 7_200u64 }
                }
            ],
            "required": ["eventAt"],
            "additionalProperties": false
        }))
        .expect_err("a user-defined time-range source must be rejected");

        assert!(
            err.to_string().contains("system timestamps"),
            "expected the system-timestamp restriction, got: {err}"
        );
    }

    #[test]
    fn should_parse_time_range_on_required_system_timestamp() {
        try_document_type_from_schema_full_validation(json!({
            "type": "object",
            "properties": {
                "hashtag": {
                    "type": "string",
                    "maxLength": 63,
                    "position": 0
                }
            },
            "indices": [
                {
                    "name": "trending",
                    "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
                    "timeRange": { "on": "$createdAt", "range": 21_600u64, "step": 7_200u64 }
                }
            ],
            "required": ["$createdAt", "hashtag"],
            "additionalProperties": false
        }))
        .expect("a required system timestamp is the supported time-range source");
    }

    /// The overlap-factor cap is a versioned system limit
    /// (`SystemLimits::max_time_range_overlap_factor`), enforced at
    /// registration rather than at parse; both sides of the boundary are
    /// pinned here through the versioned dispatch. 24 is a day-long window
    /// sliding hourly — the natural worst case the cap is sized for.
    #[test]
    fn should_enforce_the_versioned_time_range_overlap_factor_cap() {
        let time_range_schema = |range_seconds: u64| {
            json!({
                "type": "object",
                "properties": {
                    "hashtag": {
                        "type": "string",
                        "maxLength": 63,
                        "position": 0
                    }
                },
                "indices": [
                    {
                        "name": "trending",
                        "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
                        "timeRange": { "on": "$createdAt", "range": range_seconds, "step": 3_600u64 }
                    }
                ],
                "required": ["$createdAt", "hashtag"],
                "additionalProperties": false
            })
        };

        try_document_type_from_schema_full_validation(time_range_schema(24 * 3_600))
            .expect("an overlap factor at the cap must register");

        let err = try_document_type_from_schema_full_validation(time_range_schema(25 * 3_600))
            .expect_err("an overlap factor over the cap must be rejected at registration");
        assert!(
            err.to_string().contains("overlap factor"),
            "expected the overlap-factor rejection, got: {err}"
        );
    }

    #[test]
    fn should_parse_unique_time_range_index_with_non_overlapping_windows_on_created_at() {
        // "one report per author per day": `range == step` makes the buckets a
        // partition, and `$createdAt` is immutable, which is exactly the pair
        // of conditions a unique bucketed index needs. Asserted through the
        // full-validation parse so the doctype-level checks (unique-index
        // limit, required system timestamp) run too.
        const ONE_DAY_SECONDS: u64 = 24 * 3_600;
        let document_type = try_document_type_from_schema_full_validation(json!({
            "type": "object",
            "properties": {
                "author": {
                    "type": "string",
                    "maxLength": 63,
                    "position": 0
                }
            },
            "indices": [
                {
                    "name": "dailyReport",
                    "properties": [{ "$createdAt": "asc" }, { "author": "asc" }],
                    "unique": true,
                    "timeRange": { "on": "$createdAt", "range": ONE_DAY_SECONDS, "step": ONE_DAY_SECONDS }
                }
            ],
            "required": ["$createdAt", "author"],
            "additionalProperties": false
        }))
        .expect("a non-overlapping $createdAt bucketing may be unique");

        let index = document_type
            .as_ref()
            .indexes()
            .get("dailyReport")
            .expect("the index should be registered")
            .clone();
        assert!(index.unique);
        assert_eq!(
            index
                .time_range
                .expect("the transform should survive the schema parse")
                .overlap_factor(),
            1
        );
    }

    /// A `score` type whose `amount` ends a `rangeSummable` index over
    /// `weight` and is bucketed by a unique `integerRange` index.
    fn range_summed_and_bucketed_schema(
        amount: serde_json::Value,
        weight: serde_json::Value,
    ) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "amount": amount,
                "weight": weight,
                "author": { "type": "string", "maxLength": 63, "position": 2 }
            },
            "indices": [
                {
                    "name": "byAmount",
                    "properties": [{ "amount": "asc" }],
                    "summable": "weight",
                    "rangeSummable": true
                },
                {
                    "name": "byAmountBand",
                    "properties": [{ "amount": "asc" }, { "author": "asc" }],
                    "unique": true,
                    "integerRange": { "on": "amount", "range": 100u64, "step": 100u64 }
                }
            ],
            "required": ["amount", "weight", "author"],
            "additionalProperties": false
        })
    }

    #[test]
    fn should_refuse_an_integer_range_on_an_unsigned_property_ending_a_range_summable_index() {
        let unsigned_amount =
            || json!({ "type": "integer", "minimum": 0, "maximum": 255, "position": 0 });
        let signed_weight =
            || json!({ "type": "integer", "minimum": -10, "maximum": 10, "position": 1 });
        for weight in [
            signed_weight(),
            json!({ "type": "integer", "minimum": 0, "maximum": 10, "position": 1 }),
        ] {
            let error = try_document_type_from_schema_full_validation(
                range_summed_and_bucketed_schema(unsigned_amount(), weight),
            )
            .expect_err("the amount keeps its earlier keys, which the grid does not read");
            assert!(
                error.to_string().contains("keeps its earlier key encoding"),
                "unexpected error: {error}"
            );
        }

        try_document_type_from_schema_full_validation(range_summed_and_bucketed_schema(
            json!({ "type": "integer", "minimum": -128, "maximum": 127, "position": 0 }),
            signed_weight(),
        ))
        .expect("a signed amount keeps its signed keys whatever the index sums");
    }

    /// A `listing` type with a required integer `price` (inferred from its
    /// bounds) and a `category`, bucketed by `integerRange` on `price`.
    fn price_band_schema(
        price: serde_json::Value,
        integer_range: serde_json::Value,
        required: serde_json::Value,
    ) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "price": price,
                "category": { "type": "string", "maxLength": 63, "position": 1 }
            },
            "indices": [
                {
                    "name": "byPriceBand",
                    "properties": [{ "price": "asc" }, { "category": "asc" }],
                    "integerRange": integer_range,
                    "countable": "countable"
                }
            ],
            "required": required,
            "additionalProperties": false
        })
    }

    fn price_band_index(document_type: &DocumentType) -> Index {
        document_type
            .as_ref()
            .indexes()
            .get("byPriceBand")
            .expect("the index should be registered")
            .clone()
    }

    #[test]
    fn should_resolve_the_integer_range_key_type_from_the_source_property() {
        for (price, expected) in [
            (
                json!({ "type": "integer", "minimum": 0, "maximum": 1_000, "position": 0 }),
                IntegerRangeKeyType::U16,
            ),
            (
                json!({ "type": "integer", "minimum": -100, "maximum": 100, "position": 0 }),
                IntegerRangeKeyType::I8,
            ),
            (
                json!({ "type": "integer", "minimum": 0, "position": 0 }),
                IntegerRangeKeyType::U64,
            ),
            (
                json!({ "type": "integer", "position": 0 }),
                IntegerRangeKeyType::I64,
            ),
        ] {
            let schema = price_band_schema(
                price,
                json!({ "on": "price", "range": 300u64, "step": 100u64 }),
                json!(["price", "category"]),
            );
            let document_type = try_document_type_from_schema_full_validation(schema.clone())
                .expect("an integer source should register");
            let transform = price_band_index(&document_type)
                .integer_range
                .expect("the transform should survive the schema parse");
            assert_eq!(transform.key_type, expected);

            // A contract loaded from state skips validation but must bucket
            // exactly as it did at registration.
            let platform_version = PlatformVersion::latest();
            let config = DataContractConfig::default_for_version(platform_version)
                .expect("config should build");
            let unvalidated = DocumentType::try_from_schema(
                Identifier::random(),
                0,
                config.version(),
                "msg",
                platform_value::to_value(schema).expect("schema should convert"),
                None,
                &BTreeMap::new(),
                &config,
                false,
                &mut vec![],
                platform_version,
            )
            .expect("the unvalidated parse should succeed");
            assert_eq!(
                price_band_index(&unvalidated)
                    .integer_range
                    .expect("transform set")
                    .key_type,
                expected
            );
        }
    }

    #[test]
    fn should_reject_integer_range_on_a_non_integer_property() {
        let err = try_document_type_from_schema_full_validation(price_band_schema(
            json!({ "type": "string", "maxLength": 20, "position": 0 }),
            json!({ "on": "price", "range": 100u64, "step": 100u64 }),
            json!(["price", "category"]),
        ))
        .expect_err("a string source must be rejected");
        assert!(
            err.to_string().contains("must name an integer property"),
            "{err}"
        );
    }

    #[test]
    fn should_reject_integer_range_on_a_system_property() {
        let err = try_document_type_from_schema_full_validation(json!({
            "type": "object",
            "properties": {
                "category": { "type": "string", "maxLength": 63, "position": 0 }
            },
            "indices": [
                {
                    "name": "byRevisionBand",
                    "properties": [{ "$createdAt": "asc" }, { "category": "asc" }],
                    "integerRange": { "on": "$createdAt", "range": 100u64, "step": 100u64 }
                }
            ],
            "required": ["$createdAt", "category"],
            "additionalProperties": false
        }))
        .expect_err("a system property source must be rejected");
        assert!(
            err.to_string().contains("must name an integer property"),
            "{err}"
        );
    }

    #[test]
    fn should_reject_integer_range_on_an_optional_property() {
        let err = try_document_type_from_schema_full_validation(price_band_schema(
            json!({ "type": "integer", "minimum": 0, "maximum": 1_000, "position": 0 }),
            json!({ "on": "price", "range": 100u64, "step": 100u64 }),
            json!(["category"]),
        ))
        .expect_err("an optional source must be rejected");
        assert!(err.to_string().contains("does not require"), "{err}");
    }

    #[test]
    fn should_enforce_the_versioned_overlap_factor_cap_on_integer_ranges() {
        let schema = |range: u64| {
            price_band_schema(
                json!({ "type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 0 }),
                json!({ "on": "price", "range": range, "step": 10u64 }),
                json!(["price", "category"]),
            )
        };
        try_document_type_from_schema_full_validation(schema(240))
            .expect("an overlap factor at the cap must register");
        let err = try_document_type_from_schema_full_validation(schema(250))
            .expect_err("an overlap factor over the cap must be rejected");
        assert!(err.to_string().contains("overlap factor"), "{err}");
    }

    /// A `listing` whose integerRange source sits inside the object `meta`;
    /// `meta_required` decides whether the object itself is required.
    fn nested_price_band_schema(meta_required: bool) -> serde_json::Value {
        let required = if meta_required {
            json!(["meta", "category"])
        } else {
            json!(["category"])
        };
        json!({
            "type": "object",
            "properties": {
                "meta": {
                    "type": "object",
                    "properties": {
                        "price": { "type": "integer", "minimum": 0, "maximum": 1_000, "position": 0 }
                    },
                    "required": ["price"],
                    "additionalProperties": false,
                    "position": 0
                },
                "category": { "type": "string", "maxLength": 63, "position": 1 }
            },
            "indices": [
                {
                    "name": "byPriceBand",
                    "properties": [{ "meta.price": "asc" }, { "category": "asc" }],
                    "integerRange": { "on": "meta.price", "range": 100u64, "step": 100u64 }
                }
            ],
            "required": required,
            "additionalProperties": false
        })
    }

    #[test]
    fn should_reject_integer_range_on_a_nested_property_under_an_optional_object() {
        let err = try_document_type_from_schema_full_validation(nested_price_band_schema(false))
            .expect_err("a document may omit the whole `meta` object");
        assert!(err.to_string().contains("sits inside \"meta\""), "{err}");

        try_document_type_from_schema_full_validation(nested_price_band_schema(true))
            .expect("a nested source under a required object is always present");
    }

    #[test]
    fn should_reject_an_integer_range_level_key_over_the_key_cap() {
        // 64 + 1 + 64 + 1 + 64 = 194 bytes of path, plus `#R#S#P` with
        // 19-20 digit numbers, is over 255 bytes.
        let segment = |c: char| std::iter::repeat_n(c, 64).collect::<String>();
        let (a, b, c) = (segment('a'), segment('b'), segment('c'));
        let source = format!("{a}.{b}.{c}");
        let schema = json!({
            "type": "object",
            "properties": {
                a.clone(): {
                    "type": "object",
                    "properties": {
                        b.clone(): {
                            "type": "object",
                            "properties": {
                                c.clone(): { "type": "integer", "minimum": 0, "position": 0 }
                            },
                            "required": [c],
                            "additionalProperties": false,
                            "position": 0
                        }
                    },
                    "required": [b.clone()],
                    "additionalProperties": false,
                    "position": 0
                }
            },
            "indices": [
                {
                    "name": "byBand",
                    "properties": [{ source.clone(): "asc" }],
                    "integerRange": {
                        "on": source,
                        "range": 10_000_000_000_000_000_000u64,
                        "step": 10_000_000_000_000_000_000u64,
                        "phase": 9_999_999_999_999_999_999u64
                    }
                }
            ],
            "required": [a],
            "additionalProperties": false
        });
        let err = try_document_type_from_schema_full_validation(schema)
            .expect_err("a 256-byte level key cannot be a GroveDB key");
        assert!(err.to_string().contains("key cap"), "{err}");
    }

    #[test]
    fn should_reject_integer_range_before_protocol_version_14() {
        let schema = price_band_schema(
            json!({ "type": "integer", "minimum": 0, "maximum": 1_000, "position": 0 }),
            json!({ "on": "price", "range": 100u64, "step": 100u64 }),
            json!(["price", "category"]),
        );
        try_document_type_from_schema_on_version(
            schema,
            PlatformVersion::get(13).expect("protocol version 13 exists"),
        )
        .expect_err("integerRange is not part of the protocol version 13 grammar");
    }

    // ================================================================
    //  required and transient entries of object members
    // ================================================================

    /// A document type with one object property, `profile`, whose own
    /// `required` list names its `name` member; `required` and `transient`
    /// are the document type's top-level lists.
    fn object_members_schema(required: &[&str], transient: &[&str]) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "profile": {
                    "type": "object",
                    "position": 0,
                    "properties": {
                        "name": {"type": "string", "position": 0, "maxLength": 60},
                        "bio": {"type": "string", "position": 1, "maxLength": 60},
                    },
                    "required": ["name"],
                    "additionalProperties": false
                },
            },
            "required": required,
            "transient": transient,
            "additionalProperties": false
        })
    }

    /// The members of the `profile` object of [`object_members_schema`].
    fn profile_members(document_type: &DocumentType) -> &IndexMap<String, DocumentProperty> {
        let DocumentPropertyType::Object(members) =
            &document_type.properties()["profile"].property_type
        else {
            panic!("profile should parse as an object");
        };
        members
    }

    #[test]
    fn should_match_required_entries_to_object_members_by_prefix() {
        for platform_version in [
            PlatformVersion::latest(),
            PlatformVersion::get(13).expect("platform version 13 should exist"),
        ] {
            let expected = try_document_type_from_schema_on_version(
                object_members_schema(&["profile"], &[]),
                platform_version,
            )
            .expect("should parse");
            let document_type = try_document_type_from_schema_on_version(
                object_members_schema(&["profile", "profileé"], &[]),
                platform_version,
            )
            .expect("an entry naming no member should parse");

            // Every member stays as the object's own list declares it
            let members = profile_members(&document_type);
            assert!(members["name"].required);
            assert!(!members["bio"].required);
            assert_eq!(document_type.properties(), expected.properties());
            assert_eq!(
                document_type.flattened_properties(),
                expected.flattened_properties()
            );
        }

        try_document_type_from_schema_full_validation(object_members_schema(
            &["profile", "profileé"],
            &[],
        ))
        .expect("an entry naming no member should pass full validation");
    }

    #[test]
    fn should_match_transient_entries_to_object_members_by_prefix() {
        for platform_version in [
            PlatformVersion::latest(),
            PlatformVersion::get(13).expect("platform version 13 should exist"),
        ] {
            let expected = try_document_type_from_schema_on_version(
                object_members_schema(&["profile"], &[]),
                platform_version,
            )
            .expect("should parse");
            let document_type = try_document_type_from_schema_on_version(
                object_members_schema(&["profile"], &["profileé"]),
                platform_version,
            )
            .expect("an entry naming no member should parse");

            let members = profile_members(&document_type);
            assert!(!members["name"].transient);
            assert!(!members["bio"].transient);
            assert_eq!(document_type.properties(), expected.properties());
            assert_eq!(
                document_type.flattened_properties(),
                expected.flattened_properties()
            );
        }

        // From protocol version 14 the validating parse holds every transient
        // entry to naming a top-level property
        let err = try_document_type_from_schema_full_validation(object_members_schema(
            &["profile"],
            &["profileé"],
        ))
        .expect_err("a transient entry naming no top-level property should be refused");
        assert!(
            err.to_string().contains("not a top-level property"),
            "expected the transient entry refusal, got: {err}"
        );
    }
}
