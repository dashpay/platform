use crate::drive::votes::paths::{
    vote_contested_resource_identity_votes_tree_path_vec,
    vote_identity_contender_identity_votes_tree_path_vec,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::{GroveError, QueryItem};
use crate::util::grove_operations::BatchDeleteApplyType;
use crate::verify::bounded_decode::decode_vote_reference;
use dpp::dashcore::Network;
use dpp::prelude::{BlockHeight, Identifier};
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType;
use grovedb::{MaybeTree, PathQuery, Query, SizedQuery, TransactionArg};
use std::ops::RangeFull;

impl Drive {
    /// Removes every vote the given masternodes cast, on contested resources and on identity
    /// contender vote polls alike, along with the references they keep to them.
    pub(super) fn remove_all_votes_given_by_identities_v1(
        &self,
        identity_ids_as_byte_arrays: Vec<Vec<u8>>,
        _block_height: BlockHeight,
        _network: Network,
        _chain_id: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut deletion_batch = vec![];
        for identity_votes_path in [
            vote_contested_resource_identity_votes_tree_path_vec(),
            vote_identity_contender_identity_votes_tree_path_vec(),
        ] {
            self.remove_all_votes_given_by_identities_under_operations(
                identity_votes_path,
                identity_ids_as_byte_arrays.clone(),
                &mut deletion_batch,
                transaction,
                platform_version,
            )?;
        }
        if !deletion_batch.is_empty() {
            self.apply_batch_low_level_drive_operations(
                None,
                transaction,
                deletion_batch,
                &mut vec![],
                &platform_version.drive,
            )
            .inspect_err(|err| tracing::error!(?err, "vote deletion batch failed"))?;
        }
        Ok(())
    }

    /// The operations removing every vote referenced from the given masternodes' trees under
    /// one identity votes tree, and the references themselves. A tree that does not exist yet,
    /// as the identity contender one before the first such poll, holds nothing to remove.
    pub(in crate::drive::votes) fn remove_all_votes_given_by_identities_under_operations(
        &self,
        identity_votes_path: Vec<Vec<u8>>,
        identity_ids_as_byte_arrays: Vec<Vec<u8>>,
        deletion_batch: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut query = Query::new_with_direction(true);
        query.insert_keys(identity_ids_as_byte_arrays);
        let subquery = Query::new_single_query_item(QueryItem::RangeFull(RangeFull));
        query.set_subquery(subquery);
        let path_query = PathQuery::new(
            identity_votes_path.clone(),
            SizedQuery::new(query, None, None),
        );
        let votes_to_remove_by_identity_id = match self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryPathKeyElementTrioResultType,
            &mut vec![],
            &platform_version.drive,
        ) {
            Ok((results, _)) => results.to_last_path_to_key_elements_btree_map(),
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    GroveError::PathKeyNotFound(_)
                        | GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                ) =>
            {
                return Ok(());
            }
            Err(e) => return Err(e),
        };
        for (identifier_bytes, votes_to_remove) in votes_to_remove_by_identity_id {
            let identity_id = Identifier::from_vec(identifier_bytes.clone())?;
            let mut vote_path_ref = identity_votes_path.clone();
            vote_path_ref.push(identity_id.to_vec());
            let vote_path_ref: Vec<&[u8]> = vote_path_ref.iter().map(Vec::as_slice).collect();
            for (vote_id, vote_to_remove) in votes_to_remove {
                self.batch_delete(
                    vote_path_ref.as_slice().into(),
                    vote_id.as_slice(),
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    transaction,
                    deletion_batch,
                    &platform_version.drive,
                )?;
                let serialized_reference = vote_to_remove.into_item_bytes()?;
                let reference = decode_vote_reference(&serialized_reference)?;
                let mut absolute_path = reference
                    .reference_path_type
                    .absolute_path(vote_path_ref.as_slice(), Some(vote_id.as_slice()))?;
                absolute_path.pop(); // the key of the vote is the masternode's pro tx hash
                let absolute_path_ref: Vec<_> =
                    absolute_path.iter().map(|a| a.as_slice()).collect();
                self.batch_delete(
                    absolute_path_ref.as_slice().into(),
                    identifier_bytes.as_slice(),
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    transaction,
                    deletion_batch,
                    &platform_version.drive,
                )?;
            }
        }
        Ok(())
    }
}
