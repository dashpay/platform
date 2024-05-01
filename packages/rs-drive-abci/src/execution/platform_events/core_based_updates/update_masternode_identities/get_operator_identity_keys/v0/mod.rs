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
