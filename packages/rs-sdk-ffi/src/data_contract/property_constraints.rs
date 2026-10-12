//! `propertyConstraints` rules (protocol version 14): the named conditions a
//! document type holds every created or replaced document's properties to.
//! Consensus refuses a document breaking one with
//! `DocumentPropertyConstraintViolatedError` (basic code 10422), and a refused
//! state transition is still paid for.
//!
//! Two functions expose what DPP already knows about them:
//!
//! - [`dash_sdk_data_contract_get_property_constraints`]: the rules a document
//!   type declares, in name order (the order consensus checks them in), each
//!   with what it reads, as a JSON array;
//! - [`dash_sdk_data_contract_check_property_constraints`]: the first rule a
//!   document to create breaks, judged by DPP's own
//!   `DocumentType::validate_property_constraints` (the check consensus runs,
//!   evaluating each rule with `PropertyConstraint::violation`), as JSON, or
//!   JSON `null` when it meets them all.
//!
//! The JSON shapes are those of wasm-dpp2's `documentTypePropertyConstraints`
//! and `checkDocumentPropertyConstraints`, key for key, so every SDK reports a
//! rule and a violation alike.
//!
//! Both take the contract as its platform serialization: the bytes a client
//! keeps beside a fetched contract, which it also hands
//! `dash_sdk_add_known_contracts`. They read it at the SDK's protocol version,
//! the version the SDK builds and sends documents at, so a client refreshing
//! that version sees the rules consensus applies: before protocol version 14 a
//! document type carries none.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::time::{SystemTime, UNIX_EPOCH};

use dash_sdk::dpp::consensus::basic::document::PropertyConstraintViolation;
use dash_sdk::dpp::consensus::basic::BasicError;
use dash_sdk::dpp::consensus::ConsensusError;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use dash_sdk::dpp::data_contract::document_type::methods::{
    DocumentTypeBasicMethods, DocumentTypeV0Methods,
};
use dash_sdk::dpp::data_contract::document_type::property_constraints::{
    AggregateKind, AggregateRead, DocumentSystemValues, PropertyRead,
};
use dash_sdk::dpp::data_contract::document_type::DocumentTypeRef;
use dash_sdk::dpp::document::{Document, DocumentV0Getters};
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::{DataContract, Identifier};
use dash_sdk::dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted;
use dash_sdk::dpp::version::PlatformVersion;
use serde_json::json;

use crate::document::{build_document_from_properties, parse_document_properties_json};
use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// The doctype keyword declaring the rules, whose entries are the rules as
/// the contract wrote them.
const PROPERTY_CONSTRAINTS_KEYWORD: &str = "propertyConstraints";

/// Get the `propertyConstraints` rules of a document type as a JSON array
///
/// Each element is
/// `{ "name": string, "rule": object, "reads": [{ "path": string, "kind": string }], "readsOwner": bool, "readsSystem": [string], "readsTotals": [{ "kind": string, "documentType": string, "property"?: string, "filter": [string] }] }`:
/// the rule's name (its key in `propertyConstraints`), the rule exactly as the
/// document type's schema declares it, every property it reads in declared
/// order (`kind` is `"value"` for an integer operand, `"presence"` for
/// `present` / `absent`, `"text"` for a string comparison, `"identifier"` for
/// an identifier comparison, `"length"` for a `length` or `byteLength` operand,
/// `"count"` for a `count` operand and `"elements"` for the array a `contains`
/// looks in; `$ownerId` is no property and is not listed), whether it reads `$ownerId` (or a
/// total that depends on the owner), which makes a transfer or a purchase answer to it too,
/// the system times and heights it reads (`"$createdAt"`, ...), which make a price update
/// answer to a rule reading the update's and a transfer or purchase one reading the
/// transfer's, and the `countOf` and `sumOf` totals it reads in declared order (`kind`
/// `"countOf"` or `"sumOf"`, the type of the contract it totals, the summed `property` of
/// a `sumOf` only, and the keys its `filter` matches documents by; a total the platform
/// reads from state when the document is sent, which the pre-check does not). Rules
/// are listed in name order, the order
/// consensus checks them in. A document type declaring none gives `[]`, and so
/// does every document type when the SDK's protocol version is below 14.
///
/// The contract is read from its platform serialization at the SDK's protocol
/// version, as `dash_sdk_add_known_contracts` reads it, without re-validating
/// it.
///
/// Errors: `InvalidParameter` for a null or empty argument or a document type
/// name that is not UTF-8, `SerializationError` for bytes that are not a
/// contract, `NotFound` for a document type the contract does not declare.
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - `serialized_contract` must point to `serialized_contract_len` readable bytes.
/// - `document_type` must point to a NUL-terminated C string valid for the duration of the call.
/// - On success the result's `data` is a heap-allocated C string the caller frees with
///   `dash_sdk_string_free`; on error the caller frees `error` with `dash_sdk_error_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_get_property_constraints(
    sdk_handle: *const SDKHandle,
    serialized_contract: *const u8,
    serialized_contract_len: usize,
    document_type: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || serialized_contract.is_null() || document_type.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, serialized contract or document type is null".to_string(),
        ));
    }

    // SAFETY: the caller guarantees `sdk_handle` points to a live SDKWrapper
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    // SAFETY: non-null, and the caller guarantees NUL termination
    let document_type_name = match CStr::from_ptr(document_type).to_str() {
        Ok(name) => name,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let rules = deserialize_contract(
        serialized_contract,
        serialized_contract_len,
        wrapper.sdk.version(),
    )
    .and_then(|contract| {
        let document_type = document_type_named(&contract, document_type_name)?;
        property_constraints_json(document_type)
    });
    json_result(rules)
}

