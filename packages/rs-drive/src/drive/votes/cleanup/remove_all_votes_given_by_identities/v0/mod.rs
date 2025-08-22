use crate::drive::Drive;
use std::ops::RangeFull;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::votes::paths::{
    vote_contested_resource_identity_votes_tree_path_for_identity,
    vote_contested_resource_identity_votes_tree_path_vec,
};
use crate::drive::votes::storage_form::contested_document_resource_reference_storage_form::ContestedDocumentResourceVoteReferenceStorageForm;
use crate::query::QueryItem;
use crate::util::grove_operations::BatchDeleteApplyType;
use dpp::dashcore::Network;
use dpp::prelude::{BlockHeight, Identifier};
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType;
use grovedb::{MaybeTree, PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    /// We remove votes for an identity when that identity is somehow disabled. Currently there is
    /// no way to "disable" identities except for masternodes being removed from the list
    pub(super) fn remove_all_votes_given_by_identities_v0(
        &self,
        identity_ids_as_byte_arrays: Vec<Vec<u8>>,
        block_height: BlockHeight,
        network: Network,
        chain_id: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // We first query for all vote_choices that the identity has

        let vote_path = vote_contested_resource_identity_votes_tree_path_vec();

        let mut query = Query::new_with_direction(true);

        query.insert_keys(identity_ids_as_byte_arrays);

        let subquery = Query::new_single_query_item(QueryItem::RangeFull(RangeFull));

        query.set_subquery(subquery);

        let path_query = PathQuery::new(vote_path.clone(), SizedQuery::new(query, None, None));

        let votes_to_remove_by_identity_id = self
            .grove_get_raw_path_query(
                &path_query,
                transaction,
                QueryPathKeyElementTrioResultType,
                &mut vec![],
                &platform_version.drive,
            )?
            .0
            .to_last_path_to_key_elements_btree_map();

        // Then we take each votes and go looking for it (to remove it)

        let mut deletion_batch = vec![];

        for (identifier_bytes, votes_to_remove) in votes_to_remove_by_identity_id {
            let identity_id = Identifier::from_vec(identifier_bytes.clone())?;
            let vote_path_ref = vote_contested_resource_identity_votes_tree_path_for_identity(
                identity_id.as_bytes(),
            );

            for (vote_id, vote_to_remove) in votes_to_remove {
                // We delete the vote item as reference
                self.batch_delete(
                    vote_path_ref.as_slice().into(),
                    vote_id.as_slice(),
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    transaction,
                    &mut deletion_batch,
                    &platform_version.drive,
                )?;

                let serialized_reference = vote_to_remove.into_item_bytes()?;
                let bincode_config = bincode::config::standard()
                    .with_big_endian()
                    .with_no_limit();
                let reference: ContestedDocumentResourceVoteReferenceStorageForm =
                    bincode::decode_from_slice(&serialized_reference, bincode_config)
                        .map_err(|e| {
                            Error::Drive(DriveError::CorruptedSerialization(format!(
                                "serialization of reference {} is corrupted: {}",
                                hex::encode(serialized_reference),
                                e
                            )))
                        })?
                        .0;
                let mut absolute_path = reference
                    .reference_path_type
                    .absolute_path(vote_path_ref.as_slice(), Some(vote_id.as_slice()))?;

                // we then need to add to the batch the deletion

                absolute_path.pop(); // we need to get rid of the key (which is the identifier bytes)

                let absolute_path_ref: Vec<_> =
                    absolute_path.iter().map(|a| a.as_slice()).collect();

                self.batch_delete(
                    absolute_path_ref.as_slice().into(),
                    identifier_bytes.as_slice(),
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    transaction,
                    &mut deletion_batch,
                    &platform_version.drive,
                )?;
            }
        }

        if !deletion_batch.is_empty() {
            // We had a sequence of errors on the mainnet started since block 32326.
            // We got RocksDB's "transaction is busy" error because of a bug (https://github.com/dashpay/platform/pull/2309).
            // Due to another bug in Tenderdash (https://github.com/dashpay/tenderdash/pull/966),
            // validators just proceeded to the next block partially committing the state
            // and updating the cache (https://github.com/dashpay/platform/pull/2305).
            // Full nodes are stuck and proceeded after re-sync.
            // For the mainnet chain, we enable this fix at the block when we consider the state is consistent.
            let transaction =
                if network == Network::Dash && chain_id == "evo1" && block_height < 33000 {
                    // Old behaviour on mainnet
                    None
                } else {
                    // We should use transaction
                    transaction
                };

            self.apply_batch_low_level_drive_operations(
                None,
                transaction,
                deletion_batch,
                &mut vec![],
                &platform_version.drive,
            )?;
        }

        Ok(())
    }
}
