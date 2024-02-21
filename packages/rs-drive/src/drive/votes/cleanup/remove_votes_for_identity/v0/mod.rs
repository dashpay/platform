use crate::drive::Drive;
use std::ops::RangeFull;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::grove_operations::BatchDeleteApplyType;
use crate::drive::votes::vote_contested_resource_identity_votes_tree_path_for_identity_vec;
use crate::query::QueryItem;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use dpp::voting::ContestedDocumentResourceVoteType;
use grovedb::query_result_type::QueryResultType::QueryElementResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use grovedb_path::SubtreePath;
use crate::drive::votes::TreePath;

impl Drive {
    /// We remove votes for an identity when that identity is somehow disabled. Currently there is
    /// no way to "disable" identities except for masternodes being removed from the list
    pub(super) fn remove_votes_for_identity_v0(
        &self,
        identity_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // We first query for all votes that the identity has

        let vote_path = vote_contested_resource_identity_votes_tree_path_for_identity_vec(
            identity_id.as_bytes(),
        );

        let path_query = PathQuery::new(
            vote_path,
            SizedQuery::new(
                Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
                Some(512),
                None,
            ),
        );

        let votes_to_remove_elements = self
            .grove_get_path_query(
                &path_query,
                transaction,
                QueryElementResultType,
                &mut vec![],
                &platform_version.drive,
            )?
            .0
            .to_elements();

        // Then we take each vote and go looking for it (to remove it)

        let mut deletion_batch = vec![];

        for vote_to_remove in votes_to_remove_elements {
            let Element::Item(vote, ..) = vote_to_remove else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "vote {:?} for identity {} is not an item",
                    vote_to_remove, identity_id
                ))));
            };

            let vote = ContestedDocumentResourceVoteType::deserialize_from_bytes(vote.as_slice())?;

            // we then need to add to the batch the deletion

            self.batch_delete(
                SubtreePath::from(vote.tree_path()),
                vote.tree_key(),
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some((false, false)),
                },
                transaction,
                &mut deletion_batch,
                &platform_version.drive,
            )?;
        }

        self.Ok(())
    }
}
