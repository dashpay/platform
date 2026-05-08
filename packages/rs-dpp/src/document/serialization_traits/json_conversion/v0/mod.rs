use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::convert::TryInto;

/// Document-specific JSON conversion helpers that don't have a canonical
/// `JsonConvertible` equivalent. After Phase D step 8 slice A trimmed
/// the trait, what stays is two methods with semantic content distinct
/// from canonical:
///
/// - `to_json_with_identifiers_using_bytes` — produces a
///   **validating-JSON** wire shape: bs58 string identifiers + binary
///   fields rendered as JSON arrays of u8. Used by JSON Schema validators
///   that don't accept base64 string encodings of binary data.
///
/// - `from_json_value<S, E>` — accepts a JSON shape distinct from
///   canonical: generic over the identifier deserialization type
///   (`String` for bs58, `Vec<u8>` for raw bytes), manually parses each
///   system field, and **doesn't require `$formatVersion`** — accepts
///   legacy un-tagged JSON. Canonical `JsonConvertible::from_json` would
///   error on missing `$formatVersion`.
///
/// The previously-defined `to_json` method was deleted — it was a 1:1
/// equivalent of canonical `JsonConvertible::to_json`.
pub trait DocumentJsonMethodsV0<'a>: DocumentPlatformValueMethodsV0<'a> {
    fn to_json_with_identifiers_using_bytes(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError>;
    fn from_json_value<S, E>(
        document_value: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = E>,
        E: Into<ProtocolError>,
        Self: Sized;
}
