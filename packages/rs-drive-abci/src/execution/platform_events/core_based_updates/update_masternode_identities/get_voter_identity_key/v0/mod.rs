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
            contract_bounds: None,
        }
        .into())
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::platform::Platform;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};

    /// Voter key is VOTING purpose, HIGH security (NOT CRITICAL, distinguishing
    /// it from owner/withdrawal), read-only, ECDSA_HASH160. Any of those constants
    /// changing silently would break the identity-key schema.
    #[test]
    fn v0_builds_voting_high_ecdsa_hash160_key() {
        let voting_address = [9u8; 20];
        let key_id = 0u32;

        let key = Platform::<MockCoreRPCLike>::get_voter_identity_key_v0(voting_address, key_id)
            .expect("v0 constructor must not fail");

        assert_eq!(key.id(), key_id);
        assert_eq!(key.key_type(), KeyType::ECDSA_HASH160);
        assert_eq!(key.purpose(), Purpose::VOTING);
        assert_eq!(
            key.security_level(),
            SecurityLevel::HIGH,
            "voter key is HIGH, not CRITICAL"
        );
        assert!(key.read_only());
        assert!(key.disabled_at().is_none());
        assert!(key.contract_bounds().is_none());
        assert_eq!(key.data().as_slice(), &voting_address);
    }

    /// Voting address bytes must be preserved verbatim in the key's data.
    #[test]
    fn v0_preserves_voting_address_bytes() {
        let voting_address: [u8; 20] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let key = Platform::<MockCoreRPCLike>::get_voter_identity_key_v0(voting_address, 42)
            .expect("must succeed");
        assert_eq!(key.data().as_slice(), &voting_address);
    }
}
