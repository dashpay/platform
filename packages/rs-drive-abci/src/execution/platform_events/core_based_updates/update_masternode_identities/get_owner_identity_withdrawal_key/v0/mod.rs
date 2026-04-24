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
    pub(super) fn get_owner_identity_withdrawal_key_v0(
        payout_address: [u8; 20],
        key_id: KeyID,
    ) -> Result<IdentityPublicKey, Error> {
        Ok(IdentityPublicKeyV0 {
            id: key_id,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(payout_address.to_vec()),
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

    /// Withdrawal key is a TRANSFER purpose, CRITICAL, read-only ECDSA_HASH160 key
    /// pinned to the payout address. Any drift in the key constants should fail this test.
    #[test]
    fn v0_builds_transfer_critical_ecdsa_hash160_key() {
        let payout_address = [3u8; 20];
        let key_id = 11u32;

        let key = Platform::<MockCoreRPCLike>::get_owner_identity_withdrawal_key_v0(
            payout_address,
            key_id,
        )
        .expect("v0 constructor must not fail");

        assert_eq!(key.id(), key_id);
        assert_eq!(key.key_type(), KeyType::ECDSA_HASH160);
        assert_eq!(key.purpose(), Purpose::TRANSFER);
        assert_eq!(key.security_level(), SecurityLevel::CRITICAL);
        assert!(key.read_only());
        assert!(key.disabled_at().is_none());
        assert!(key.contract_bounds().is_none());
        assert_eq!(key.data().as_slice(), &payout_address);
    }

    /// Zero-initialized payout address (valid 20-byte value) must be accepted without
    /// panic — the constructor is just a pure data shaping helper.
    #[test]
    fn v0_accepts_zero_payout_address() {
        let payout_address = [0u8; 20];
        let key =
            Platform::<MockCoreRPCLike>::get_owner_identity_withdrawal_key_v0(payout_address, 0)
                .expect("zero address must be accepted");
        assert_eq!(key.data().as_slice(), &payout_address);
    }
}
