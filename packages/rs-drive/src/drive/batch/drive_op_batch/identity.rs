use crate::drive::batch::drive_op_batch::DriveOperationConverter;
use crate::drive::block_info::BlockInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use dpp::prelude::Revision;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// Operations on Identities
#[derive(Clone, Debug)]
pub enum IdentityOperationType {
    /// Inserts a new identity to the `Identities` subtree.
    AddNewIdentity {
        /// The identity we wish to insert
        identity: Identity,
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
        /// The keys to be added
        keys_to_add: Vec<IdentityPublicKey>,
    },
    /// Disable Identity Keys
    DisableIdentityKeys {
        /// The identity id of the identity
        identity_id: [u8; 32],
        /// The keys to be added
        keys_ids: Vec<KeyID>,
        /// The time at which they were disabled
        disable_at: TimestampMillis,
    },

    /// Updates an identities revision.
    UpdateIdentityRevision {
        /// The revision id
        identity_id: [u8; 32],
        /// The revision we are updating to
        revision: Revision,
    },
}

impl DriveOperationConverter for IdentityOperationType {
    fn to_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        match self {
            IdentityOperationType::AddNewIdentity { identity } => drive
                .add_insert_identity_operations(
                    identity,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                ),
            IdentityOperationType::AddToIdentityBalance {
                identity_id,
                added_balance,
            } => drive.add_to_identity_balance_operations(
                identity_id,
                added_balance,
                estimated_costs_only_with_layer_info,
                transaction,
            ),
            IdentityOperationType::RemoveFromIdentityBalance {
                identity_id,
                balance_to_remove,
            } => drive.remove_from_identity_balance_operations(
                identity_id,
                balance_to_remove,
                estimated_costs_only_with_layer_info,
                transaction,
            ),
            IdentityOperationType::AddNewKeysToIdentity {
                identity_id,
                keys_to_add,
            } => drive.add_new_keys_to_identity_operations(
                identity_id,
                keys_to_add,
                true,
                &block_info.epoch,
                estimated_costs_only_with_layer_info,
                transaction,
            ),
            IdentityOperationType::DisableIdentityKeys {
                identity_id,
                keys_ids,
                disable_at,
            } => drive.disable_identity_keys_operations(
                identity_id,
                keys_ids,
                disable_at,
                &block_info.epoch,
                estimated_costs_only_with_layer_info,
                transaction,
            ),
            IdentityOperationType::UpdateIdentityRevision {
                identity_id,
                revision,
            } => Ok(vec![drive.update_identity_revision_operation(
                identity_id,
                revision,
                estimated_costs_only_with_layer_info,
            )]),
        }
    }
}
