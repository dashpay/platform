use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
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

#[cfg(test)]
mod tests {
    use crate::platform_types::platform::Platform;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};

    /// The owner key is an ECDSA_HASH160 OWNER key with CRITICAL security, read-only,
    /// carrying the provided 20-byte address as its data. This helper is a pure constructor
    /// — any drift in the hard-coded key fields must be caught here.
    #[test]
    fn v0_builds_read_only_critical_owner_ecdsa_hash160_key() {
        let address = [7u8; 20];
        let key_id = 5u32;

        let key = Platform::<MockCoreRPCLike>::get_owner_identity_owner_key_v0(address, key_id)
            .expect("v0 constructor must not fail for a well-formed address");

        assert_eq!(key.id(), key_id);
        assert_eq!(key.key_type(), KeyType::ECDSA_HASH160);
        assert_eq!(key.purpose(), Purpose::OWNER);
        assert_eq!(key.security_level(), SecurityLevel::CRITICAL);
        assert!(key.read_only(), "owner key must be read-only");
        assert!(
            key.disabled_at().is_none(),
            "owner key must not be disabled at construction"
        );
        assert!(
            key.contract_bounds().is_none(),
            "owner key must have no contract bounds"
        );
        assert_eq!(key.data().as_slice(), &address);
    }

    /// The key's `id` field is threaded through unchanged; test with a couple of
    /// extreme/representative values to guard the assignment line.
    #[test]
    fn v0_threads_key_id_through_for_multiple_values() {
        let address = [0u8; 20];
        for key_id in [0u32, 1u32, u32::MAX] {
            let key = Platform::<MockCoreRPCLike>::get_owner_identity_owner_key_v0(address, key_id)
                .expect("v0 constructor must not fail");
            assert_eq!(key.id(), key_id);
        }
    }

    /// Distinct addresses must produce distinct key data. Guards against any
    /// accidental aliasing or forgotten copy.
    #[test]
    fn v0_distinct_addresses_produce_distinct_keys() {
        let addr_a = [0x11u8; 20];
        let addr_b = [0x22u8; 20];
        let a = Platform::<MockCoreRPCLike>::get_owner_identity_owner_key_v0(addr_a, 0).expect("a");
        let b = Platform::<MockCoreRPCLike>::get_owner_identity_owner_key_v0(addr_b, 0).expect("b");
        assert_ne!(a.data().as_slice(), b.data().as_slice());
    }
}
