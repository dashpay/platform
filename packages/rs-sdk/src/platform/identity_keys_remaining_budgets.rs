//! What is left of the budgets of identity keys.
//!
//! An authentication key may carry a total budget (protocol version 14). The key itself never
//! changes, so what remains of that budget is tracked by Platform next to it, and read here:
//!
//! ```ignore
//! let budgets = IdentityKeysRemainingBudgets::fetch(
//!     &sdk,
//!     IdentityKeysRemainingBudgetsQuery { identity_id, key_ids: vec![5, 6] },
//! )
//! .await?
//! .unwrap_or_default();
//! match budgets.get(&5) {
//!     Some(Some(0)) => { /* spent: the key can no longer sign */ }
//!     Some(Some(remaining)) => { /* `remaining` credits left */ }
//!     _ => { /* the key has no budget, or does not exist */ }
//! }
//! ```
//!
//! [`IdentityKeysRemainingBudgets`] also implements [`FetchUnproved`] for the unverified fast
//! path.

use crate::platform::{Fetch, FetchUnproved, Identifier, Query, QuerySettings};
use crate::Error;
use dapi_grpc::platform::v0::get_identity_keys_remaining_budgets_request::{
    self, GetIdentityKeysRemainingBudgetsRequestV0,
};
use dapi_grpc::platform::v0::GetIdentityKeysRemainingBudgetsRequest;
use dpp::identity::KeyID;
pub use drive_proof_verifier::types::identity_keys_remaining_budgets::IdentityKeysRemainingBudgets;

/// Query for what is left of the budgets of several keys of one identity
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityKeysRemainingBudgetsQuery {
    /// The identity the keys belong to
    pub identity_id: Identifier,
    /// The ids of the keys to look up: at least one, none repeated
    pub key_ids: Vec<KeyID>,
}

impl Query<GetIdentityKeysRemainingBudgetsRequest> for IdentityKeysRemainingBudgetsQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetIdentityKeysRemainingBudgetsRequest, Error> {
        Ok(GetIdentityKeysRemainingBudgetsRequest {
            version: Some(get_identity_keys_remaining_budgets_request::Version::V0(
                GetIdentityKeysRemainingBudgetsRequestV0 {
                    identity_id: self.identity_id.to_vec(),
                    key_ids: self.key_ids.clone(),
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Fetch for IdentityKeysRemainingBudgets {
    type Query = GetIdentityKeysRemainingBudgetsRequest;
    type Request = GetIdentityKeysRemainingBudgetsRequest;
}

impl FetchUnproved for IdentityKeysRemainingBudgets {
    type Request = GetIdentityKeysRemainingBudgetsRequest;
}
