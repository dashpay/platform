use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(crate) fn get_voter_identity_key_v0(
        voting_address: [u8; 20],
        key_id: KeyID,
    ) -> Result<IdentityPublicKey, Error> {
        Ok(IdentityPublicKeyV0 {
            id: key_id,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::VOTING,
            security_level: SecurityLevel::HIGH,
            read_only: true,
            data: BinaryData::new(voting_address.to_vec()),
            disabled_at: None,
        }
        .into())
    }
}
