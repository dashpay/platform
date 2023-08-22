use crate::identity::identity_public_key::conversion::cbor::IdentityPublicKeyCborConversionMethodsV0;
use crate::identity::identity_public_key::conversion::platform_value::IdentityPublicKeyPlatformValueConversionMethodsV0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::util::cbor_serializer;
use crate::util::cbor_value::{CborCanonicalMap, CborMapExtension};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use ciborium::Value as CborValue;
use platform_value::{BinaryData, ValueMapHelper};
use std::convert::TryInto;

impl IdentityPublicKeyCborConversionMethodsV0 for IdentityPublicKeyV0 {
    fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut object = self.to_cleaned_object()?;
        object
            .to_map_mut()
            .unwrap()
            .sort_by_lexicographical_byte_ordering_keys_and_inner_maps();

        cbor_serializer::serializable_value_to_cbor(&object, None)
    }

    fn from_cbor_value(
        cbor_value: &CborValue,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let key_value_map = cbor_value.as_map().ok_or_else(|| {
            ProtocolError::DecodingError(String::from(
                "Expected identity public key to be a key value map",
            ))
        })?;

        let id = key_value_map.as_u16("id", "A key must have an uint16 id")?;
        let key_type = key_value_map.as_u8("type", "Identity public key must have a type")?;
        let purpose = key_value_map.as_u8("purpose", "Identity public key must have a purpose")?;
        let security_level = key_value_map.as_u8(
            "securityLevel",
            "Identity public key must have a securityLevel",
        )?;
        let readonly =
            key_value_map.as_bool("readOnly", "Identity public key must have a readOnly")?;
        let public_key_bytes =
            key_value_map.as_bytes("data", "Identity public key must have a data")?;
        let disabled_at = key_value_map.as_u64("disabledAt", "").ok();

        Ok(IdentityPublicKeyV0 {
            id: id.into(),
            purpose: purpose.try_into()?,
            security_level: security_level.try_into()?,
            key_type: key_type.try_into()?,
            data: BinaryData::new(public_key_bytes),
            read_only: readonly,
            disabled_at,
        })
    }

    fn to_cbor_value(&self) -> CborValue {
        let mut pk_map = CborCanonicalMap::new();

        pk_map.insert("id", self.id);
        pk_map.insert("data", self.data.as_slice());
        pk_map.insert("type", self.key_type);
        pk_map.insert("purpose", self.purpose);
        pk_map.insert("readOnly", self.read_only);
        pk_map.insert("securityLevel", self.security_level);
        if let Some(ts) = self.disabled_at {
            pk_map.insert("disabledAt", ts)
        }

        pk_map.to_value_sorted()
    }
}

impl Into<CborValue> for &IdentityPublicKeyV0 {
    fn into(self) -> CborValue {
        self.to_cbor_value()
    }
}
