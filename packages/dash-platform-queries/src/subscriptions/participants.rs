//! Who and what a state transition names, read from the transition alone.

use super::Role;
use dpp::address_funds::PlatformAddress;
use dpp::prelude::Identifier;
use dpp::state_transition::address_credit_withdrawal_transition::accessors::AddressCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::address_funding_from_asset_lock_transition::accessors::AddressFundingFromAssetLockTransitionAccessorsV0;
use dpp::state_transition::address_funds_transfer_transition::accessors::AddressFundsTransferTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use dpp::state_transition::batch_transition::token_freeze_transition::v0::v0_methods::TokenFreezeTransitionV0Methods;
use dpp::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::token_unshield_transition::v0::v0_methods::TokenUnshieldTransitionV0Methods;
use dpp::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use dpp::state_transition::contract_fee_claim_transition::accessors::ContractFeeClaimTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::accessors::IdentityTopUpFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::accessors::IdentityTopUpFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::shield_from_asset_lock_transition::accessors::ShieldFromAssetLockTransitionAccessorsV0;
use dpp::state_transition::shield_from_identity_transition::accessors::ShieldFromIdentityTransitionAccessorsV0;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::accessors::TokenPurchaseFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::accessors::TokenShieldedTransferWithShieldedFeeTransitionAccessorsV0;
use dpp::state_transition::token_unshield_with_shielded_fee_transition::accessors::TokenUnshieldWithShieldedFeeTransitionAccessorsV0;
use dpp::state_transition::unshield_transition::accessors::UnshieldTransitionAccessorsV0;
use dpp::state_transition::{
    StateTransition, StateTransitionIdentityIdFromInputs, StateTransitionWitnessSigned,
};

/// The identities, addresses and data contract a non-batch transition names, each with the
/// side it is on. Batch transitions name their parties per inner transition; see
/// [`document_recipient`] and [`token_recipient`].
#[derive(Debug, Default)]
pub(super) struct Participants {
    pub identities: Vec<(Identifier, Role)>,
    pub addresses: Vec<(PlatformAddress, Role)>,
    /// The data contract a contract-level transition creates, updates, moderates or claims
    /// fees on.
    pub data_contract_id: Option<Identifier>,
    /// The token a transition outside a batch acts on (the shielded token transitions).
    pub token_id: Option<Identifier>,
}

impl Participants {
    fn identity(&mut self, identity_id: Identifier, side: Role) {
        self.identities.push((identity_id, side));
    }

    fn address(&mut self, address: PlatformAddress, side: Role) {
        self.addresses.push((address, side));
    }

