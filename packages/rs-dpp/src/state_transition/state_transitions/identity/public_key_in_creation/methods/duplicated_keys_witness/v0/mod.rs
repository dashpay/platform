use crate::identity::KeyID;
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use std::collections::HashMap;

impl IdentityPublicKeyInCreation {
    #[inline(always)]
    pub(super) fn duplicated_keys_witness_v0(
        public_keys: &[IdentityPublicKeyInCreation],
    ) -> Vec<KeyID> {
        let mut keys_count = HashMap::<Vec<u8>, usize>::new();
        let mut duplicated_key_ids = vec![];

        for public_key in public_keys.iter() {
            let data = public_key.data().as_slice();
            let count = *keys_count.get(data).unwrap_or(&0_usize);
            let count = count + 1;
            keys_count.insert(data.to_vec(), count);

            if count > 1 {
                duplicated_key_ids.push(public_key.id());
            }
        }

        duplicated_key_ids
    }
}
