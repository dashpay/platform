//! Which inputs decide each transition type's outcome.
//!
//! The rule for a type names the inputs the network takes when it executes the transition, or
//! when it fails with a paid error, and nothing else. A pool-paid shielded spend takes its
//! nullifiers and needs its anchor kept; an identity-signed transition takes its identity or
//! identity-contract nonce, read with the identity and contract its nonce validator reads; a
//! transition paid from addresses takes each input address's next nonce.
//!
//! Types funded by an asset lock have no proved query for the lock, and a masternode vote's
//! nonce belongs to a different identity depending on the protocol version, so those are refused
//! rather than judged from part of their inputs.

use crate::Error;
use dpp::address_funds::PlatformAddress;
use dpp::identifier::Identifier;
use dpp::prelude::{AddressNonce, IdentityNonce};
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::contract_fee_claim_transition::accessors::ContractFeeClaimTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_key_limits_update_transition::accessors::IdentityKeyLimitsUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::accessors::IdentityTopUpFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::private_document_create_from_shielded_pool_transition::accessors::PrivateDocumentCreateFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::shield_from_identity_transition::accessors::ShieldFromIdentityTransitionAccessorsV0;
use dpp::state_transition::shielded_transfer_transition::accessors::ShieldedTransferTransitionAccessorsV0;
use dpp::state_transition::shielded_withdrawal_transition::accessors::ShieldedWithdrawalTransitionAccessorsV0;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::accessors::TokenPurchaseFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::accessors::TokenShieldedTransferWithShieldedFeeTransitionAccessorsV0;
use dpp::state_transition::token_unshield_with_shielded_fee_transition::accessors::TokenUnshieldWithShieldedFeeTransitionAccessorsV0;
use dpp::state_transition::unshield_transition::accessors::UnshieldTransitionAccessorsV0;
use dpp::state_transition::{StateTransition, StateTransitionOwned};

/// The shielded pool a bundle spends from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShieldedPool {
    /// The credit shielded pool.
    Credit,
    /// The shielded pool of the token with this id.
    Token(Identifier),
}

/// One bundle of a pool-paid shielded spend: the pool it spends from, the nullifiers it records
/// there when it executes, and the anchor that pool has to keep for it to execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PoolSpend {
    pub pool: ShieldedPool,
    pub nullifiers: Vec<[u8; 32]>,
    pub anchor: [u8; 32],
}

/// A nonce a transition takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NonceInput {
    /// An identity's nonce.
    Identity {
        identity_id: Identifier,
        nonce: IdentityNonce,
    },
    /// An identity's nonce for one data contract.
    IdentityContract {
        identity_id: Identifier,
        contract_id: Identifier,
        nonce: IdentityNonce,
    },
    /// An input address's nonce: the transition carries the address's next one.
    Address {
        address: PlatformAddress,
        nonce: AddressNonce,
    },
}

/// The inputs that decide a transition's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TransitionInputs {
    /// A pool-paid shielded spend: every bundle that records nullifiers.
    Shielded(Vec<PoolSpend>),
    /// A transition that takes nonces.
    Nonces(Vec<NonceInput>),
}

