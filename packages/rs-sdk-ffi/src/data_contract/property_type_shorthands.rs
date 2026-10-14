//! Property type shorthands (protocol version 14): a document property schema
//! may write `"type": "identifier"`, or `"type": "bytes"` with a `size`, for
//! the byte array it stands for. The contract is stored, proved and returned
//! exactly as sent, so the contract JSON a client fetches holds the shorthand.
//!
//! [`dash_sdk_data_contract_json_expand_property_type_shorthands`] hands a
//! client that JSON with every shorthand written in full, rewritten by DPP's
//! own `DocumentType::expand_property_type_shorthands`, the rewrite consensus
//! reads every schema through, so a client reading property types never
//! learns what a shorthand means.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::dpp::data_contract::document_type::DocumentType;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::dpp::ProtocolError;

use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// The keys a contract JSON holds its document type schemas under: the
/// serialization format's and the contract's own.
const DOCUMENT_SCHEMAS_KEYS: [&str; 2] = ["documentSchemas", "documents"];

/// The keys a contract JSON holds its definitions under: the serialization
/// format's and the contract's own.
const SCHEMA_DEFS_KEYS: [&str; 2] = ["schemaDefs", "$defs"];

/// Get a data contract's JSON with its property type shorthands written in full
///
/// From protocol version 14 a document property schema may write
/// `"type": "identifier"` for `"type": "array", "byteArray": true,
/// "minItems": 32, "maxItems": 32, "contentMediaType":
/// "application/x.dash.dpp.identifier"`, and `"type": "bytes", "size": n` for
/// `"type": "array", "byteArray": true, "minItems": n, "maxItems": n`. The
/// contract is stored and returned as sent, so the JSON the fetch functions
/// return may hold either form.
///
/// `contract_json` is a contract as that JSON. The result is the same contract
/// with each document type schema (every object under `documentSchemas` or
/// `documents`) rewritten by DPP's `DocumentType::expand_property_type_shorthands`
/// and the definitions (the object under `schemaDefs` or `$defs`) by
/// `DocumentType::expand_schema_defs_property_type_shorthands`: a shorthand is
/// read on every property schema, the members of an object at any depth and
/// the `items` of a typed array included. Everything else is passed through
/// untouched, and a contract without a shorthand comes back as given,
/// re-serialized. It is a view for reading property types: what a client
/// stores and sends stays the contract as sent.
///
/// No SDK handle: the rewrite runs at `PlatformVersion::latest()` without full
/// validation. The contract is one the network already accepted, and a
/// shorthand can only be in a contract accepted at protocol version 14 or
/// later (every earlier meta-schema refuses both), so the latest version
/// writes it in the long form the network parses it as. Nothing is refused: a
/// shorthand that cannot be rewritten is left as sent. Taking no handle lets a
/// client holding only the JSON (the Swift contract parser, Kotlin's contract
/// view) call it.
///
/// Errors: `InvalidParameter` for a null pointer, or text that is not UTF-8,
/// not JSON or not a JSON object; `InternalError` when the rewrite fails, which
/// only a protocol version naming a rewrite this build does not know can cause.
///
/// # Safety
/// - `contract_json` must point to a NUL-terminated C string valid for the duration of the call.
/// - On success the result's `data` is a heap-allocated C string the caller frees with
///   `dash_sdk_string_free`; on error the caller frees `error` with `dash_sdk_error_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_json_expand_property_type_shorthands(
    contract_json: *const c_char,
) -> DashSDKResult {
    if contract_json.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Contract JSON is null".to_string(),
        ));
    }

    // SAFETY: non-null, and the caller guarantees NUL termination
    let text = match CStr::from_ptr(contract_json).to_str() {
        Ok(text) => text,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let expanded = contract_object(text).and_then(|mut contract| {
        expand_contract(&mut contract, PlatformVersion::latest())?;
        Ok(serde_json::Value::Object(contract))
    });
    let expanded = match expanded {
        Ok(expanded) => expanded,
        Err(error) => return DashSDKResult::error(error),
    };
    // JSON escapes every control character, so the text holds no NUL byte
    match CString::new(expanded.to_string()) {
        Ok(text) => DashSDKResult::success_string(text.into_raw()),
        Err(e) => DashSDKResult::error(FFIError::from(e).into()),
    }
}