/// Check a document to create against its document type's `propertyConstraints`
///
/// `properties_json` is the document's properties as `dash_sdk_document_create`
/// takes them (a JSON object keyed by property name, byte arrays as hex or
/// base64 and identifiers as base58 or hex strings), and `owner_id` the 32
/// bytes of the identity that will own it, which `$ownerId` reads. The
/// properties are turned into the document `dash_sdk_document_create` builds,
/// with the same parsing, sanitizing and `create_document_from_data`, so every
/// value is typed as it would be sent; the document is then judged by DPP's
/// `validate_property_constraints`, the check consensus runs on a create:
/// every rule, in name order, evaluated by `PropertyConstraint::violation`.
/// The device clock stands in for the block time the create records
/// (`$createdAt`, `$updatedAt`, `$transferredAt`), and a rule reading a block
/// height is not judged, since the height is unknown until the block, and
/// neither is a rule reading a `countOf` or `sumOf` total (`"readsTotals"`),
/// which the platform reads from state when the document is sent.
/// Nothing but the rules is checked: not the JSON schema, not the state.
///
/// The result is the first rule broken, as
/// `{ "rule": string, "violation": string, "message": string }`, with
/// `violation` one of `"NotMet"`, `"Overflow"`, `"DivisionByZero"`,
/// `"NegativeExponent"` and `"NotAnInteger"` and `message` the reason
/// consensus gives, or JSON `null` when the document meets every rule (always
/// so below protocol version 14).
///
/// The contract is read from its platform serialization at the SDK's protocol
/// version, as `dash_sdk_add_known_contracts` reads it, without re-validating
/// it.
///
/// Errors: `InvalidParameter` for a null or empty argument, text that is not
/// UTF-8, properties that are not a JSON object, or properties no document can
/// be built from; `SerializationError` for bytes that are not a contract;
/// `NotFound` for a document type the contract does not declare.
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - `serialized_contract` must point to `serialized_contract_len` readable bytes.
/// - `document_type` and `properties_json` must point to NUL-terminated C strings valid for the
///   duration of the call.
/// - `owner_id` must point to 32 readable bytes.
/// - On success the result's `data` is a heap-allocated C string the caller frees with
///   `dash_sdk_string_free`; on error the caller frees `error` with `dash_sdk_error_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_check_property_constraints(
    sdk_handle: *const SDKHandle,
    serialized_contract: *const u8,
    serialized_contract_len: usize,
    document_type: *const c_char,
    properties_json: *const c_char,
    owner_id: *const u8,
) -> DashSDKResult {
    if sdk_handle.is_null()
        || serialized_contract.is_null()
        || document_type.is_null()
        || properties_json.is_null()
        || owner_id.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, serialized contract, document type, properties JSON or owner ID is null"
                .to_string(),
        ));
    }

    // SAFETY: the caller guarantees `sdk_handle` points to a live SDKWrapper
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    // SAFETY: non-null, and the caller guarantees NUL termination
    let document_type_name = match CStr::from_ptr(document_type).to_str() {
        Ok(name) => name,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };
    // SAFETY: non-null, and the caller guarantees NUL termination
    let properties_str = match CStr::from_ptr(properties_json).to_str() {
        Ok(properties) => properties,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };
    // SAFETY: non-null, and the caller guarantees 32 readable bytes
    let owner_bytes = &*(owner_id as *const [u8; 32]);
    let owner_id = Identifier::new(*owner_bytes);

    // Read once, so the contract and the document are read at one version
    let platform_version = wrapper.sdk.version();
    let violation = parse_document_properties_json(properties_str).and_then(|properties| {
        let contract = deserialize_contract(
            serialized_contract,
            serialized_contract_len,
            platform_version,
        )?;
        let document_type = document_type_named(&contract, document_type_name)?;
        // The id is derived from the entropy, and no rule can read it
        let document = build_document_from_properties(
            document_type,
            properties,
            owner_id,
            [0u8; 32],
            platform_version,
        )
        .map_err(|e| {
            DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Failed to build the document from its properties: {}", e),
            )
        })?;
        property_constraint_violation_json(document_type, &document, platform_version)
    });
    json_result(violation)
}

/// Read the contract a caller holds as its platform serialization, at
/// `platform_version` (the SDK's) and without re-validating it: the way
/// `dash_sdk_add_known_contracts` and the token transitions read one.
///
/// # Safety
/// - `serialized_contract` must be non-null and point to `serialized_contract_len` readable bytes.
unsafe fn deserialize_contract(
    serialized_contract: *const u8,
    serialized_contract_len: usize,
    platform_version: &PlatformVersion,
) -> Result<DataContract, DashSDKError> {
    if serialized_contract_len == 0 {
        return Err(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Serialized contract is empty".to_string(),
        ));
    }
    // SAFETY: the caller guarantees `serialized_contract_len` readable bytes
    let bytes = std::slice::from_raw_parts(serialized_contract, serialized_contract_len);
    DataContract::versioned_deserialize_untrusted(bytes, false, platform_version).map_err(|e| {
        DashSDKError::new(
            DashSDKErrorCode::SerializationError,
            format!("Failed to deserialize contract: {}", e),
        )
    })
}

/// The document type `contract` declares under `name`.
fn document_type_named<'a>(
    contract: &'a DataContract,
    name: &str,
) -> Result<DocumentTypeRef<'a>, DashSDKError> {
    contract
        .document_type_optional_for_name(name)
        .ok_or_else(|| {
            DashSDKError::new(
                DashSDKErrorCode::NotFound,
                format!("Document type '{}' not found in the data contract", name),
            )
        })
}

