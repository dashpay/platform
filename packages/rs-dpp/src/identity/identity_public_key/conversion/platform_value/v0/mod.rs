use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;

/// Version-aware ingest for `IdentityPublicKey` from a `platform_value::Value`.
///
/// `from_object(value, platform_version)` routes by the platform's
/// `identity_key_structure_version` into the correct V0 / V1 / ... inner
/// type. Distinct from canonical `ValueConvertible::from_object`, which
/// dispatches on the value's own `$formatVersion` tag.
///
/// The trait was previously also exposing `to_object` / `into_object` /
/// `to_cleaned_object`, but all three are byte-identical to canonical
/// `ValueConvertible` after Phase D step 4 (which added
/// `skip_serializing_if` to `disabled_at`). They've been deleted; this
/// trait now exists solely as the version-dispatch wrapper around
/// `from_object`.
pub trait IdentityPublicKeyPlatformValueConversionMethodsV0 {
    fn from_object(value: Value, platform_version: &PlatformVersion) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
