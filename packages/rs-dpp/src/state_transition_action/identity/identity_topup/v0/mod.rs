#[cfg(feature = "state-transition-transformers")]
mod transformer;

use crate::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::identifier::Identifier;

use platform_value::Bytes36;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityTopUpTransitionActionV0 {
    pub top_up_balance_amount: u64,
    pub identity_id: Identifier,
    pub asset_lock_outpoint: Bytes36,
}
