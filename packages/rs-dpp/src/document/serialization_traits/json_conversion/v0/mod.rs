use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use serde_json::Value as JsonValue;

/// Document-specific JSON helper trait — after Phase D step 8 slice B,
/// holds only the **validating-JSON** wire shape that has no canonical
/// equivalent.
///
/// `to_json_with_identifiers_using_bytes` produces JSON with bs58
/// string identifiers + binary fields rendered as JSON arrays of u8.
/// Used by JSON Schema validators that don't accept base64 string
/// encodings of binary data.
///
/// History:
/// - Slice A deleted `to_json(&self, &PlatformVersion)` — 1:1 of
///   canonical `JsonConvertible::to_json`.
/// - Slice B deleted `from_json_value<S, E>` — accepted legacy
///   un-tagged JSON, but the only production caller (wasm-dpp2
///   DocumentWasm.fromJSON) was migrated to canonical
///   `JsonConvertible::from_json` after `toJSON` was made to emit
///   `$formatVersion`.
pub trait DocumentJsonMethodsV0<'a>: DocumentPlatformValueMethodsV0<'a> {
    fn to_json_with_identifiers_using_bytes(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError>;
}