/// Every rule of `document_type`'s `propertyConstraints`, in name order, as
/// the JSON array `dash_sdk_data_contract_get_property_constraints` returns.
///
/// The parsed rules give the name, the reads and whether the owner is read;
/// the rule itself is the schema's declaration, what the contract wrote.
fn property_constraints_json(
    document_type: DocumentTypeRef<'_>,
) -> Result<serde_json::Value, DashSDKError> {
    let declarations = document_type
        .schema()
        .get_optional_value(PROPERTY_CONSTRAINTS_KEYWORD)
        .ok()
        .flatten();

    let constraints = document_type.property_constraints();
    let mut rules = Vec::with_capacity(constraints.len());
    for (name, constraint) in constraints {
        let declared = declarations
            .and_then(|declarations| declarations.get_optional_value(name).ok().flatten())
            .ok_or_else(|| {
                DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!(
                        "The propertyConstraints rule '{}' is missing from the document type's schema",
                        name
                    ),
                )
            })?;
        let rule = serde_json::to_value(declared).map_err(|e| {
            DashSDKError::new(
                DashSDKErrorCode::SerializationError,
                format!(
                    "Failed to serialize the propertyConstraints rule '{}': {}",
                    name, e
                ),
            )
        })?;
        let reads: Vec<serde_json::Value> = constraint
            .property_reads()
            .into_iter()
            .map(|(path, read)| json!({ "path": path, "kind": read_kind_name(read) }))
            .collect();
        rules.push(json!({
            "name": name,
            "rule": rule,
            "reads": reads,
            "readsOwner": constraint.reads_owner(),
            "readsSystem": constraint
                .system_reads()
                .into_iter()
                .map(|property| property.name())
                .collect::<Vec<_>>(),
            "readsTotals": constraint
                .aggregate_reads()
                .into_iter()
                .map(total_read_json)
                .collect::<Vec<_>>(),
        }));
    }
    Ok(serde_json::Value::Array(rules))
}

/// A `countOf` or `sumOf` a rule reads, as the descriptor lists it: its operator,
/// the type it totals, the summed property of a `sumOf`, and the keys of its
/// filter.
fn total_read_json(read: &AggregateRead) -> serde_json::Value {
    let mut entry = json!({
        "kind": read.wire_name(),
        "documentType": read.document_type,
        "filter": read.filter.keys().collect::<Vec<_>>(),
    });
    if let AggregateKind::Sum { property } = &read.kind {
        entry["property"] = json!(property);
    }
    entry
}

/// The system values a create of a document owned by `owner_id` will have, as
/// far as a client can tell before its block: the device clock stands in for
/// the block time it records as its creation, update and transfer, and the
/// block heights are unknown until the block, so a rule reading one is not
/// judged.
fn system_values_for_create(owner_id: Identifier) -> DocumentSystemValues {
    // A clock before the epoch reads as the epoch; milliseconds fit a `u64`
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);
    DocumentSystemValues {
        created_at: Some(now),
        updated_at: Some(now),
        transferred_at: Some(now),
        ..DocumentSystemValues::owned_by(owner_id)
    }
}

/// The first rule of `document_type`'s `propertyConstraints` that `document`
/// breaks, judged as consensus judges a create (its properties with every
/// `generatedFrom` property generated from its params, as the transition
/// builders send them, its owner for `$ownerId`, and the system times
/// [`system_values_for_create`] estimates), as the JSON
/// `dash_sdk_data_contract_check_property_constraints` returns: JSON `null`
/// when it meets them all.
fn property_constraint_violation_json(
    document_type: DocumentTypeRef<'_>,
    document: &Document,
    platform_version: &PlatformVersion,
) -> Result<serde_json::Value, DashSDKError> {
    let mut properties = document.properties().clone();
    document_type
        .regenerate_generated_properties(&mut properties, platform_version)
        .map_err(|e| {
            DashSDKError::new(
                DashSDKErrorCode::ProtocolError,
                format!("Failed to generate the generatedFrom properties: {}", e),
            )
        })?;
    let data = Value::from(properties);
    let system = system_values_for_create(document.owner_id());
    let result = document_type
        .validate_property_constraints(&data, &system, platform_version)
        .map_err(|e| {
            DashSDKError::new(
                DashSDKErrorCode::ProtocolError,
                format!("Failed to check the propertyConstraints rules: {}", e),
            )
        })?;
    match result.first_error() {
        None => Ok(serde_json::Value::Null),
        Some(ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(
            error,
        ))) => Ok(json!({
            "rule": error.constraint(),
            "violation": violation_name(error.violation()),
            "message": error.violation().to_string(),
        })),
        Some(other) => Err(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!(
                "Unexpected error checking the propertyConstraints rules: {}",
                other
            ),
        )),
    }
}

/// The name a read kind goes by in the rule JSON.
fn read_kind_name(read: PropertyRead) -> &'static str {
    match read {
        PropertyRead::Value => "value",
        PropertyRead::Presence => "presence",
        PropertyRead::Text => "text",
        PropertyRead::Identifier => "identifier",
        PropertyRead::Length => "length",
        PropertyRead::Count => "count",
        PropertyRead::Elements(_) => "elements",
    }
}

/// The name a violation goes by in the violation JSON, the variant's own.
fn violation_name(violation: PropertyConstraintViolation) -> &'static str {
    match violation {
        PropertyConstraintViolation::NotMet => "NotMet",
        PropertyConstraintViolation::Overflow => "Overflow",
        PropertyConstraintViolation::DivisionByZero => "DivisionByZero",
        PropertyConstraintViolation::NegativeExponent => "NegativeExponent",
        PropertyConstraintViolation::NotAnInteger => "NotAnInteger",
    }
}