/// The JSON object `text` holds.
fn contract_object(text: &str) -> Result<serde_json::Map<String, serde_json::Value>, DashSDKError> {
    match serde_json::from_str(text) {
        Ok(serde_json::Value::Object(contract)) => Ok(contract),
        Ok(_) => Err(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Contract JSON is not an object".to_string(),
        )),
        Err(e) => Err(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!("Invalid contract JSON: {}", e),
        )),
    }
}

/// Rewrites, in place, the document type schemas and the definitions of
/// `contract` that hold a shorthand; leaves every other value as it is.
fn expand_contract(
    contract: &mut serde_json::Map<String, serde_json::Value>,
    platform_version: &PlatformVersion,
) -> Result<(), DashSDKError> {
    for key in DOCUMENT_SCHEMAS_KEYS {
        let Some(serde_json::Value::Object(schemas)) = contract.get_mut(key) else {
            continue;
        };
        for schema in schemas.values_mut() {
            if !schema.is_object() {
                continue;
            }
            let expanded = DocumentType::expand_property_type_shorthands(
                &Value::from(&*schema),
                false,
                platform_version,
            )
            .map_err(expansion_error)?;
            if let Some(expanded) = expanded {
                *schema = json_of(expanded)?;
            }
        }
    }

    for key in SCHEMA_DEFS_KEYS {
        let Some(serde_json::Value::Object(definitions)) = contract.get_mut(key) else {
            continue;
        };
        let schema_defs: BTreeMap<String, Value> = definitions
            .iter()
            .map(|(name, definition)| (name.clone(), Value::from(definition)))
            .collect();
        let expanded = DocumentType::expand_schema_defs_property_type_shorthands(
            &schema_defs,
            false,
            platform_version,
        )
        .map_err(expansion_error)?;
        if let Some(expanded) = expanded {
            for (name, definition) in expanded {
                definitions.insert(name, json_of(definition)?);
            }
        }
    }
    Ok(())
}

/// A rewritten schema back as JSON. Every value in it came from JSON or is a
/// text, a boolean or an integer the rewrite wrote, so this cannot fail in
/// practice.
fn json_of(value: Value) -> Result<serde_json::Value, DashSDKError> {
    value.try_into().map_err(|e| {
        DashSDKError::new(
            DashSDKErrorCode::SerializationError,
            format!("Failed to write the expanded schema as JSON: {}", e),
        )
    })
}

