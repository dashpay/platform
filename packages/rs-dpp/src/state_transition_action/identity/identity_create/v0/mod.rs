#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

use crate::identifier::Identifier;
use crate::identity::{IdentityPublicKey, PartialIdentity};

use platform_value::Bytes36;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreateTransitionActionV0 {
    pub public_keys: Vec<IdentityPublicKey>,
    pub initial_balance_amount: u64,
    pub identity_id: Identifier,
    pub asset_lock_outpoint: Bytes36,
}

impl From<IdentityCreateTransitionActionV0> for PartialIdentity {
    fn from(value: IdentityCreateTransitionActionV0) -> Self {
        let IdentityCreateTransitionActionV0 {
            initial_balance_amount,
            identity_id,
            ..
        } = value;
        PartialIdentity {
            id: identity_id,
            loaded_public_keys: Default::default(), //no need to load public keys
            balance: Some(initial_balance_amount),
            revision: None,
            not_found_public_keys: Default::default(),
        }
    }
}

impl From<&IdentityCreateTransitionActionV0> for PartialIdentity {
    fn from(value: &IdentityCreateTransitionActionV0) -> Self {
        let IdentityCreateTransitionActionV0 {
            initial_balance_amount,
            identity_id,
            ..
        } = value;
        PartialIdentity {
            id: *identity_id,
            loaded_public_keys: Default::default(), //no need to load public keys
            balance: Some(*initial_balance_amount),
            revision: None,
            not_found_public_keys: Default::default(),
        }
    }
}
