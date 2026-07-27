use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(crate) fn get_operator_identity_keys_v0(
        pub_key_operator: Vec<u8>,
        operator_payout_address: Option<[u8; 20]>,
        platform_node_id: Option<[u8; 20]>,
    ) -> Result<Vec<IdentityPublicKey>, Error> {
        let mut identity_public_keys = vec![IdentityPublicKeyV0 {
            id: 0,
            key_type: KeyType::BLS12_381,
            purpose: Purpose::SYSTEM,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(pub_key_operator),
            disabled_at: None,
            contract_bounds: None,
        }
        .into()];
        if let Some(operator_payout_address) = operator_payout_address {
            identity_public_keys.push(
                IdentityPublicKeyV0 {
                    id: 1,
                    key_type: KeyType::ECDSA_HASH160,
                    purpose: Purpose::TRANSFER,
                    security_level: SecurityLevel::CRITICAL,
                    read_only: true,
                    data: BinaryData::new(operator_payout_address.to_vec()),
                    disabled_at: None,
                    contract_bounds: None,
                }
                .into(),
            );
        }
        if let Some(node_id) = platform_node_id {
            identity_public_keys.push(
                IdentityPublicKeyV0 {
                    id: 2,
                    key_type: KeyType::EDDSA_25519_HASH160,
                    purpose: Purpose::SYSTEM,
                    security_level: SecurityLevel::CRITICAL,
                    read_only: true,
                    data: BinaryData::new(node_id.to_vec()),
                    disabled_at: None,
                    contract_bounds: None,
                }
                .into(),
            );
        }

        Ok(identity_public_keys)
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::platform::Platform;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};

    /// When both `operator_payout_address` and `platform_node_id` are `None`, the
    /// function must return exactly one key: the BLS12_381 SYSTEM CRITICAL operator key.
    #[test]
    fn v0_only_operator_key_when_no_optionals() {
        let pub_key_operator = vec![1u8; 48];
        let keys = Platform::<MockCoreRPCLike>::get_operator_identity_keys_v0(
            pub_key_operator.clone(),
            None,
            None,
        )
        .expect("must succeed");

        assert_eq!(keys.len(), 1, "exactly 1 key when both optionals are None");
        assert_eq!(keys[0].id(), 0);
        assert_eq!(keys[0].key_type(), KeyType::BLS12_381);
        assert_eq!(keys[0].purpose(), Purpose::SYSTEM);
        assert_eq!(keys[0].security_level(), SecurityLevel::CRITICAL);
        assert!(keys[0].read_only());
        assert_eq!(keys[0].data().as_slice(), pub_key_operator.as_slice());
    }

    /// With only `operator_payout_address` set, return 2 keys: the operator key and
    /// a TRANSFER ECDSA_HASH160 key at id=1 carrying the payout address.
    #[test]
    fn v0_two_keys_with_only_payout_address() {
        let pub_key_operator = vec![2u8; 48];
        let payout = [7u8; 20];
        let keys = Platform::<MockCoreRPCLike>::get_operator_identity_keys_v0(
            pub_key_operator,
            Some(payout),
            None,
        )
        .expect("must succeed");

        assert_eq!(keys.len(), 2);
        assert_eq!(keys[1].id(), 1);
        assert_eq!(keys[1].key_type(), KeyType::ECDSA_HASH160);
        assert_eq!(keys[1].purpose(), Purpose::TRANSFER);
        assert_eq!(keys[1].security_level(), SecurityLevel::CRITICAL);
        assert!(keys[1].read_only());
        assert_eq!(keys[1].data().as_slice(), &payout);
    }

    /// With only `platform_node_id` set, return 2 keys: the operator key and
    /// an EDDSA_25519_HASH160 SYSTEM CRITICAL key at id=2 carrying the node_id.
    /// Note: key id 2 is used even though id 1 is empty — ids are position-based in the schema.
    #[test]
    fn v0_two_keys_with_only_node_id() {
        let pub_key_operator = vec![3u8; 48];
        let node_id = [4u8; 20];
        let keys = Platform::<MockCoreRPCLike>::get_operator_identity_keys_v0(
            pub_key_operator,
            None,
            Some(node_id),
        )
        .expect("must succeed");

        assert_eq!(keys.len(), 2);
        assert_eq!(
            keys[1].id(),
            2,
            "node_id key must be id=2 even when payout is absent"
        );
        assert_eq!(keys[1].key_type(), KeyType::EDDSA_25519_HASH160);
        assert_eq!(keys[1].purpose(), Purpose::SYSTEM);
        assert_eq!(keys[1].security_level(), SecurityLevel::CRITICAL);
        assert_eq!(keys[1].data().as_slice(), &node_id);
    }

    /// With both optionals set, return 3 keys: operator (id=0), payout (id=1), node_id (id=2).
    #[test]
    fn v0_three_keys_when_both_optionals_present() {
        let pub_key_operator = vec![4u8; 48];
        let payout = [5u8; 20];
        let node_id = [6u8; 20];
        let keys = Platform::<MockCoreRPCLike>::get_operator_identity_keys_v0(
            pub_key_operator.clone(),
            Some(payout),
            Some(node_id),
        )
        .expect("must succeed");

        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].id(), 0);
        assert_eq!(keys[1].id(), 1);
        assert_eq!(keys[2].id(), 2);
        assert_eq!(keys[0].data().as_slice(), pub_key_operator.as_slice());
        assert_eq!(keys[1].data().as_slice(), &payout);
        assert_eq!(keys[2].data().as_slice(), &node_id);
    }

    /// The function accepts `pub_key_operator` of any length and stores it verbatim
    /// (no length validation in this v0 helper). This guards the current contract.
    #[test]
    fn v0_operator_pub_key_stored_verbatim_including_unusual_length() {
        let pub_key_operator = vec![9u8; 10]; // not a real BLS key length
        let keys = Platform::<MockCoreRPCLike>::get_operator_identity_keys_v0(
            pub_key_operator.clone(),
            None,
            None,
        )
        .expect("helper does not validate key length");
        assert_eq!(keys[0].data().as_slice(), pub_key_operator.as_slice());
    }
}
