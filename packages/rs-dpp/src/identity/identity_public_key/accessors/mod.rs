use crate::fee::Credits;
use crate::identity::contract_bounds::ContractBounds;
use crate::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use crate::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use crate::identity::KeyType;
use crate::identity::Purpose;
use crate::identity::SecurityLevel;
use crate::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use platform_value::BinaryData;

pub mod v0;
pub mod v1;

impl IdentityPublicKeyGettersV0 for IdentityPublicKey {
    fn id(&self) -> KeyID {
        match self {
            IdentityPublicKey::V0(v0) => v0.id(),
            IdentityPublicKey::V1(v1) => v1.id(),
        }
    }

    fn purpose(&self) -> Purpose {
        match self {
            IdentityPublicKey::V0(v0) => v0.purpose(),
            IdentityPublicKey::V1(v1) => v1.purpose(),
        }
    }

    fn security_level(&self) -> SecurityLevel {
        match self {
            IdentityPublicKey::V0(v0) => v0.security_level(),
            IdentityPublicKey::V1(v1) => v1.security_level(),
        }
    }

    fn key_type(&self) -> KeyType {
        match self {
            IdentityPublicKey::V0(v0) => v0.key_type(),
            IdentityPublicKey::V1(v1) => v1.key_type(),
        }
    }

    fn read_only(&self) -> bool {
        match self {
            IdentityPublicKey::V0(v0) => v0.read_only(),
            IdentityPublicKey::V1(v1) => v1.read_only(),
        }
    }

    fn data(&self) -> &BinaryData {
        match self {
            IdentityPublicKey::V0(v0) => v0.data(),
            IdentityPublicKey::V1(v1) => v1.data(),
        }
    }

    fn data_owned(self) -> BinaryData {
        match self {
            IdentityPublicKey::V0(v0) => v0.data_owned(),
            IdentityPublicKey::V1(v1) => v1.data_owned(),
        }
    }

    fn disabled_at(&self) -> Option<TimestampMillis> {
        match self {
            IdentityPublicKey::V0(v0) => v0.disabled_at(),
            IdentityPublicKey::V1(v1) => v1.disabled_at(),
        }
    }

    fn is_disabled(&self) -> bool {
        match self {
            IdentityPublicKey::V0(v0) => v0.is_disabled(),
            IdentityPublicKey::V1(v1) => v1.is_disabled(),
        }
    }

    fn contract_bounds(&self) -> Option<&ContractBounds> {
        match self {
            IdentityPublicKey::V0(v0) => v0.contract_bounds(),
            IdentityPublicKey::V1(v1) => v1.contract_bounds(),
        }
    }
}

impl IdentityPublicKeyGettersV1 for IdentityPublicKey {
    fn budget(&self) -> Option<Credits> {
        match self {
            IdentityPublicKey::V0(_) => None,
            IdentityPublicKey::V1(v1) => v1.budget(),
        }
    }

    fn expires_at(&self) -> Option<TimestampMillis> {
        match self {
            IdentityPublicKey::V0(_) => None,
            IdentityPublicKey::V1(v1) => v1.expires_at(),
        }
    }
}

impl IdentityPublicKeySettersV0 for IdentityPublicKey {
    fn set_id(&mut self, id: KeyID) {
        match self {
            IdentityPublicKey::V0(v0) => {
                v0.set_id(id);
            }
            IdentityPublicKey::V1(v1) => {
                v1.set_id(id);
            }
        }
    }

    fn set_purpose(&mut self, purpose: Purpose) {
        match self {
            IdentityPublicKey::V0(v0) => {
                v0.set_purpose(purpose);
            }
            IdentityPublicKey::V1(v1) => {
                v1.set_purpose(purpose);
            }
        }
    }

    fn set_security_level(&mut self, security_level: SecurityLevel) {
        match self {
            IdentityPublicKey::V0(v0) => {
                v0.set_security_level(security_level);
            }
            IdentityPublicKey::V1(v1) => {
                v1.set_security_level(security_level);
            }
        }
    }

    fn set_key_type(&mut self, key_type: KeyType) {
        match self {
            IdentityPublicKey::V0(v0) => {
                v0.set_key_type(key_type);
            }
            IdentityPublicKey::V1(v1) => {
                v1.set_key_type(key_type);
            }
        }
    }

    fn set_read_only(&mut self, read_only: bool) {
        match self {
            IdentityPublicKey::V0(v0) => {
                v0.set_read_only(read_only);
            }
            IdentityPublicKey::V1(v1) => {
                v1.set_read_only(read_only);
            }
        }
    }

    fn set_data(&mut self, data: BinaryData) {
        match self {
            IdentityPublicKey::V0(v0) => {
                v0.set_data(data);
            }
            IdentityPublicKey::V1(v1) => {
                v1.set_data(data);
            }
        }
    }

    fn set_disabled_at(&mut self, timestamp_millis: u64) {
        match self {
            IdentityPublicKey::V0(v0) => {
                v0.set_disabled_at(timestamp_millis);
            }
            IdentityPublicKey::V1(v1) => {
                v1.set_disabled_at(timestamp_millis);
            }
        }
    }

    fn remove_disabled_at(&mut self) {
        match self {
            IdentityPublicKey::V0(v0) => {
                v0.remove_disabled_at();
            }
            IdentityPublicKey::V1(v1) => {
                v1.remove_disabled_at();
            }
        }
    }
}
