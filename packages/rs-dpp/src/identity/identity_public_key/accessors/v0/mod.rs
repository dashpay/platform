use crate::identity::contract_bounds::ContractBounds;
use crate::identity::identity_public_key::KeyID;
use crate::identity::KeyType;
use crate::identity::Purpose;
use crate::identity::SecurityLevel;
use crate::identity::TimestampMillis;
use platform_value::BinaryData;

/// Trait for getters in IdentityPublicKeyV0
pub trait IdentityPublicKeyGettersV0 {
    /// Returns the KeyID
    fn id(&self) -> KeyID;

    /// Returns the Purpose
    fn purpose(&self) -> Purpose;

    /// Returns the SecurityLevel
    fn security_level(&self) -> SecurityLevel;

    /// Returns the KeyType
    fn key_type(&self) -> KeyType;

    /// Returns the read_only flag
    fn read_only(&self) -> bool;

    /// Returns the data as BinaryData
    fn data(&self) -> &BinaryData;

    /// Returns the data as BinaryData
    fn data_owned(self) -> BinaryData;

    /// Returns the disabled_at timestamp as Option
    fn disabled_at(&self) -> Option<TimestampMillis>;

    /// Is public key disabled
    fn is_disabled(&self) -> bool;

    /// Contract bounds
    fn contract_bounds(&self) -> Option<&ContractBounds>;
}

/// Trait for setters in IdentityPublicKeyV0
pub trait IdentityPublicKeySettersV0 {
    /// Sets the KeyID
    fn set_id(&mut self, id: KeyID);

    /// Sets the Purpose
    fn set_purpose(&mut self, purpose: Purpose);

    /// Sets the SecurityLevel
    fn set_security_level(&mut self, security_level: SecurityLevel);

    /// Sets the KeyType
    fn set_key_type(&mut self, key_type: KeyType);

    /// Sets the read_only flag
    fn set_read_only(&mut self, read_only: bool);

    /// Sets the data as BinaryData
    fn set_data(&mut self, data: BinaryData);

    /// Sets the disabled_at timestamp
    fn set_disabled_at(&mut self, timestamp_millis: u64);
    fn remove_disabled_at(&mut self);
}