impl TransitionInputs {
    /// The inputs of `transition`, or an error naming why its outcome cannot be proven.
    pub(super) fn of(transition: &StateTransition) -> Result<Self, Error> {
        let inputs = match transition {
            // Pool-paid shielded spends: one credit pool bundle.
            StateTransition::ShieldedTransfer(st) => credit_spend(st.nullifiers(), st.anchor()),
            StateTransition::Unshield(st) => credit_spend(st.nullifiers(), st.anchor()),
            StateTransition::ShieldedWithdrawal(st) => credit_spend(st.nullifiers(), st.anchor()),
            StateTransition::IdentityCreateFromShieldedPool(st) => {
                credit_spend(st.nullifiers(), st.anchor())
            }
            StateTransition::IdentityTopUpFromShieldedPool(st) => {
                credit_spend(st.nullifiers(), st.anchor())
            }
            StateTransition::PrivateDocumentCreateFromShieldedPool(st) => {
                credit_spend(st.nullifiers(), st.anchor())
            }
            // Token pool transitions pay their fee from a credit pool bundle and spend a token
            // pool bundle; both are recorded at once.
            StateTransition::TokenShieldedTransferWithShieldedFee(st) => {
                TransitionInputs::Shielded(vec![
                    PoolSpend {
                        pool: ShieldedPool::Credit,
                        nullifiers: st.fee_nullifiers(),
                        anchor: st.fee_anchor(),
                    },
                    PoolSpend {
                        pool: ShieldedPool::Token(st.token_id()),
                        nullifiers: st.token_nullifiers(),
                        anchor: st.token_anchor(),
                    },
                ])
            }
            StateTransition::TokenUnshieldWithShieldedFee(st) => TransitionInputs::Shielded(vec![
                PoolSpend {
                    pool: ShieldedPool::Credit,
                    nullifiers: st.fee_nullifiers(),
                    anchor: st.fee_anchor(),
                },
                PoolSpend {
                    pool: ShieldedPool::Token(st.token_id()),
                    nullifiers: st.token_nullifiers(),
                    anchor: st.token_anchor(),
                },
            ]),
            // A purchase mints its token bundle's notes into the pool and records no token pool
            // nullifier, so only the fee bundle decides it.
            StateTransition::TokenPurchaseFromShieldedPool(st) => {
                credit_spend(st.fee_nullifiers(), st.fee_anchor())
            }

            // Identity nonces, read for the identity each nonce validator reads.
            StateTransition::DataContractCreate(st) => {
                identity_nonce(st.data_contract().owner_id(), st.identity_nonce())
            }
            StateTransition::IdentityUpdate(st) => identity_nonce(st.identity_id(), st.nonce()),
            StateTransition::IdentityKeyLimitsUpdate(st) => {
                identity_nonce(st.identity_id(), st.nonce())
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                identity_nonce(st.identity_id(), st.nonce())
            }
            StateTransition::IdentityCreditTransfer(st) => {
                identity_nonce(st.identity_id(), st.nonce())
            }
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                identity_nonce(st.identity_id(), st.nonce())
            }
            StateTransition::ShieldFromIdentity(st) => identity_nonce(st.identity_id(), st.nonce()),

            // Identity-contract nonces.
            StateTransition::DataContractUpdate(st) => {
                TransitionInputs::Nonces(vec![NonceInput::IdentityContract {
                    identity_id: st.data_contract().owner_id(),
                    contract_id: st.data_contract().id(),
                    nonce: st.identity_contract_nonce(),
                }])
            }
            StateTransition::ContractUserModeration(st) => {
                TransitionInputs::Nonces(vec![NonceInput::IdentityContract {
                    identity_id: st.owner_id(),
                    contract_id: st.data_contract_id(),
                    nonce: st.identity_contract_nonce(),
                }])
            }
            StateTransition::ContractFeeClaim(st) => {
                TransitionInputs::Nonces(vec![NonceInput::IdentityContract {
                    identity_id: st.owner_id(),
                    contract_id: st.data_contract_id(),
                    nonce: st.identity_contract_nonce(),
                }])
            }
            StateTransition::Batch(st) => {
                let identity_id = st.owner_id();
                TransitionInputs::Nonces(
                    st.transitions_iter()
                        .map(|transition| NonceInput::IdentityContract {
                            identity_id,
                            contract_id: transition.data_contract_id(),
                            nonce: transition.identity_contract_nonce(),
                        })
                        .collect(),
                )
            }

            // Address nonces.
            StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::Shield(_) => TransitionInputs::Nonces(
                transition
                    .inputs()
                    .into_iter()
                    .flatten()
                    .map(|(address, (nonce, _))| NonceInput::Address {
                        address: *address,
                        nonce: *nonce,
                    })
                    .collect(),
            ),

            StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::ShieldFromAssetLock(_) => {
                return Err(unsupported(
                    transition,
                    "it is funded by an asset lock, and no proved query shows whether the lock \
                     is spent",
                ));
            }
            StateTransition::MasternodeVote(_) => {
                return Err(unsupported(
                    transition,
                    "the identity whose nonce it takes depends on the protocol version",
                ));
            }
        };
        inputs.ensure_not_empty(transition)?;
        Ok(inputs)
    }

    /// Refuse a transition with nothing to read: no read could show it executed or not.
    fn ensure_not_empty(&self, transition: &StateTransition) -> Result<(), Error> {
        let empty = match self {
            TransitionInputs::Shielded(spends) => {
                spends.is_empty() || spends.iter().any(|spend| spend.nullifiers.is_empty())
            }
            TransitionInputs::Nonces(nonces) => nonces.is_empty(),
        };
        if empty {
            return Err(Error::InvalidRequest(format!(
                "{} has no input whose proven state shows whether it executed",
                transition.name()
            )));
        }
        Ok(())
    }
}

fn credit_spend(nullifiers: Vec<[u8; 32]>, anchor: [u8; 32]) -> TransitionInputs {
    TransitionInputs::Shielded(vec![PoolSpend {
        pool: ShieldedPool::Credit,
        nullifiers,
        anchor,
    }])
}

fn identity_nonce(identity_id: Identifier, nonce: IdentityNonce) -> TransitionInputs {
    TransitionInputs::Nonces(vec![NonceInput::Identity { identity_id, nonce }])
}

fn unsupported(transition: &StateTransition, why: &str) -> Error {
    Error::InvalidRequest(format!(
        "no proven outcome for {}: {why}",
        transition.name()
    ))
}
