use crate::identity::IdentityV0;
use crate::prelude::Identity;
use platform_value::Identifier;
use std::collections::BTreeMap;

impl Identity {
    #[inline(always)]
    pub(super) fn create_basic_identity_v0(id: [u8; 32]) -> Self {
        IdentityV0 {
            id: Identifier::new(id),
            revision: 0,
            balance: 0,
            public_keys: BTreeMap::new(),
        }
        .into()
    }
}
