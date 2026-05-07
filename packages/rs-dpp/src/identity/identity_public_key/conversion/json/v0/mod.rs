use crate::ProtocolError;
use serde_json::Value as JsonValue;

/// Legacy JSON conversion surface for `IdentityPublicKey`, providing two
/// shapes that canonical `JsonConvertible` doesn't:
///
/// - `to_json_object` produces a **validating-JSON** shape (binary
///   fields rendered as JSON arrays of u8 values, identifiers similarly
///   as arrays). Used by JSON-Schema validators that don't accept
///   base64-string encodings of binary data.
///
/// - `from_json_object` accepts the validating-JSON shape on the way
///   back, performing a `replace_at_paths(BINARY_DATA_FIELDS,
///   BinaryBytes)` rewrite before deserialization. Canonical
///   `JsonConvertible::from_json` expects base64 strings and would
///   reject byte-array forms.
///
/// `to_json` produces the canonical-shape JSON (base64 strings for
/// binary fields). It exists here primarily because the inner V0
/// struct doesn't directly derive `JsonConvertible` — the outer
/// `IdentityPublicKey` enum can also be reached through canonical
/// `JsonConvertible::to_json`.
pub trait IdentityPublicKeyJsonConversionMethodsV0 {
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    fn to_json_object(&self) -> Result<JsonValue, ProtocolError>;
    fn from_json_object(raw_object: JsonValue) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
