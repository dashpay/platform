use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyID, KeyType, identity_public_key::{Purpose, SecurityLevel}};
use dpp::platform_value::BinaryData;

impl<C> Platform<C> {
    pub(super) fn get_owner_identity_owner_key_v0(
        owner_public_key_address: [u8; 20],
        key_id: KeyID,
    ) -> Result<IdentityPublicKey, Error> {
        Ok(IdentityPublicKeyV0 {
            id: key_id,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::OWNER,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(owner_public_key_address.to_vec()),
            disabled_at: None,
            contract_bounds: None,
        }
        .into())
    }
}
