//! Pay out the fee pots of a data contract (protocol version 14).
//!
//! A document type may charge a fixed fee in credits for actions on its documents (the
//! `actionFees` keyword). The `owner` parts collect in the contract's owner pot and the
//! `moderators` parts in its moderators pot. A [`ContractFeeClaimTransition`], signed by a
//! CRITICAL authentication key, pays a pot out: the owner pot to the contract owner, who alone
//! may claim it, and the moderators pot in equal shares to the contract's moderation team (by
//! the proposal's reward split for an elected contract's seated team), any member of which may
//! claim it. A pot is paid out at most once per epoch.
//!
//! ```ignore
//! let claim = moderator_identity
//!     .claim_contract_fees(&sdk, contract_id, ContractFeePot::Moderators, None, signer, None)
//!     .await?;
//! ```

use dpp::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::platform_value::Identifier;
use dpp::state_transition::contract_fee_claim_transition::methods::ContractFeeClaimTransitionMethodsV0;
use dpp::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use std::collections::BTreeMap;

use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::contract_user_moderation::{
    ensure_provider_resolves_contract, signing_key_for_moderation,
};
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};

use super::waitable::Waitable;

/// A contract fee pot after a claim, as the proof of the claim shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedContractFees {
    /// The contract whose pot was paid out
    pub contract_id: Identifier,
    /// The pot that was paid out
    pub pot: ContractFeePot,
    /// The last claim of the pot, which is this claim unless the pot was claimed again since:
    /// its epoch, the time of its block and the identity that signed it
    pub last_claim: ContractFeePotLastClaim,
    /// The credits left in the pot: what the split left over, and any fee collected since the
    /// claim
    pub remaining_credits: Credits,
    /// The balance, after the claim, of every identity the contract names as a recipient of the
    /// pot; for a claim by a member of an elected contract's seated team, which the contract
    /// does not name, the claimant's balance alone
    pub balances: BTreeMap<Identifier, Credits>,
}

impl TryFrom<StateTransitionProofResult> for ClaimedContractFees {
    type Error = Error;

    fn try_from(value: StateTransitionProofResult) -> Result<Self, Self::Error> {
        match value {
            StateTransitionProofResult::VerifiedContractFeeClaim(
                contract_id,
                pot,
                last_claim,
                remaining_credits,
                balances,
            ) => Ok(Self {
                contract_id,
                pot,
                last_claim,
                remaining_credits,
                balances,
            }),
            other => Err(Error::Generic(format!(
                "expected a contract fee claim proof result, got {other}"
            ))),
        }
    }
}

#[async_trait::async_trait]
pub trait ClaimContractFees: Waitable {
    /// Pays out `pot` of `contract_id`, signed by this identity, and resolves once the pot and
    /// the balances it paid are proved. The contract owner claims the owner pot; any member of
    /// the contract's moderation team claims the moderators pot, for the whole team. If
    /// `signing_key_to_use` is not set, the first CRITICAL authentication key without contract
    /// bounds that the signer can sign with is used.
    ///
    /// The proof shows the pot and the balances, not this exact transition (the nonce is not
    /// stored), so it is waited for as affected state.
    async fn claim_contract_fees<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        pot: ContractFeePot,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ClaimedContractFees, Error>;
}

#[async_trait::async_trait]
impl ClaimContractFees for Identity {
    async fn claim_contract_fees<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        pot: ContractFeePot,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ClaimedContractFees, Error> {
        let signing_key_id = match signing_key_to_use {
            Some(key) => key.id(),
            None => signing_key_for_moderation(self, &signer)?,
        };

        // The proof of a claim covers the balance of every identity the pot pays, and the
        // verifier reads who they are from the contract through the context provider. The
        // moderation team can change by a contract update, and a verifier working from a copy
        // that names the old team would ask for other balances than the node proved and refuse
        // a claim that executed and was paid for. So the contract is fetched again, whatever
        // copy the provider holds.
        ensure_provider_resolves_contract(sdk, contract_id, true).await?;

        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(self.id(), contract_id, true, settings)
            .await?;
        let user_fee_increase = settings.and_then(|settings| settings.user_fee_increase);
        let state_transition = ContractFeeClaimTransition::try_from_identity_with_signer(
            self,
            &signing_key_id,
            contract_id,
            pot,
            identity_contract_nonce,
            user_fee_increase.unwrap_or_default(),
            &signer,
            sdk.version(),
            None,
        )
        .await?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

        state_transition
            .broadcast_and_wait_for_affected_state(sdk, settings)
            .await
    }
}
