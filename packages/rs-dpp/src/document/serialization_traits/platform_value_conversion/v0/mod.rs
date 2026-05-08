use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Document-specific value-conversion helpers that don't have a canonical
/// `ValueConvertible` equivalent. After Phase D step 8 (slice A) trimmed
/// the trait, what stays is two distinct semantics:
///
/// - **Map-shape view**: `to_map_value` / `into_map_value` produce
///   `BTreeMap<String, Value>`. Used internally by `ExtendedDocument`
///   (which composes a Document plus metadata fields like
///   `$dataContractId`, `$type`, `$entropy`) and by `wasm-dpp2`'s
///   DocumentWasm wrapper for the same composition. Canonical
///   `ValueConvertible::to_object` returns `Value::Map(Vec<(Value, Value)>)`,
///   not `BTreeMap<String, Value>`; the conversion is one extra step but
///   the callers preferred the map directly.
///
/// - **Legacy-shape ingest**: `from_platform_value` accepts an un-tagged
///   Document value (no `$formatVersion`) and routes through the V0 inner
///   directly. Canonical `ValueConvertible::from_object` would error on
///   missing tag. The legacy DPNS / DashPay JSON fixtures and older
///   stored shapes lack the version tag — `from_platform_value` is how
///   the platform ingests them. Symmetric with `from_json_value` on the
///   JSON side.
///
/// The previously-defined `to_object` / `into_value` methods were deleted
/// — they were 1:1 equivalents of canonical
/// `ValueConvertible::to_object` / `into_object`. Use canonical for
/// produce-shape; use `from_platform_value` only for ingest of
/// un-tagged legacy shapes.
pub trait DocumentPlatformValueMethodsV0<'a>: Serialize + Deserialize<'a> {
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError>;
    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError>;
    fn from_platform_value(
        document_value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