    fn inputs<'a, V: 'a>(&mut self, inputs: impl IntoIterator<Item = (&'a PlatformAddress, V)>) {
        for (address, _) in inputs {
            self.address(*address, Role::Sender);
        }
    }

    fn outputs<'a, V: 'a>(&mut self, outputs: impl IntoIterator<Item = (&'a PlatformAddress, V)>) {
        for (address, _) in outputs {
            self.address(*address, Role::Recipient);
        }
    }
}

/// The parties `state_transition` names. For a batch, only its owner (the sender); the
/// recipients of its inner transitions come from [`document_recipient`] and [`token_recipient`].
pub(super) fn participants(state_transition: &StateTransition) -> Participants {
    let mut participants = Participants::default();
    match state_transition {
        StateTransition::DataContractCreate(transition) => {
            participants.identity(state_transition_owner(state_transition), Role::Sender);
            participants.data_contract_id = Some(transition.data_contract().id());
        }
        StateTransition::DataContractUpdate(transition) => {
            participants.identity(state_transition_owner(state_transition), Role::Sender);
            participants.data_contract_id = Some(transition.data_contract().id());
        }
        StateTransition::ContractUserModeration(transition) => {
            participants.identity(state_transition_owner(state_transition), Role::Sender);
            if let Some(target) = transition.target_identity_id() {
                participants.identity(target, Role::Recipient);
            }
            participants.data_contract_id = Some(transition.data_contract_id());
        }
        StateTransition::ContractFeeClaim(transition) => {
            // The claimant signs the claim and receives the claimed credits.
            let claimant = state_transition_owner(state_transition);
            participants.identity(claimant, Role::Sender);
            participants.identity(claimant, Role::Recipient);
            participants.data_contract_id = Some(transition.data_contract_id());
        }
        StateTransition::Batch(_)
        | StateTransition::IdentityCreditWithdrawal(_)
        | StateTransition::IdentityUpdate(_)
        | StateTransition::IdentityKeyLimitsUpdate(_)
        | StateTransition::MasternodeVote(_) => {
            participants.identity(state_transition_owner(state_transition), Role::Sender);
        }
        StateTransition::IdentityCreate(transition) => {
            participants.identity(transition.identity_id(), Role::Recipient);
        }
        StateTransition::IdentityTopUp(transition) => {
            participants.identity(*transition.identity_id(), Role::Recipient);
        }
        StateTransition::IdentityCreditTransfer(transition) => {
            participants.identity(transition.identity_id(), Role::Sender);
            participants.identity(transition.recipient_id(), Role::Recipient);
        }
        StateTransition::IdentityCreditTransferToAddresses(transition) => {
            participants.identity(transition.identity_id(), Role::Sender);
            participants.outputs(transition.recipient_addresses());
        }
        StateTransition::IdentityCreateFromAddresses(transition) => {
            participants.inputs(transition.inputs());
            if let Some((address, _)) = transition.output() {
                participants.address(*address, Role::Recipient);
            }
            if let Ok(identity_id) = transition.identity_id_from_inputs() {
                participants.identity(identity_id, Role::Recipient);
            }
        }
        StateTransition::IdentityTopUpFromAddresses(transition) => {
            participants.inputs(transition.inputs());
            if let Some((address, _)) = transition.output() {
                participants.address(*address, Role::Recipient);
            }
            participants.identity(*transition.identity_id(), Role::Recipient);
        }
        StateTransition::AddressFundsTransfer(transition) => {
            participants.inputs(transition.inputs());
            participants.outputs(transition.outputs());
        }
        StateTransition::AddressFundingFromAssetLock(transition) => {
            participants.inputs(transition.inputs());
            participants.outputs(transition.outputs());
        }
        StateTransition::AddressCreditWithdrawal(transition) => {
            participants.inputs(transition.inputs());
            if let Some((address, _)) = transition.output() {
                participants.address(*address, Role::Recipient);
            }
        }
        StateTransition::Shield(transition) => {
            participants.inputs(transition.inputs());
        }
        // Shielded senders and recipients are not public.
        StateTransition::ShieldedTransfer(_) | StateTransition::ShieldedWithdrawal(_) => {}
        StateTransition::Unshield(transition) => {
            participants.address(*transition.output_address(), Role::Recipient);
        }
        StateTransition::ShieldFromAssetLock(transition) => {
            if let Some(address) = transition.surplus_output() {
                participants.address(*address, Role::Recipient);
            }
        }
        StateTransition::IdentityCreateFromShieldedPool(transition) => {
            participants.identity(transition.identity_id(), Role::Recipient);
            // Credited instead when the identity cannot be created.
            participants.address(
                *transition.send_to_address_on_creation_failure(),
                Role::Recipient,
            );
        }
        StateTransition::ShieldFromIdentity(transition) => {
            participants.identity(transition.identity_id(), Role::Sender);
        }
        StateTransition::IdentityTopUpFromShieldedPool(transition) => {
            participants.identity(transition.identity_id(), Role::Recipient);
        }
        // Shielded token transitions paying their fee from the shielded pool: the token is
        // public, its senders and holders are not.
        StateTransition::TokenShieldedTransferWithShieldedFee(transition) => {
            participants.token_id = Some(transition.token_id());
        }
        StateTransition::TokenPurchaseFromShieldedPool(transition) => {
            participants.token_id = Some(transition.token_id());
        }
        StateTransition::TokenUnshieldWithShieldedFee(transition) => {
            participants.token_id = Some(transition.token_id());
            participants.identity(transition.recipient_id(), Role::Recipient);
        }
    }
    participants
}

/// The owner of a transition that has one; every caller passes such a transition.
fn state_transition_owner(state_transition: &StateTransition) -> Identifier {
    state_transition.owner_id().unwrap_or_default()
}

/// The identity a batch's inner document transition makes the document's new owner: a
/// transfer's recipient, or for a purchase the batch owner, who buys it.
pub(super) fn document_recipient(
    transition: &DocumentTransition,
    batch_owner_id: Identifier,
) -> Option<Identifier> {
    match transition {
        DocumentTransition::Transfer(transfer) => Some(transfer.recipient_owner_id()),
        DocumentTransition::Purchase(_) => Some(batch_owner_id),
        _ => None,
    }
}

/// The identity a batch's inner token transition names as recipient or target: a transfer's
/// recipient, a mint's explicit `issued_to` identity, a freeze, unfreeze or frozen-funds
/// destruction target, and the batch owner for a claim or a direct purchase, which credit the
/// claimant or buyer. A mint without `issued_to` credits the contract's configured
/// destination, which the transition does not name.
pub(super) fn token_recipient(
    transition: &TokenTransition,
    batch_owner_id: Identifier,
) -> Option<Identifier> {
    match transition {
        TokenTransition::Transfer(transfer) => Some(transfer.recipient_id()),
        TokenTransition::Mint(mint) => mint.issued_to_identity_id(),
        TokenTransition::Freeze(freeze) => Some(freeze.frozen_identity_id()),
        TokenTransition::Unfreeze(unfreeze) => Some(unfreeze.frozen_identity_id()),
        TokenTransition::DestroyFrozenFunds(destroy) => Some(destroy.frozen_identity_id()),
        TokenTransition::Claim(_) | TokenTransition::DirectPurchase(_) => Some(batch_owner_id),
        TokenTransition::Unshield(unshield) => Some(unshield.recipient_id()),
        // Tokens moved into or within the shielded pool go to holders that are not public.
        TokenTransition::Shield(_)
        | TokenTransition::ShieldedTransfer(_)
        | TokenTransition::MintToPool(_)
        | TokenTransition::BurnFromPool(_)
        | TokenTransition::ClaimToPool(_)
        | TokenTransition::DirectPurchaseToPool(_)
        | TokenTransition::Burn(_)
        | TokenTransition::EmergencyAction(_)
        | TokenTransition::ConfigUpdate(_)
        | TokenTransition::SetPriceForDirectPurchase(_) => None,
    }
}

/// The token a batch's inner token transition acts on.
pub(super) fn token_id(transition: &TokenTransition) -> Identifier {
    transition.token_id()
}
