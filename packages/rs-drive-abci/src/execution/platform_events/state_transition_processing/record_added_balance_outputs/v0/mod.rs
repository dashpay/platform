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
    /// v0: the HISTORICAL recorded set. Only `Transparent`-origin credits are folded (the `Paid`
    /// arm's long-standing `IdentityCreditTransferToAddresses` recording). `ShieldedSpend`-origin
    /// credits are DROPPED — the events carry them unconditionally now, but recording them would
    /// change the committed state root, so pre-v13 blocks byte-match by discarding them here.
    pub(super) fn record_added_balance_outputs_v0(
        &self,
        address_balances_in_update: Option<&mut BTreeMap<PlatformAddress, CreditOperation>>,
        added_to_balance_outputs: Option<BTreeMap<PlatformAddress, Credits>>,
        origin: AddedBalanceOutputsOrigin,
    ) -> Result<(), Error> {
        match origin {
            AddedBalanceOutputsOrigin::Transparent => {
                fold_added_balance_outputs(address_balances_in_update, added_to_balance_outputs);
            }
            // Dropped in v0: shielded-spend credits are not recorded before protocol v13.
            AddedBalanceOutputsOrigin::ShieldedSpend => {}
        }
        Ok(())
    }
}
