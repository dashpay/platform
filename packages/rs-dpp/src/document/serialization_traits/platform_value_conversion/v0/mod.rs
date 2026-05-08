use crate::ProtocolError;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Document-specific value-conversion helpers — after Phase D step 8
/// slice B, holds only the **map-shape** view that has no canonical
/// equivalent.
///
/// `to_map_value` / `into_map_value` produce `BTreeMap<String, Value>`.
/// Used internally by `ExtendedDocument` (which composes a Document
/// plus metadata fields like `$dataContractId`, `$type`, `$entropy`)
/// and by `wasm-dpp2`'s DocumentWasm wrapper for the same composition.
/// Canonical `ValueConvertible::to_object` returns `Value::Map(...)`,
/// not the BTreeMap directly; the conversion is one extra step but
/// callers prefer the map.
///
/// History:
/// - Slice A deleted `to_object` / `into_value` — 1:1 of canonical
///   `ValueConvertible::to_object` / `into_object`.
/// - Slice B deleted `from_platform_value` — accepted legacy
///   un-tagged Document values, but the only production caller path
///   (wasm-dpp2 DocumentWasm.fromObject + ExtendedDocument's
///   `from_trusted_platform_value` / `from_untrusted_platform_value`)
///   was migrated to canonical `Document::from_object` after
///   `DocumentWasm.toObject` was made to emit `$formatVersion`.
///   ExtendedDocument's untrusted ingest now inserts the tag explicitly
///   via the `ensure_document_format_version` helper before calling
///   canonical.
pub trait DocumentPlatformValueMethodsV0<'a>: Serialize + Deserialize<'a> {
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError>;
    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError>;
}
