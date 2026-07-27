mod transformer;

use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

/// Shield from asset lock transition action v0
#[derive(Debug, Clone)]
pub struct ShieldFromAssetLockTransitionActionV0 {
    /// Asset lock outpoint bytes (txid + vout)
    pub asset_lock_outpoint: [u8; 36],
    /// Full asset-lock value consumed by this transition (added to system credits).
    /// It is distributed as: `shield_amount` -> shielded pool, `surplus_amount` ->
    /// `surplus_output` address (when set), and the remainder -> fee pools.
    pub asset_lock_value_to_be_consumed: Credits,
    /// SHA256(signable_bytes) for replay protection
    pub signable_bytes_hasher: [u8; 32],
    /// Amount going into shielded pool (|value_balance|)
    pub shield_amount: Credits,
    /// Notes from the orchard bundle actions
    pub notes: Vec<ShieldedActionNote>,
    /// Current total balance of the shielded pool
    pub current_total_balance: Credits,
    /// Optional platform address that receives the asset-lock surplus
    /// (`asset_lock_value_to_be_consumed - shield_amount - fee`). When `None`, the surplus is
    /// folded into the fee pools instead (capped at `shielded_implicit_fee_cap`).
    pub surplus_output: Option<PlatformAddress>,
    /// Credits routed to `surplus_output` (the surplus). `0` when `surplus_output` is `None`,
    /// in which case the surplus is added to the fee pools by the execution event.
    pub surplus_amount: Credits,
}
