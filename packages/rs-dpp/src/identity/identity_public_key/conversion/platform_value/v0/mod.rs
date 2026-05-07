use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;

pub trait IdentityPublicKeyPlatformValueConversionMethodsV0 {
    fn to_object(&self) -> Result<Value, ProtocolError>;
    fn into_object(self) -> Result<Value, ProtocolError>;
    /// Version-aware deserializer. Routes by platform_version's
    /// `identity_key_structure_version` into the correct V0 / V1 / ...
    /// inner type. Distinct from canonical `ValueConvertible::from_object`
    /// (which dispatches on the value's own `$formatVersion` tag).
    fn from_object(value: Value, platform_version: &PlatformVersion) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
