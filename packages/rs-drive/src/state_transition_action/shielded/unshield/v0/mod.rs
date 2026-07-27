mod transformer;

use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

/// Unshield transition action v0
#[derive(Debug, Clone)]
pub struct UnshieldTransitionActionV0 {
    /// The address receiving unshielded funds
    pub output_address: PlatformAddress,
    /// Amount being unshielded
    pub amount: Credits,
    /// Notes from the orchard bundle actions
    pub notes: Vec<ShieldedActionNote>,
    /// The anchor used for verification
    pub anchor: [u8; 32],
    /// Shielded fee paid to proposers, carved out of `amount` (the recipient
    /// receives `amount - fee_amount`). For an ordinary `Unshield` this equals
    /// `compute_shielded_unshield_fee` (the base shielded minimum fee plus the
    /// flat `AddBalanceToAddress` output-write storage cost). When
    /// `chargeable_failure` is set (the `IdentityCreateFromShieldedPool`
    /// fallback) it is instead the failure penalty.
    pub fee_amount: Credits,
    /// Current total balance of the shielded pool
    pub current_total_balance: Credits,
    /// `false` for an ordinary `Unshield`. `true` ONLY when this action is the
    /// chargeable failure of an `IdentityCreateFromShieldedPool` (the spend is
    /// finalized to `output_address` minus the penalty even though identity
    /// creation failed). This flag is what authorizes the `PaidFromShieldedPool`
    /// execution event to apply its ops despite the attached consensus errors —
    /// so the apply-despite-errors invariant is type-enforced rather than only
    /// comment-enforced. An ordinary `Unshield` must NEVER set it.
    pub chargeable_failure: bool,
}