/// `value` as a C string result the caller frees with `dash_sdk_string_free`.
fn json_result(value: Result<serde_json::Value, DashSDKError>) -> DashSDKResult {
    let value = match value {
        Ok(value) => value,
        Err(error) => return DashSDKResult::error(error),
    };
    // JSON escapes every control character, so the text holds no NUL byte
    match CString::new(value.to_string()) {
        Ok(text) => DashSDKResult::success_string(text.into_raw()),
        Err(e) => DashSDKResult::error(FFIError::from(e).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_sdk_error_free;
    use crate::test_utils::test_utils::destroy_mock_sdk_handle;
    use crate::types::dash_sdk_string_free;
    use dash_sdk::dpp::data_contract::DataContractFactory;
    use dash_sdk::dpp::platform_value::platform_value;
    use dash_sdk::dpp::platform_value::string_encoding::Encoding;
    use dash_sdk::dpp::serialization::PlatformSerializableWithPlatformVersion;
    use dash_sdk::SdkBuilder;

    const OWNER: [u8; 32] = [1; 32];
    const OTHER: [u8; 32] = [2; 32];

    /// The rules of the `offer` type, as declared: one of each family (an
    /// `ifThen` over a string constant and `present`, an integer comparison, a
    /// division, `$ownerId` with `absent`, and `in`), in a declaration order that
    /// is not name order.
    fn offer_rules() -> serde_json::Value {
        json!({
            "tieredFee": { "in": ["fee", [1, 10, 25]] },
            "discountBelowPrice": { "lessThan": ["discount", "price"] },
            "sellerIsOwner": {
                "anyOf": [{ "absent": "sellerId" }, { "equal": ["sellerId", "$ownerId"] }]
            },
            "closedNeedsClosedAt": {
                "ifThen": [
                    { "equal": ["status", { "const": "closed" }] },
                    { "present": "closedAt" }
                ]
            },
            "perUnitFee": { "greaterThanOrEqual": [{ "divide": ["price", "fee"] }, 1] }
        })
    }

    /// An `offer` type declaring `offer_rules()`, beside a `plain` type
    /// declaring none, created at `platform_version`.
    fn contract(platform_version: &PlatformVersion) -> DataContract {
        let rules = Value::from(offer_rules());
        let documents = platform_value!({
            "offer": {
                "type": "object",
                "properties": {
                    "price": { "type": "integer", "minimum": 0, "position": 0 },
                    "fee": { "type": "integer", "minimum": 0, "position": 1 },
                    "discount": { "type": "integer", "minimum": 0, "position": 2 },
                    "status": {
                        "type": "string",
                        "enum": ["open", "closed"],
                        "maxLength": 10,
                        "position": 3
                    },
                    "closedAt": { "type": "integer", "minimum": 0, "position": 4 },
                    "sellerId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 5
                    }
                },
                "required": ["price", "fee"],
                "additionalProperties": false,
                "propertyConstraints": rules
            },
            "plain": {
                "type": "object",
                "properties": {
                    "message": { "type": "string", "maxLength": 64, "position": 0 }
                },
                "additionalProperties": false
            }
        });

        let factory = DataContractFactory::new(platform_version.protocol_version)
            .expect("factory for the protocol version");
        factory
            .create_with_value_config(Identifier::new(OWNER), 1, documents, None, None)
            .expect("offer contract")
            .data_contract()
            .clone()
    }

    fn serialized_contract(platform_version: &PlatformVersion) -> Vec<u8> {
        contract(platform_version)
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("serialized contract")
    }

    /// A mock SDK handle pinned to `platform_version`.
    fn sdk_handle(platform_version: &'static PlatformVersion) -> *mut SDKHandle {
        let mut wrapper = SDKWrapper::new_mock();
        wrapper.sdk = SdkBuilder::new_mock()
            .with_version(platform_version)
            .build()
            .expect("mock SDK");
        Box::into_raw(Box::new(wrapper)) as *mut SDKHandle
    }

    /// The result's string, or its error code and message; frees both.
    fn take(result: DashSDKResult) -> Result<serde_json::Value, (DashSDKErrorCode, String)> {
        unsafe {
            if !result.error.is_null() {
                let error = &*result.error;
                let outcome = (
                    error.code,
                    CStr::from_ptr(error.message).to_string_lossy().into_owned(),
                );
                dash_sdk_error_free(result.error);
                return Err(outcome);
            }
            let text = result.data as *mut c_char;
            let value =
                serde_json::from_str(&CStr::from_ptr(text).to_string_lossy()).expect("JSON result");
            dash_sdk_string_free(text);
            Ok(value)
        }
    }

    fn rules_of(
        sdk: *mut SDKHandle,
        contract: &[u8],
        document_type: &str,
    ) -> Result<serde_json::Value, (DashSDKErrorCode, String)> {
        let document_type = CString::new(document_type).expect("no NUL in the type name");
        take(unsafe {
            dash_sdk_data_contract_get_property_constraints(
                sdk,
                contract.as_ptr(),
                contract.len(),
                document_type.as_ptr(),
            )
        })
    }

    fn check(
        sdk: *mut SDKHandle,
        contract: &[u8],
        document_type: &str,
        properties: serde_json::Value,
        owner: [u8; 32],
    ) -> Result<serde_json::Value, (DashSDKErrorCode, String)> {
        check_json(sdk, contract, document_type, &properties.to_string(), owner)
    }

    fn check_json(
        sdk: *mut SDKHandle,
        contract: &[u8],
        document_type: &str,
        properties_json: &str,
        owner: [u8; 32],
    ) -> Result<serde_json::Value, (DashSDKErrorCode, String)> {
        let document_type = CString::new(document_type).expect("no NUL in the type name");
        let properties_json = CString::new(properties_json).expect("no NUL in the JSON");
        take(unsafe {
            dash_sdk_data_contract_check_property_constraints(
                sdk,
                contract.as_ptr(),
                contract.len(),
                document_type.as_ptr(),
                properties_json.as_ptr(),
                owner.as_ptr(),
            )
        })
    }

    fn base58(bytes: [u8; 32]) -> String {
        Identifier::new(bytes).to_string(Encoding::Base58)
    }

    #[test]
    fn should_list_every_rule_in_name_order_with_what_it_reads() {
        let platform_version = PlatformVersion::latest();
        let sdk = sdk_handle(platform_version);
        let contract = serialized_contract(platform_version);
        let declared = offer_rules();

        let rules = rules_of(sdk, &contract, "offer");
        destroy_mock_sdk_handle(sdk);

        assert_eq!(
            rules.expect("rules of offer"),
            json!([
                {
                    "name": "closedNeedsClosedAt",
                    "rule": declared["closedNeedsClosedAt"],
                    "reads": [
                        { "path": "status", "kind": "text" },
                        { "path": "closedAt", "kind": "presence" }
                    ],
                    "readsOwner": false,
                    "readsSystem": [],
                    "readsTotals": []
                },
                {
                    "name": "discountBelowPrice",
                    "rule": declared["discountBelowPrice"],
                    "reads": [
                        { "path": "discount", "kind": "value" },
                        { "path": "price", "kind": "value" }
                    ],
                    "readsOwner": false,
                    "readsSystem": [],
                    "readsTotals": []
                },
                {
                    "name": "perUnitFee",
                    "rule": declared["perUnitFee"],
                    "reads": [
                        { "path": "price", "kind": "value" },
                        { "path": "fee", "kind": "value" }
                    ],
                    "readsOwner": false,
                    "readsSystem": [],
                    "readsTotals": []
                },
                {
                    "name": "sellerIsOwner",
                    "rule": declared["sellerIsOwner"],
                    "reads": [
                        { "path": "sellerId", "kind": "presence" },
                        { "path": "sellerId", "kind": "identifier" }
                    ],
                    "readsOwner": true,
                    "readsSystem": [],
                    "readsTotals": []
                },
                {
                    "name": "tieredFee",
                    "rule": declared["tieredFee"],
                    "reads": [{ "path": "fee", "kind": "value" }],
                    "readsOwner": false,
                    "readsSystem": [],
                    "readsTotals": []
                }
            ])
        );
    }

    #[test]
    fn should_list_no_rules_for_a_document_type_declaring_none() {
        let platform_version = PlatformVersion::latest();
        let sdk = sdk_handle(platform_version);
        let contract = serialized_contract(platform_version);

        let rules = rules_of(sdk, &contract, "plain");
        destroy_mock_sdk_handle(sdk);

        assert_eq!(rules.expect("rules of plain"), json!([]));
    }

    #[test]
    fn should_refuse_a_document_type_the_contract_does_not_declare() {
        let platform_version = PlatformVersion::latest();
        let sdk = sdk_handle(platform_version);
        let contract = serialized_contract(platform_version);

        let rules = rules_of(sdk, &contract, "letter");
        let violation = check(sdk, &contract, "letter", json!({}), OWNER);
        destroy_mock_sdk_handle(sdk);

        for outcome in [rules, violation] {
            let (code, message) = outcome.expect_err("an unknown type is refused");
            assert_eq!(code, DashSDKErrorCode::NotFound);
            assert!(message.contains("'letter' not found"), "{message}");
        }
    }

    #[test]
    fn should_report_nothing_for_a_document_meeting_every_rule() {
        let platform_version = PlatformVersion::latest();
        let sdk = sdk_handle(platform_version);
        let contract = serialized_contract(platform_version);

        let bare = check(
            sdk,
            &contract,
            "offer",
            json!({ "price": 100, "fee": 10 }),
            OWNER,
        );
        // The seller arrives as base58 text and is typed an identifier, as the
        // create path types it, so it equals the owner
        let full = check(
            sdk,
            &contract,
            "offer",
            json!({
                "price": 100,
                "fee": 10,
                "discount": 5,
                "status": "closed",
                "closedAt": 1000,
                "sellerId": base58(OWNER)
            }),
            OWNER,
        );
        let plain = check(sdk, &contract, "plain", json!({ "message": "hi" }), OWNER);
        destroy_mock_sdk_handle(sdk);

        assert_eq!(bare.expect("checked"), serde_json::Value::Null);
        assert_eq!(full.expect("checked"), serde_json::Value::Null);
        assert_eq!(plain.expect("checked"), serde_json::Value::Null);
    }

    #[test]
    fn should_report_the_first_rule_broken_in_name_order() {
        let platform_version = PlatformVersion::latest();
        let sdk = sdk_handle(platform_version);
        let contract = serialized_contract(platform_version);
        let violation_of = |properties: serde_json::Value| {
            let mut document = json!({ "price": 100, "fee": 10 });
            for (key, value) in properties.as_object().expect("an object") {
                document[key] = value.clone();
            }
            check(sdk, &contract, "offer", document, OWNER).expect("checked")
        };

        // A string constant: a closed offer must say when it closed
        let closed = violation_of(json!({ "status": "closed" }));
        // An integer comparison
        let discounted = violation_of(json!({ "discount": 200 }));
        // A fee of 0 divides by zero before the `in` rule is reached
        let free = violation_of(json!({ "fee": 0 }));
        // `in`
        let untiered = violation_of(json!({ "fee": 5 }));
        // Two broken rules: the first in name order is reported
        let both = violation_of(json!({ "discount": 200, "fee": 5 }));
        destroy_mock_sdk_handle(sdk);

        assert_eq!(
            closed,
            json!({
                "rule": "closedNeedsClosedAt",
                "violation": "NotMet",
                "message": PropertyConstraintViolation::NotMet.to_string()
            })
        );
        assert_eq!(discounted["rule"], "discountBelowPrice");
        assert_eq!(discounted["violation"], "NotMet");
        assert_eq!(
            free,
            json!({
                "rule": "perUnitFee",
                "violation": "DivisionByZero",
                "message": PropertyConstraintViolation::DivisionByZero.to_string()
            })
        );
        assert_eq!(untiered["rule"], "tieredFee");
        assert_eq!(untiered["violation"], "NotMet");
        assert_eq!(both["rule"], "discountBelowPrice");
    }

    #[test]
    fn should_read_the_owner_for_owner_id() {
        let platform_version = PlatformVersion::latest();
        let sdk = sdk_handle(platform_version);
        let contract = serialized_contract(platform_version);
        let offer = json!({ "price": 100, "fee": 10, "sellerId": base58(OTHER) });

        let owned_by_someone_else = check(sdk, &contract, "offer", offer.clone(), OWNER);
        let owned_by_the_seller = check(sdk, &contract, "offer", offer, OTHER);
        destroy_mock_sdk_handle(sdk);

        let violation = owned_by_someone_else.expect("checked");
        assert_eq!(violation["rule"], "sellerIsOwner");
        assert_eq!(violation["violation"], "NotMet");
        assert_eq!(
            owned_by_the_seller.expect("checked"),
            serde_json::Value::Null
        );
    }

    /// Parsers before protocol version 14 ignore the keyword, so an SDK at such
    /// a version reports no rule and no violation: exactly what consensus
    /// enforced there. The bytes are those a network at 14 returns; the
    /// contract cannot be created at 13, whose meta-schema refuses the keyword
    /// when JSON schema validation is compiled in.
    #[test]
    fn should_report_no_rules_below_protocol_version_14() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        let sdk = sdk_handle(platform_version);
        let contract = serialized_contract(PlatformVersion::latest());

        let rules = rules_of(sdk, &contract, "offer");
        let violation = check(
            sdk,
            &contract,
            "offer",
            json!({ "price": 100, "fee": 0, "discount": 200 }),
            OWNER,
        );
        destroy_mock_sdk_handle(sdk);

        assert_eq!(rules.expect("rules of offer"), json!([]));
        assert_eq!(violation.expect("checked"), serde_json::Value::Null);
    }

    #[test]
    fn should_refuse_what_is_not_a_contract_or_a_properties_object() {
        let platform_version = PlatformVersion::latest();
        let sdk = sdk_handle(platform_version);
        let contract = serialized_contract(platform_version);

        let not_a_contract = rules_of(sdk, &[0xff, 0x00, 0x13], "offer");
        let not_json = check_json(sdk, &contract, "offer", "{price:", OWNER);
        let not_an_object = check_json(sdk, &contract, "offer", "[1, 2]", OWNER);
        destroy_mock_sdk_handle(sdk);

        assert_eq!(
            not_a_contract.expect_err("refused").0,
            DashSDKErrorCode::SerializationError
        );
        let (code, message) = not_json.expect_err("refused");
        assert_eq!(code, DashSDKErrorCode::InvalidParameter);
        assert!(message.contains("Invalid properties JSON"), "{message}");
        let (code, message) = not_an_object.expect_err("refused");
        assert_eq!(code, DashSDKErrorCode::InvalidParameter);
        assert!(
            message.contains("Failed to convert properties"),
            "{message}"
        );
    }

    #[test]
    fn should_refuse_null_or_empty_arguments() {
        let platform_version = PlatformVersion::latest();
        let sdk = sdk_handle(platform_version);
        let contract = serialized_contract(platform_version);
        let offer = CString::new("offer").expect("no NUL");
        let properties = CString::new("{}").expect("no NUL");

        let outcomes = unsafe {
            [
                take(dash_sdk_data_contract_get_property_constraints(
                    std::ptr::null(),
                    contract.as_ptr(),
                    contract.len(),
                    offer.as_ptr(),
                )),
                take(dash_sdk_data_contract_get_property_constraints(
                    sdk,
                    contract.as_ptr(),
                    0,
                    offer.as_ptr(),
                )),
                take(dash_sdk_data_contract_check_property_constraints(
                    sdk,
                    contract.as_ptr(),
                    contract.len(),
                    offer.as_ptr(),
                    properties.as_ptr(),
                    std::ptr::null(),
                )),
                take(dash_sdk_data_contract_check_property_constraints(
                    sdk,
                    std::ptr::null(),
                    contract.len(),
                    offer.as_ptr(),
                    properties.as_ptr(),
                    OWNER.as_ptr(),
                )),
            ]
        };
        destroy_mock_sdk_handle(sdk);

        for outcome in outcomes {
            assert_eq!(
                outcome.expect_err("refused").0,
                DashSDKErrorCode::InvalidParameter
            );
        }
    }

    /// A `listing` type recording its creation time and block height, with a
    /// rule on each: it ends after its creation, and is listed from block 10 on.
    fn timed_contract_bytes() -> Vec<u8> {
        let platform_version = PlatformVersion::latest();
        let documents = platform_value!({
            "listing": {
                "type": "object",
                "properties": {
                    "endsAt": { "type": "integer", "minimum": 0, "position": 0 }
                },
                "required": ["endsAt", "$createdAt", "$createdAtBlockHeight"],
                "additionalProperties": false,
                "propertyConstraints": {
                    "endsAfterCreation": { "greaterThan": ["endsAt", "$createdAt"] },
                    "listedAfterHeight10": {
                        "greaterThanOrEqual": ["$createdAtBlockHeight", 10]
                    }
                }
            }
        });
        DataContractFactory::new(platform_version.protocol_version)
            .expect("factory for the protocol version")
            .create_with_value_config(Identifier::new(OWNER), 1, documents, None, None)
            .expect("listing contract")
            .data_contract()
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("serialized contract")
    }

    /// The pre-check reads the device clock for the times a create records, and
    /// leaves a rule reading a block height unjudged, since the height is unknown
    /// until the block.
    #[test]
    fn should_read_the_clock_for_system_times_and_skip_block_heights() {
        let sdk = sdk_handle(PlatformVersion::latest());
        let contract = timed_contract_bytes();

        let rules = rules_of(sdk, &contract, "listing");
        // Ended in 1970, before any create today
        let ended = check(sdk, &contract, "listing", json!({ "endsAt": 1 }), OWNER);
        // Ends in 2100; the height rule, which it would not meet at block 0, is skipped
        let open = check(
            sdk,
            &contract,
            "listing",
            json!({ "endsAt": 4_102_444_800_000u64 }),
            OWNER,
        );
        destroy_mock_sdk_handle(sdk);

        let rules = rules.expect("rules of listing");
        assert_eq!(rules[0]["name"], "endsAfterCreation");
        assert_eq!(rules[0]["readsSystem"], json!(["$createdAt"]));
        assert_eq!(
            rules[0]["reads"],
            json!([{ "path": "endsAt", "kind": "value" }])
        );
        assert_eq!(rules[1]["readsSystem"], json!(["$createdAtBlockHeight"]));
        assert_eq!(rules[1]["reads"], json!([]));

        let ended = ended.expect("checked");
        assert_eq!(ended["rule"], "endsAfterCreation");
        assert_eq!(ended["violation"], "NotMet");
        assert_eq!(open.expect("checked"), serde_json::Value::Null);
    }

    /// A `deal` type with two `ifThenElse` rules: `feePerPrice` (with a fee, the
    /// price is at least ten times it; without one, at most 100) and
    /// `openEndedSoldByOwner` (a deal without an end is sold by its owner, one
    /// with an end ends after its creation).
    fn branching_contract_bytes() -> Vec<u8> {
        let platform_version = PlatformVersion::latest();
        let documents = platform_value!({
            "deal": {
                "type": "object",
                "properties": {
                    "price": { "type": "integer", "minimum": 0, "position": 0 },
                    "fee": { "type": "integer", "minimum": 0, "position": 1 },
                    "endsAt": { "type": "integer", "minimum": 0, "position": 2 },
                    "sellerId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 3
                    }
                },
                "required": ["price", "fee", "$createdAt"],
                "additionalProperties": false,
                "propertyConstraints": {
                    "feePerPrice": {
                        "ifThenElse": [
                            { "greaterThan": ["fee", 0] },
                            { "greaterThanOrEqual": [{ "divide": ["price", "fee"] }, 10] },
                            { "lessThanOrEqual": ["price", 100] }
                        ]
                    },
                    "openEndedSoldByOwner": {
                        "ifThenElse": [
                            { "absent": "endsAt" },
                            { "equal": ["sellerId", "$ownerId"] },
                            { "greaterThan": ["endsAt", "$createdAt"] }
                        ]
                    }
                }
            }
        });
        DataContractFactory::new(platform_version.protocol_version)
            .expect("factory for the protocol version")
            .create_with_value_config(Identifier::new(OWNER), 1, documents, None, None)
            .expect("deal contract")
            .data_contract()
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("serialized contract")
    }

    /// An `ifThenElse` reports what every branch reads, the owner and the
    /// system times included, and the pre-check judges only the branch its
    /// condition takes: with no fee, `feePerPrice`'s division by zero is never
    /// reached.
    #[test]
    fn should_report_every_branch_of_an_if_then_else_and_judge_the_one_taken() {
        let sdk = sdk_handle(PlatformVersion::latest());
        let contract = branching_contract_bytes();
        // Open-ended and sold by its owner unless stated, so
        // `openEndedSoldByOwner` holds
        let violation_of = |properties: serde_json::Value| {
            let mut document = json!({ "price": 100, "fee": 10, "sellerId": base58(OWNER) });
            for (key, value) in properties.as_object().expect("an object") {
                document[key] = value.clone();
            }
            check(sdk, &contract, "deal", document, OWNER).expect("checked")
        };

        let rules = rules_of(sdk, &contract, "deal");
        let met = violation_of(json!({}));
        let fee_too_high = violation_of(json!({ "fee": 20 }));
        let no_fee = violation_of(json!({ "fee": 0 }));
        let no_fee_too_dear = violation_of(json!({ "price": 101, "fee": 0 }));
        let sold_by_other = violation_of(json!({ "sellerId": base58(OTHER) }));
        let ended = violation_of(json!({ "endsAt": 1 }));
        // Ends in 2100: the else branch is taken, and the seller is not judged
        let ending_sold_by_other = violation_of(json!({
            "endsAt": 4_102_444_800_000u64,
            "sellerId": base58(OTHER)
        }));
        destroy_mock_sdk_handle(sdk);

        assert_eq!(
            rules.expect("rules of deal"),
            json!([
                {
                    "name": "feePerPrice",
                    "rule": {
                        "ifThenElse": [
                            { "greaterThan": ["fee", 0] },
                            { "greaterThanOrEqual": [{ "divide": ["price", "fee"] }, 10] },
                            { "lessThanOrEqual": ["price", 100] }
                        ]
                    },
                    "reads": [
                        { "path": "fee", "kind": "value" },
                        { "path": "price", "kind": "value" },
                        { "path": "fee", "kind": "value" },
                        { "path": "price", "kind": "value" }
                    ],
                    "readsOwner": false,
                    "readsSystem": [],
                    "readsTotals": []
                },
                {
                    "name": "openEndedSoldByOwner",
                    "rule": {
                        "ifThenElse": [
                            { "absent": "endsAt" },
                            { "equal": ["sellerId", "$ownerId"] },
                            { "greaterThan": ["endsAt", "$createdAt"] }
                        ]
                    },
                    "reads": [
                        { "path": "endsAt", "kind": "presence" },
                        { "path": "sellerId", "kind": "identifier" },
                        { "path": "endsAt", "kind": "value" }
                    ],
                    "readsOwner": true,
                    "readsSystem": ["$createdAt"],
                    "readsTotals": []
                }
            ])
        );

        assert_eq!(met, serde_json::Value::Null);
        // The then branch: 100 / 20 is below 10
        assert_eq!(fee_too_high["rule"], "feePerPrice");
        assert_eq!(fee_too_high["violation"], "NotMet");
        // The else branch: no division, so no division by zero
        assert_eq!(no_fee, serde_json::Value::Null);
        assert_eq!(no_fee_too_dear["rule"], "feePerPrice");
        assert_eq!(no_fee_too_dear["violation"], "NotMet");
        // Without an end, the owner must be the seller
        assert_eq!(sold_by_other["rule"], "openEndedSoldByOwner");
        assert_eq!(sold_by_other["violation"], "NotMet");
        // With one, it must come after the create, timed by the device clock
        assert_eq!(ended["rule"], "openEndedSoldByOwner");
        assert_eq!(ended["violation"], "NotMet");
        assert_eq!(ending_sold_by_other, serde_json::Value::Null);
    }

    /// A `handle` type whose `normalizedLabel` is generated from `label`, with a
    /// rule reserving the normalized name `dash`.
    fn handle_contract_bytes() -> Vec<u8> {
        let platform_version = PlatformVersion::latest();
        let documents = platform_value!({
            "handle": {
                "type": "object",
                "properties": {
                    "label": { "type": "string", "maxLength": 63, "position": 0 },
                    "normalizedLabel": {
                        "type": "string",
                        "maxLength": 63,
                        "position": 1,
                        "generatedFrom": {
                            "function": "sys.stringTransformations.homographSafeASCII",
                            "params": ["label"]
                        }
                    }
                },
                "additionalProperties": false,
                "propertyConstraints": {
                    "notReserved": { "notEqual": ["normalizedLabel", { "const": "dash" }] }
                }
            }
        });
        DataContractFactory::new(platform_version.protocol_version)
            .expect("factory for the protocol version")
            .create_with_value_config(Identifier::new(OWNER), 1, documents, None, None)
            .expect("handle contract")
            .data_contract()
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("serialized contract")
    }

    /// The pre-check judges the rules on the document the transition builders
    /// send: a `generatedFrom` property left out, or stale, is generated from
    /// its params first, as consensus sees it.
    #[test]
    fn should_judge_a_generated_property_as_the_builders_send_it() {
        let sdk = sdk_handle(PlatformVersion::latest());
        let contract = handle_contract_bytes();

        let left_out = check(sdk, &contract, "handle", json!({ "label": "DASH" }), OWNER);
        let stale = check(
            sdk,
            &contract,
            "handle",
            json!({ "label": "DASH", "normalizedLabel": "b0b" }),
            OWNER,
        );
        let other = check(sdk, &contract, "handle", json!({ "label": "Bob" }), OWNER);
        destroy_mock_sdk_handle(sdk);

        for result in [left_out, stale] {
            let result = result.expect("checked");
            assert_eq!(result["rule"], "notReserved");
            assert_eq!(result["violation"], "NotMet");
        }
        assert_eq!(other.expect("checked"), serde_json::Value::Null);
    }
    /// A `listing` type whose trees keep its count, each owner's count and each
    /// category's total price, with three rules reading those totals and one,
    /// `priceCap`, reading only the price.
    fn totalled_contract_bytes() -> Vec<u8> {
        let platform_version = PlatformVersion::latest();
        let documents = platform_value!({
            "listing": {
                "type": "object",
                "documentsCountable": true,
                "properties": {
                    "price": { "type": "integer", "minimum": 0, "maximum": 1000000000, "position": 0 },
                    "category": { "type": "integer", "minimum": 0, "maximum": 100, "position": 1 }
                },
                "required": ["price", "category"],
                "indices": [
                    {
                        "name": "byOwner",
                        "properties": [{ "$ownerId": "asc" }],
                        "countable": "countable"
                    },
                    {
                        "name": "byCategory",
                        "properties": [{ "category": "asc" }],
                        "summable": "price"
                    }
                ],
                "propertyConstraints": {
                    "allListings": { "lessThan": [{ "countOf": ["listing"] }, 1000] },
                    "atMostTwoPerOwner": {
                        "lessThanOrEqual": [
                            { "countOf": ["listing", { "$ownerId": "$ownerId" }] },
                            2
                        ]
                    },
                    "categoryBudget": {
                        "lessThanOrEqual": [
                            { "sumOf": ["listing", "price", { "category": "category" }] },
                            250
                        ]
                    },
                    "priceCap": { "lessThanOrEqual": ["price", 1000] }
                },
                "additionalProperties": false
            }
        });
        DataContractFactory::new(platform_version.protocol_version)
            .expect("factory for the protocol version")
            .create_with_value_config(Identifier::new(OWNER), 1, documents, None, None)
            .expect("listing contract")
            .data_contract()
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("serialized contract")
    }

    /// The descriptor lists the `countOf` and `sumOf` totals each rule reads,
    /// and the pre-check, which reads no state, leaves a rule reading one
    /// unjudged while it still judges the others.
    #[test]
    fn should_list_the_totals_a_rule_reads_and_leave_it_unjudged() {
        let sdk = sdk_handle(PlatformVersion::latest());
        let contract = totalled_contract_bytes();

        let rules = rules_of(sdk, &contract, "listing");
        // 500 is past the category budget of 250 even alone, but the budget's
        // total is read from state, which the pre-check does not do
        let over_budget = check(
            sdk,
            &contract,
            "listing",
            json!({ "price": 500, "category": 1 }),
            OWNER,
        );
        let over_cap = check(
            sdk,
            &contract,
            "listing",
            json!({ "price": 2000, "category": 1 }),
            OWNER,
        );
        destroy_mock_sdk_handle(sdk);

        let rules = rules.expect("rules of listing");
        let totals = |index: usize| rules[index]["readsTotals"].clone();
        assert_eq!(
            totals(0),
            json!([{ "kind": "countOf", "documentType": "listing", "filter": [] }])
        );
        assert_eq!(
            totals(1),
            json!([{ "kind": "countOf", "documentType": "listing", "filter": ["$ownerId"] }])
        );
        assert_eq!(rules[1]["readsOwner"], json!(true));
        assert_eq!(
            totals(2),
            json!([{
                "kind": "sumOf",
                "documentType": "listing",
                "property": "price",
                "filter": ["category"]
            }])
        );
        assert_eq!(
            rules[2]["reads"],
            json!([{ "path": "category", "kind": "value" }])
        );
        assert_eq!(totals(3), json!([]));

        assert_eq!(over_budget.expect("checked"), serde_json::Value::Null);
        let over_cap = over_cap.expect("checked");
        assert_eq!(over_cap["rule"], "priceCap");
        assert_eq!(over_cap["violation"], "NotMet");
    }
}
