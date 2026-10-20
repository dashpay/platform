use super::{fold_added_balance_outputs, AddedBalanceOutputsOrigin};
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::fee::Credits;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// v1 (protocol v13+): records BOTH origins. In addition to the historical `Transparent`-origin
    /// credits, this folds `ShieldedSpend`-origin credits (the `Unshield` net output, the
    /// `ShieldFromAssetLock` surplus, and the `IdentityCreateFromShieldedPool` fallback) so
    /// incremental client sync observes shielded-spend payouts. The merge semantics are identical to
    /// v0; only the origin filter differs (there is none here), so `origin` is unused.
    pub(super) fn record_added_balance_outputs_v1(
        &self,
        address_balances_in_update: Option<&mut BTreeMap<PlatformAddress, CreditOperation>>,
        added_to_balance_outputs: Option<BTreeMap<PlatformAddress, Credits>>,
        _origin: AddedBalanceOutputsOrigin,
    ) -> Result<(), Error> {
        fold_added_balance_outputs(address_balances_in_update, added_to_balance_outputs);
        Ok(())
    }
}
