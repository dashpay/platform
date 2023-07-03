use crate::data_contract::DataContract;
use platform_value::Identifier;
use std::io::Write;

use crate::util::hash::hash;

impl DataContract {
    /// Generate data contract id based on owner id and entropy
    pub fn generate_data_contract_id_v0(
        owner_id: impl AsRef<[u8]>,
        entropy: impl AsRef<[u8]>,
    ) -> Identifier {
        let mut b: Vec<u8> = vec![];
        let _ = b.write(owner_id.as_ref());
        let _ = b.write(entropy.as_ref());
        Identifier::from(hash(b))
    }
}
