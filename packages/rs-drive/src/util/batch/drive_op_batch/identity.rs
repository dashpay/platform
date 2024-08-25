use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::block::block_info::BlockInfo;
use dpp::identity::{Identity, IdentityPublicKey, KeyID};
use dpp::prelude::{IdentityNonce, Revision};

use crate::drive::identity::update::methods::merge_identity_nonce::MergeIdentityContractNonceResultToResult;
use crate::drive::votes::resolved::votes::ResolvedVote;
use crate::state_transition_action::identity::masternode_vote::v0::PreviousVoteCount;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// Operations on Identities
#[derive(Clone, Debug)]
pub enum IdentityOperationType {
    /// Inserts a new identity to the `Identities` subtree.
    /// A masternode identity is an identity, but can not have unique keys.
    /// It also will skip testing for unique keys when adding non unique keys, so no one will
    /// take a key, then add it to a masternode
    AddNewIdentity {
        /// The identity we wish to insert
        identity: Identity,
        /// Is this identity a masternode identity
        /// On Masternode identities we do not add lookup key hashes
        is_masternode_identity: bool,
    },
    /// Adds balance to an identity
    AddToIdentityBalance {
        /// The identity id of the identity
        identity_id: [u8; 32],
        /// The added balance
        added_balance: u64,
    },
    /// Removes balance from an identity
    RemoveFromIdentityBalance {
        /// The identity id of the identity
        identity_id: [u8; 32],
        /// The balance that will be removed from the identity
        /// This needs to be verified in advance
        balance_to_remove: u64,
    },
    /// Adds an array of keys to the identity
    AddNewKeysToIdentity {
        /// The identity id of the identity
        identity_id: [u8; 32],
        /// The unique keys to be added
        unique_keys_to_add: Vec<IdentityPublicKey>,
        /// The non unique keys to be added
        non_unique_keys_to_add: Vec<IdentityPublicKey>,
    },
    /// Disable Identity Keys
    DisableIdentityKeys {
        /// The identity id of the identity
        identity_id: [u8; 32],
        /// The keys to be added
        keys_ids: Vec<KeyID>,
    },

    /// Re-Enable Identity Keys
    /// This should only be used internally in Drive (for masternode identities)
    ReEnableIdentityKeys {
        /// The identity id of the identity
        identity_id: [u8; 32],
        /// The keys to be added
        keys_ids: Vec<KeyID>,
    },

    /// Updates an identities revision.
    UpdateIdentityRevision {
        /// The revision id
        identity_id: [u8; 32],
        /// The revision we are updating to
        revision: Revision,
    },
    /// Casts a votes as a masternode.
    MasternodeCastVote {
        /// The pro tx hash of the masternode doing the voting
        voter_pro_tx_hash: [u8; 32],
        /// The strength of the vote, masternodes have 1, evonodes have 4,
        strength: u8,
        /// Contested Vote type
        vote: ResolvedVote,
        /// Remove previous contested resource vote choice
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
    },
    /// Updates an identities nonce for a specific contract.
    UpdateIdentityNonce {
        /// The revision id
        identity_id: [u8; 32],
        /// The nonce we are updating to
        nonce: IdentityNonce,
    },

    /// Updates an identities nonce for a specific contract.
    UpdateIdentityContractNonce {
        /// The revision id
        identity_id: [u8; 32],
        /// The contract id
        contract_id: [u8; 32],
        /// The nonce we are updating to
        nonce: IdentityNonce,
    },
}

impl DriveLowLevelOperationConverter for IdentityOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            IdentityOperationType::AddNewIdentity {
                identity,
                is_masternode_identity,
            } => drive.add_new_identity_operations(
                identity,
                is_masternode_identity,
                block_info,
                &mut None,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            IdentityOperationType::AddToIdentityBalance {
                identity_id,
                added_balance,
            } => drive.add_to_identity_balance_operations(
                identity_id,
                added_balance,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            IdentityOperationType::RemoveFromIdentityBalance {
                identity_id,
                balance_to_remove,
            } => drive.remove_from_identity_balance_operations(
                identity_id,
                balance_to_remove,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            IdentityOperationType::AddNewKeysToIdentity {
                identity_id,
                unique_keys_to_add,
                non_unique_keys_to_add,
            } => drive.add_new_keys_to_identity_operations(
                identity_id,
                unique_keys_to_add,
                non_unique_keys_to_add,
                true,
                &block_info.epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            IdentityOperationType::DisableIdentityKeys {
                identity_id,
                keys_ids,
            } => drive.disable_identity_keys_operations(
                identity_id,
                keys_ids,
                block_info.time_ms,
                &block_info.epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            IdentityOperationType::ReEnableIdentityKeys {
                identity_id,
                keys_ids,
            } => drive.re_enable_identity_keys_operations(
                identity_id,
                keys_ids,
                &block_info.epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            IdentityOperationType::UpdateIdentityRevision {
                identity_id,
                revision,
            } => Ok(vec![drive.update_identity_revision_operation(
                identity_id,
                revision,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?]),
            IdentityOperationType::MasternodeCastVote {
                voter_pro_tx_hash,
                strength,
                vote,
                previous_resource_vote_choice_to_remove,
            } => {
                // No need to have estimated_costs_only_with_layer_info and block_info here
                // This is because voting is a special operation with a fixed cost
                drive.register_identity_vote_operations(
                    voter_pro_tx_hash,
                    strength,
                    vote,
                    previous_resource_vote_choice_to_remove,
                    transaction,
                    platform_version,
                )
            }
            IdentityOperationType::UpdateIdentityContractNonce {
                identity_id,
                contract_id,
                nonce,
            } => {
                let (result, operations) = drive.merge_identity_contract_nonce_operations(
                    identity_id,
                    contract_id,
                    nonce,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                result.to_result()?;
                Ok(operations)
            }
            IdentityOperationType::UpdateIdentityNonce { identity_id, nonce } => {
                let (result, operations) = drive.merge_identity_nonce_operations(
                    identity_id,
                    nonce,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                result.to_result()?;
                Ok(operations)
            }
        }
    }
}
