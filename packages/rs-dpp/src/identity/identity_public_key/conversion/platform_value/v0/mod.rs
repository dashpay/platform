use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;

pub trait IdentityPublicKeyPlatformValueConversionMethodsV0 {
    fn to_object(&self) -> Result<Value, ProtocolError>;
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError>;
    fn into_object(self) -> Result<Value, ProtocolError>;
    fn from_object(
        value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<IdentityPublicKeyV0, ProtocolError>;
}