/// Without full validation the rewrite refuses nothing; it fails only on a
/// version of it this build does not know, which is the build's fault.
fn expansion_error(error: ProtocolError) -> DashSDKError {
    DashSDKError::new(
        DashSDKErrorCode::InternalError,
        format!("Failed to expand the property type shorthands: {}", error),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dash_sdk_error_free;
    use crate::types::dash_sdk_string_free;
    use serde_json::json;

    /// The long form of `"type": "identifier"` beside `position`.
    fn identifier_long_form(position: u32) -> serde_json::Value {
        json!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": position
        })
    }

    /// The long form of `"type": "bytes", "size": size` beside `position`.
    fn bytes_long_form(size: u16, position: u32) -> serde_json::Value {
        json!({
            "type": "array",
            "byteArray": true,
            "minItems": size,
            "maxItems": size,
            "position": position
        })
    }

    /// A contract JSON shaped as the fetch functions return it, with one
    /// document type `note` declaring `properties`.
    fn contract_with(properties: serde_json::Value) -> serde_json::Value {
        json!({
            "$formatVersion": "1",
            "id": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            "ownerId": "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF",
            "version": 1,
            "config": { "$formatVersion": "1", "canBeDeleted": false, "readonly": false },
            "schemaDefs": null,
            "documentSchemas": {
                "note": {
                    "type": "object",
                    "properties": properties,
                    "additionalProperties": false
                }
            }
        })
    }

    /// The result's JSON, or its error code and message; frees both.
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

    fn expand_text(text: &str) -> Result<serde_json::Value, (DashSDKErrorCode, String)> {
        let text = CString::new(text).expect("no NUL in the JSON");
        take(unsafe { dash_sdk_data_contract_json_expand_property_type_shorthands(text.as_ptr()) })
    }

    fn expand(contract: &serde_json::Value) -> serde_json::Value {
        expand_text(&contract.to_string()).expect("the expanded contract")
    }

    #[test]
    fn should_write_identifier_and_bytes_properties_in_full() {
        let expanded = expand(&contract_with(json!({
            "recipientId": { "type": "identifier", "position": 0 },
            "txHash": { "type": "bytes", "size": 20, "position": 1 },
            "message": { "type": "string", "maxLength": 64, "position": 2 }
        })));

        let properties = &expanded["documentSchemas"]["note"]["properties"];
        assert_eq!(properties["recipientId"], identifier_long_form(0));
        assert_eq!(properties["txHash"], bytes_long_form(20, 1));
        assert_eq!(
            properties["message"],
            json!({ "type": "string", "maxLength": 64, "position": 2 })
        );
    }

    #[test]
    fn should_write_an_object_member_in_full() {
        let expanded = expand(&contract_with(json!({
            "payment": {
                "type": "object",
                "properties": {
                    "to": { "type": "identifier", "position": 0 },
                    "memo": {
                        "type": "object",
                        "properties": {
                            "hash": { "type": "bytes", "size": 32, "position": 0 }
                        },
                        "additionalProperties": false,
                        "position": 1
                    }
                },
                "additionalProperties": false,
                "position": 0
            }
        })));

        let payment = &expanded["documentSchemas"]["note"]["properties"]["payment"];
        assert_eq!(payment["type"], json!("object"));
        assert_eq!(payment["properties"]["to"], identifier_long_form(0));
        assert_eq!(
            payment["properties"]["memo"]["properties"]["hash"],
            bytes_long_form(32, 0)
        );
    }

    #[test]
    fn should_write_typed_array_items_in_full() {
        let expanded = expand(&contract_with(json!({
            "reasons": {
                "type": "array",
                "items": { "type": "identifier" },
                "maxItems": 4,
                "uniqueItems": true,
                "position": 0
            },
            "hashes": {
                "type": "array",
                "items": { "type": "bytes", "size": 20 },
                "maxItems": 8,
                "position": 1
            }
        })));

        let properties = &expanded["documentSchemas"]["note"]["properties"];
        assert_eq!(
            properties["reasons"],
            json!({
                "type": "array",
                "items": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier"
                },
                "maxItems": 4,
                "uniqueItems": true,
                "position": 0
            })
        );
        assert_eq!(
            properties["hashes"],
            json!({
                "type": "array",
                "items": { "type": "array", "byteArray": true, "minItems": 20, "maxItems": 20 },
                "maxItems": 8,
                "position": 1
            })
        );
    }

    #[test]
    fn should_write_definitions_in_full() {
        let mut contract = contract_with(json!({
            "owner": { "$ref": "#/$defs/owner", "position": 0 }
        }));
        contract["schemaDefs"] = json!({
            "owner": { "type": "identifier" },
            "hash": { "type": "bytes", "size": 20, "description": "a hash" },
            "note": { "type": "string", "maxLength": 10 }
        });

        let expanded = expand(&contract);

        assert_eq!(
            expanded["schemaDefs"],
            json!({
                "owner": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier"
                },
                "hash": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 20,
                    "maxItems": 20,
                    "description": "a hash"
                },
                "note": { "type": "string", "maxLength": 10 }
            })
        );
        // The reference itself is left as written
        assert_eq!(
            expanded["documentSchemas"]["note"]["properties"]["owner"],
            json!({ "$ref": "#/$defs/owner", "position": 0 })
        );
    }

    #[test]
    fn should_write_the_documents_and_defs_keys_in_full() {
        let contract = json!({
            "id": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            "documents": {
                "note": {
                    "type": "object",
                    "properties": {
                        "recipientId": { "type": "identifier", "position": 0 }
                    },
                    "additionalProperties": false
                }
            },
            "$defs": {
                "hash": { "type": "bytes", "size": 32 }
            }
        });

        let expanded = expand(&contract);

        assert_eq!(
            expanded["documents"]["note"]["properties"]["recipientId"],
            identifier_long_form(0)
        );
        assert_eq!(
            expanded["$defs"]["hash"],
            json!({ "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32 })
        );
    }

    #[test]
    fn should_return_a_contract_without_shorthands_as_given() {
        let mut contract = contract_with(json!({
            "recipientId": identifier_long_form(0),
            "message": { "type": "string", "maxLength": 64, "position": 1 },
            "size": { "type": "integer", "minimum": 0, "position": 2 }
        }));
        contract["schemaDefs"] = json!({ "note": { "type": "string", "maxLength": 10 } });

        assert_eq!(expand(&contract), contract);
    }

    /// Only the document type schemas and the definitions are read: a value
    /// shaped like a schema anywhere else is no property schema.
    #[test]
    fn should_pass_every_other_value_through_untouched() {
        let mut contract = contract_with(json!({
            "recipientId": { "type": "identifier", "position": 0 }
        }));
        contract["tokens"] = json!({
            "0": {
                "$formatVersion": "0",
                "baseSupply": 1000,
                "properties": { "x": { "type": "identifier" } },
                "type": "bytes",
                "size": 4
            }
        });
        contract["groups"] = json!({ "0": { "members": {}, "requiredPower": 1 } });

        let expanded = expand(&contract);

        for key in [
            "$formatVersion",
            "id",
            "ownerId",
            "version",
            "config",
            "schemaDefs",
            "tokens",
            "groups",
        ] {
            assert_eq!(expanded[key], contract[key], "{key}");
        }
        assert_eq!(
            expanded["documentSchemas"]["note"]["properties"]["recipientId"],
            identifier_long_form(0)
        );
    }

    /// Without full validation nothing is refused: a shorthand that cannot be
    /// rewritten is left as sent, and the others beside it are rewritten.
    #[test]
    fn should_leave_a_shorthand_it_cannot_rewrite_as_sent() {
        let expanded = expand(&contract_with(json!({
            "recipientId": { "type": "identifier", "position": 0 },
            "hash": { "type": "bytes", "position": 1 }
        })));

        let properties = &expanded["documentSchemas"]["note"]["properties"];
        assert_eq!(properties["recipientId"], identifier_long_form(0));
        assert_eq!(
            properties["hash"],
            json!({ "type": "bytes", "position": 1 })
        );
    }

    #[test]
    fn should_refuse_a_null_pointer_or_text_that_is_not_a_json_object() {
        let not_utf8 = take(unsafe {
            dash_sdk_data_contract_json_expand_property_type_shorthands(c"\xff\xfe".as_ptr())
        });
        let outcomes = [
            take(unsafe {
                dash_sdk_data_contract_json_expand_property_type_shorthands(std::ptr::null())
            }),
            not_utf8,
            expand_text("{\"documentSchemas\":"),
            expand_text("[1, 2]"),
            expand_text("null"),
        ];

        for outcome in outcomes {
            assert_eq!(
                outcome.expect_err("refused").0,
                DashSDKErrorCode::InvalidParameter
            );
        }
    }
}
