use crate::identity::contract_bounds::ContractBounds;
use crate::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::identity::KeyType;
use crate::identity::Purpose;
use crate::identity::SecurityLevel;
use crate::identity::{KeyID, TimestampMillis};
use platform_value::BinaryData;

impl IdentityPublicKeyGettersV0 for IdentityPublicKeyV0 {
    fn id(&self) -> KeyID {
        self.id
    }

    fn purpose(&self) -> Purpose {
        self.purpose
    }

    fn security_level(&self) -> SecurityLevel {
        self.security_level
    }

    fn key_type(&self) -> KeyType {
        self.key_type
    }

    fn read_only(&self) -> bool {
        self.read_only
    }

    fn data(&self) -> &BinaryData {
        &self.data
    }

    fn data_owned(self) -> BinaryData {
        self.data
    }

    fn disabled_at(&self) -> Option<TimestampMillis> {
        self.disabled_at
    }

    fn is_disabled(&self) -> bool {
        self.disabled_at.is_some()
    }

    fn contract_bounds(&self) -> Option<&ContractBounds> {
        self.contract_bounds.as_ref()
    }
}

impl IdentityPublicKeySettersV0 for IdentityPublicKeyV0 {
    fn set_id(&mut self, id: KeyID) {
        self.id = id;
    }

    fn set_purpose(&mut self, purpose: Purpose) {
        self.purpose = purpose;
    }

    fn set_security_level(&mut self, security_level: SecurityLevel) {
        self.security_level = security_level;
    }

    fn set_key_type(&mut self, key_type: KeyType) {
        self.key_type = key_type;
    }

    fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    fn set_data(&mut self, data: BinaryData) {
        self.data = data;
    }

    fn set_disabled_at(&mut self, timestamp_millis: u64) {
        self.disabled_at = Some(timestamp_millis);
    }

    fn remove_disabled_at(&mut self) {
        self.disabled_at = None;
    }
}
