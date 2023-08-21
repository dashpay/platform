mod accessors;
mod conversion;
mod methods;
#[cfg(feature = "random-public-keys")]
mod random;

pub use crate::identity::KeyType;
pub use crate::identity::Purpose;
pub use crate::identity::SecurityLevel;

use bincode::{Decode, Encode};

use platform_value::BinaryData;
use serde::{Deserialize, Serialize};

use crate::identity::identity_public_key::contract_bounds::ContractBounds;
use crate::identity::identity_public_key::key_type::KEY_TYPE_MAX_SIZE_TYPE;
use crate::identity::Purpose::AUTHENTICATION;
use crate::identity::SecurityLevel::MASTER;
use crate::identity::{KeyID, TimestampMillis};
#[cfg(feature = "state-transitions")]
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;

#[derive(
    Debug, Serialize, Deserialize, Encode, Decode, Clone, PartialEq, Eq, Ord, PartialOrd, Hash,
)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPublicKeyV0 {
    pub id: KeyID,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    pub contract_bounds: Option<ContractBounds>,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    pub read_only: bool,
    pub data: BinaryData,
    #[serde(default)]
    pub disabled_at: Option<TimestampMillis>,
}

impl IdentityPublicKeyV0 {
    pub fn max_possible_size_key(id: KeyID) -> Self {
        let key_type = *KEY_TYPE_MAX_SIZE_TYPE;
        let purpose = AUTHENTICATION;
        let security_level = MASTER;
        let read_only = false;
        let data = BinaryData::new(vec![255; key_type.default_size()]);

        IdentityPublicKeyV0 {
            id,
            key_type,
            purpose,
            security_level,
            read_only,
            disabled_at: None,
            data,
            contract_bounds: None,
        }
    }
}

#[cfg(feature = "state-transitions")]
impl Into<IdentityPublicKeyInCreationV0> for &IdentityPublicKeyV0 {
    fn into(self) -> IdentityPublicKeyInCreationV0 {
        IdentityPublicKeyInCreationV0 {
            id: self.id,
            purpose: self.purpose,
            security_level: self.security_level,
            key_type: self.key_type,
            read_only: self.read_only,
            data: self.data.clone(),
            signature: BinaryData::default(),
            contract_bounds: self.contract_bounds.clone(),
        }
    }
}
