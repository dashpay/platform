use crate::drive::Drive;
use std::ops::RangeFull;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::grove_operations::BatchDeleteApplyType;
use crate::drive::votes::paths::{
    vote_contested_resource_identity_votes_tree_path_for_identity,
    vote_contested_resource_identity_votes_tree_path_for_identity_vec,
};
use crate::query::QueryItem;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType::QueryElementResultType;
use grovedb::reference_path::path_from_reference_path_type;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    /// We remove votes for an identity when that identity is somehow disabled. Currently there is
    /// no way to "disable" identities except for masternodes being removed from the list
    pub(super) fn remove_all_votes_given_by_identity_v0(
        &self,
        identity_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // We first query for all vote_choices that the identity has

        let vote_path = vote_contested_resource_identity_votes_tree_path_for_identity_vec(
            identity_id.as_bytes(),
        );

        let path_query = PathQuery::new(
            vote_path,
            SizedQuery::new(
                Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
                None, // Todo: there might be an issue if too many votes are removed at the same time
                None,
            ),
        );

        let votes_to_remove_elements = self
            .grove_get_raw_path_query(
                &path_query,
                transaction,
                QueryElementResultType,
                &mut vec![],
                &platform_version.drive,
            )?
            .0
            .to_elements();

        // Then we take each votes and go looking for it (to remove it)

        let mut deletion_batch = vec![];

        let vote_path_ref =
            vote_contested_resource_identity_votes_tree_path_for_identity(identity_id.as_bytes());

        for vote_to_remove in votes_to_remove_elements {
            let Element::Reference(vote_reference, ..) = vote_to_remove else {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "votes {:?} for identity {} are not a reference",
                    vote_to_remove, identity_id
                ))));
            };

            let mut absolute_path =
                path_from_reference_path_type(vote_reference, vote_path_ref.as_ref(), None)?;

            if absolute_path.is_empty() {
                return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                    "reference to vote for identity {} is empty",
                    identity_id
                ))));
            }
            let key = absolute_path.remove(absolute_path.len() - 1);

            // we then need to add to the batch the deletion

            let absolute_path_ref: Vec<_> = absolute_path.iter().map(|a| a.as_slice()).collect();

            self.batch_delete(
                absolute_path_ref.as_slice().into(),
                key.as_slice(),
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some((false, false)),
                },
                transaction,
                &mut deletion_batch,
                &platform_version.drive,
            )?;
        }

        Ok(())
    }
}
