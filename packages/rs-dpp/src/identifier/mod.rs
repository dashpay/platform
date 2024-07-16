pub use platform_value::Identifier;
pub use platform_value::IDENTIFIER_MEDIA_TYPE as MEDIA_TYPE;
use sha2::{Digest, Sha256};

pub trait MasternodeIdentifiers {
    fn create_voter_identifier(pro_tx_hash: &[u8; 32], voting_address: &[u8; 20]) -> Identifier;

    fn create_operator_identifier(pro_tx_hash: &[u8; 32], pub_key_operator: &[u8]) -> Identifier;
}

trait IdentifierConstructorPrivate {
    fn hash_protxhash_with_key_data(pro_tx_hash: &[u8; 32], key_data: &[u8]) -> Identifier;
}
impl MasternodeIdentifiers for Identifier {
    fn create_voter_identifier(pro_tx_hash: &[u8; 32], voting_address: &[u8; 20]) -> Identifier {
        Self::hash_protxhash_with_key_data(pro_tx_hash, voting_address)
    }

    fn create_operator_identifier(pro_tx_hash: &[u8; 32], pub_key_operator: &[u8]) -> Identifier {
        Self::hash_protxhash_with_key_data(pro_tx_hash, pub_key_operator)
    }
}

impl IdentifierConstructorPrivate for Identifier {
    fn hash_protxhash_with_key_data(pro_tx_hash: &[u8; 32], key_data: &[u8]) -> Identifier {
        let mut hasher = Sha256::new();
        hasher.update(pro_tx_hash);
        hasher.update(key_data);
        let bytes: [u8; 32] = hasher.finalize().into();
        bytes.into()
    }
}
