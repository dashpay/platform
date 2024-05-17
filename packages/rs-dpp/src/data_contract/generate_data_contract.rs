use crate::data_contract::DataContract;
use crate::prelude::IdentityNonce;
use platform_value::Identifier;
use std::io::Write;

use crate::util::hash::hash_double;

impl DataContract {
    /// Generate data contract id based on owner id and identity nonce
    pub fn generate_data_contract_id_v0(
        owner_id: impl AsRef<[u8]>,
        identity_nonce: IdentityNonce,
    ) -> Identifier {
        let mut b: Vec<u8> = vec![];
        let _ = b.write(owner_id.as_ref());
        let _ = b.write(identity_nonce.to_be_bytes().as_slice());
        Identifier::from(hash_double(b))
    }
}
