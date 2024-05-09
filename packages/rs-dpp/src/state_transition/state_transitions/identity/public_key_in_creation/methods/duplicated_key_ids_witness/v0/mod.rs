use crate::identity::KeyID;
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use std::collections::HashMap;

impl IdentityPublicKeyInCreation {
    /// Find duplicate key ids
    #[inline(always)]
    pub(super) fn duplicated_key_ids_witness_v0(
        public_keys: &[IdentityPublicKeyInCreation],
    ) -> Vec<KeyID> {
        let mut duplicated_ids = Vec::<KeyID>::new();
        let mut ids_count = HashMap::<KeyID, usize>::new();

        for public_key in public_keys.iter() {
            let id = public_key.id();
            let count = *ids_count.get(&id).unwrap_or(&0_usize);
            let count = count + 1;
            ids_count.insert(id, count);

            if count > 1 {
                duplicated_ids.push(id);
            }
        }

        duplicated_ids
    }
}
