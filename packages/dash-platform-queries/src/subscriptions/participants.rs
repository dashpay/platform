//! The platform addresses a state transition names, read from the transition alone.

use super::Role;
use dpp::address_funds::PlatformAddress;
use dpp::state_transition::address_credit_withdrawal_transition::accessors::AddressCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::address_funding_from_asset_lock_transition::accessors::AddressFundingFromAssetLockTransitionAccessorsV0;
use dpp::state_transition::address_funds_transfer_transition::accessors::AddressFundsTransferTransitionAccessorsV0;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::accessors::IdentityTopUpFromAddressesTransitionAccessorsV0;
use dpp::state_transition::shield_from_asset_lock_transition::accessors::ShieldFromAssetLockTransitionAccessorsV0;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::unshield_transition::accessors::UnshieldTransitionAccessorsV0;
use dpp::state_transition::StateTransition;

/// The platform addresses `state_transition` names, each with its side: the inputs it spends
/// from are senders, the outputs it credits recipients.
pub(super) fn addresses(state_transition: &StateTransition) -> Vec<(PlatformAddress, Role)> {
    // Every variant is listed, so a new one must be classified here.
    let recipients: Vec<&PlatformAddress> = match state_transition {
        StateTransition::IdentityCreditTransferToAddresses(transition) => {
            transition.recipient_addresses().keys().collect()
        }
        StateTransition::AddressFundsTransfer(transition) => transition.outputs().keys().collect(),
        StateTransition::AddressFundingFromAssetLock(transition) => {
            transition.outputs().keys().collect()
        }
        StateTransition::IdentityCreateFromAddresses(transition) => {
            transition.output().map(|(address, _)| address).into_iter().collect()
        }
        StateTransition::IdentityTopUpFromAddresses(transition) => {
            transition.output().map(|(address, _)| address).into_iter().collect()
        }
        StateTransition::AddressCreditWithdrawal(transition) => {
            transition.output().map(|(address, _)| address).into_iter().collect()
        }
        StateTransition::Unshield(transition) => vec![transition.output_address()],
        StateTransition::ShieldFromAssetLock(transition) => {
            transition.surplus_output().into_iter().collect()
        }
        // Credited instead when the identity cannot be created.
        StateTransition::IdentityCreateFromShieldedPool(transition) => {
            vec![transition.send_to_address_on_creation_failure()]
        }
        // Shield spends address inputs (listed below) but credits the shielded pool.
        StateTransition::Shield(_)
        | StateTransition::DataContractCreate(_)
        | StateTransition::DataContractUpdate(_)
        | StateTransition::ContractUserModeration(_)
        | StateTransition::ContractFeeClaim(_)
        | StateTransition::Batch(_)
        | StateTransition::IdentityCreate(_)
        | StateTransition::IdentityTopUp(_)
        | StateTransition::IdentityCreditWithdrawal(_)
        | StateTransition::IdentityUpdate(_)
        | StateTransition::IdentityKeyLimitsUpdate(_)
        | StateTransition::IdentityCreditTransfer(_)
        | StateTransition::MasternodeVote(_)
        | StateTransition::ShieldFromIdentity(_)
        | StateTransition::IdentityTopUpFromShieldedPool(_)
        // Shielded senders and recipients are not public.
        | StateTransition::ShieldedTransfer(_)
        | StateTransition::ShieldedWithdrawal(_)
        | StateTransition::TokenShieldedTransferWithShieldedFee(_)
        | StateTransition::TokenPurchaseFromShieldedPool(_)
        | StateTransition::TokenUnshieldWithShieldedFee(_) => vec![],
    };
    let senders = state_transition
        .inputs()
        .into_iter()
        .flat_map(|inputs| inputs.keys());
    senders
        .map(|address| (*address, Role::Sender))
        .chain(
            recipients
                .into_iter()
                .map(|address| (*address, Role::Recipient)),
        )
        .collect()
}
