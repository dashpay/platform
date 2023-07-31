use std::collections::BTreeMap;
use platform_value::Identifier;
use crate::identity::IdentityV0;
use crate::prelude::Identity;

impl Identity {
    pub(super) fn create_basic_identity_v0(id: [u8; 32]) -> Self {
        IdentityV0 {
            id: Identifier::new(id),
            revision: 1,
            balance: 0,
            public_keys: BTreeMap::new(),
        }.into()
    }
}