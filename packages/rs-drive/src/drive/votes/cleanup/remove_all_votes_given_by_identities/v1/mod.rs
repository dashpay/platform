use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::votes::paths::{
    vote_contested_resource_identity_votes_tree_path_for_identity,
    vote_contested_resource_identity_votes_tree_path_vec,
    vote_decisions_identity_votes_tree_path_vec, vote_decisions_poll_choice_votes_tree_path_vec,
    vote_decisions_tree_path, IDENTITY_VOTES_TREE_KEY,
};
use crate::drive::votes::storage_form::contested_document_resource_reference_storage_form::ContestedDocumentResourceVoteReferenceStorageForm;
use crate::drive::votes::storage_form::yes_no_vote_reference_storage_form::YesNoVoteReferenceStorageForm;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::GroveError;
use crate::util::grove_operations::{BatchDeleteApplyType, DirectQueryType};
use dpp::platform_serialization::PLATFORM_BINCODE_CONFIG;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{MaybeTree, TransactionArg, TreeType};
use std::collections::BTreeSet;

impl Drive {
    /// Version 1 (protocol version 15) removes the contested resource votes of the removed
    /// masternodes, then their yes/no votes in a batch of its own: the vote under the poll's
    /// choice sum tree, and each removed masternode's tree in the decisions identity votes
    /// index, entries included. The two batches touch different subtrees, so applying them one
    /// after the other leaves the same state as one batch would.
    ///
    /// It only runs from protocol version 15, long after the mainnet heights version 0 treats
    /// apart, so it takes neither the height nor the network.
    pub(super) fn remove_all_votes_given_by_identities_v1(
        &self,
        identity_ids_as_byte_arrays: Vec<Vec<u8>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.remove_contested_resource_votes_given_by_identities_v1(
            identity_ids_as_byte_arrays.clone(),
            transaction,
            platform_version,
        )?;

        // Yes/no votes: the index item names the poll and the answer, so the vote sits under
        // the poll's tree at the answer's sum tree. A chain that activated protocol version 15
        // before the decisions trees existed has no identity votes index, and no yes/no votes.
        let has_decisions_index = self.grove_has_raw(
            (&vote_decisions_tree_path()).into(),
            &[IDENTITY_VOTES_TREE_KEY as u8],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;
        if !has_decisions_index {
            return Ok(());
        }

        let mut deletion_batch = vec![];
        let identity_vote_entries = self.identity_vote_entries_of_identities_v0(
            vote_decisions_identity_votes_tree_path_vec(),
            identity_ids_as_byte_arrays.clone(),
            transaction,
            platform_version,
        )?;
        let identities_with_entries: BTreeSet<Vec<u8>> =
            identity_vote_entries.keys().cloned().collect();
        for (identifier_bytes, votes_to_remove) in identity_vote_entries {
            for (vote_poll_id, vote_to_remove) in votes_to_remove {
                let storage_form =
                    YesNoVoteReferenceStorageForm::deserialize(&vote_to_remove.into_item_bytes()?)?;
                let vote_poll_id = Identifier::from_vec(vote_poll_id)?;
                let votes_path = vote_decisions_poll_choice_votes_tree_path_vec(
                    vote_poll_id.as_bytes(),
                    storage_form.vote_choice,
                );
                self.batch_delete(
                    votes_path.as_slice().into(),
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

        // Then each removed masternode's tree in the identity votes index, emptied or not: the
        // close of a poll removes the poll's entries but keeps the tree, which no one else
        // removes. It holds only items, so it is dropped flat with them. A masternode with
        // entries has its tree; one without is read, since the close of its last poll leaves
        // its tree empty.
        let identity_votes_path = vote_decisions_identity_votes_tree_path_vec();
        for identifier_bytes in identity_ids_as_byte_arrays {
            let has_identity_votes_tree = identities_with_entries.contains(&identifier_bytes)
                || self.grove_has_raw(
                    identity_votes_path.as_slice().into(),
                    identifier_bytes.as_slice(),
                    DirectQueryType::StatefulDirectQuery,
                    transaction,
                    &mut vec![],
                    &platform_version.drive,
                )?;
            if has_identity_votes_tree {
                deletion_batch.push(LowLevelDriveOperation::for_known_path_key_drop_flat_tree(
                    identity_votes_path.clone(),
                    identifier_bytes,
                    TreeType::NormalTree,
                ));
            }
        }

        if !deletion_batch.is_empty() {
            self.apply_batch_low_level_drive_operations(
                None,
                transaction,
                deletion_batch,
                &mut vec![],
                &platform_version.drive,
            )
            .inspect_err(|err| tracing::error!(?err, "yes/no vote deletion batch failed"))?;
        }

        Ok(())
    }

    /// Removes every contested resource vote reference of the identities, and the vote each
    /// one points at while that vote is stored: the vote is looked up at the path its
    /// reference names, and deleted there when it is found. A vote two references point at is
    /// deleted once.
    fn remove_contested_resource_votes_given_by_identities_v1(
        &self,
        identity_ids_as_byte_arrays: Vec<Vec<u8>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // We first query for all vote references that the identities have

        let votes_to_remove_by_identity_id = self.identity_vote_entries_of_identities_v0(
            vote_contested_resource_identity_votes_tree_path_vec(),
            identity_ids_as_byte_arrays,
            transaction,
            platform_version,
        )?;

        // Then we take each vote reference and go looking for its vote (to remove it)

        let mut deletion_batch = vec![];

        for (identifier_bytes, votes_to_remove) in votes_to_remove_by_identity_id {
            let identity_id = Identifier::from_vec(identifier_bytes.clone())?;
            let vote_path_ref = vote_contested_resource_identity_votes_tree_path_for_identity(
                identity_id.as_bytes(),
            );

            // The paths this identity's votes were already deleted at
            let mut deleted_vote_paths = BTreeSet::new();

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
                let bincode_config = PLATFORM_BINCODE_CONFIG;
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

                absolute_path.pop(); // we need to get rid of the key (which is the identifier bytes)

                let absolute_path_ref: Vec<_> =
                    absolute_path.iter().map(|a| a.as_slice()).collect();

                // The vote is deleted only while it is stored at the path its reference names
                let vote_is_stored = match self.grove_get_raw_optional(
                    absolute_path_ref.as_slice().into(),
                    identifier_bytes.as_slice(),
                    DirectQueryType::StatefulDirectQuery,
                    transaction,
                    &mut vec![],
                    &platform_version.drive,
                ) {
                    Ok(vote) => vote.is_some(),
                    Err(Error::GroveDB(error))
                        if matches!(
                            *error,
                            GroveError::PathNotFound(_)
                                | GroveError::PathParentLayerNotFound(_)
                                | GroveError::PathKeyNotFound(_)
                        ) =>
                    {
                        false
                    }
                    Err(error) => return Err(error),
                };

                if !vote_is_stored || !deleted_vote_paths.insert(absolute_path.clone()) {
                    continue;
                }

                // we then need to add to the batch the deletion

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
}

#[cfg(test)]
mod tests {
    use crate::drive::votes::paths::{
        vote_contested_resource_identity_votes_tree_path_for_identity,
        vote_contested_resource_identity_votes_tree_path_vec,
        vote_contested_resource_tree_path_vec,
    };
    use crate::drive::votes::storage_form::contested_document_resource_reference_storage_form::ContestedDocumentResourceVoteReferenceStorageForm;
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::dashcore::Network;
    use dpp::platform_serialization::PLATFORM_BINCODE_CONFIG;
    use dpp::version::PlatformVersion;
    use grovedb::element::reference_path::ReferencePathType;
    use grovedb::Element;

    const IDENTITY_ID: [u8; 32] = [7u8; 32];
    const VOTE_ID: [u8; 32] = [9u8; 32];
    /// The id a second reference to the same vote is stored under
    const OTHER_VOTE_ID: [u8; 32] = [10u8; 32];
    /// Subtree under the contested resource tree that holds the vote.
    const VOTE_TREE_KEY: &[u8] = b"t";

    fn insert(drive: &Drive, path: &[&[u8]], key: &[u8], element: Element) {
        drive
            .grove
            .insert(
                path,
                key,
                element,
                None,
                None,
                &PlatformVersion::latest().drive.grove_version,
            )
            .unwrap()
            .expect("expected to insert");
    }

    fn exists(drive: &Drive, path: &[&[u8]], key: &[u8]) -> bool {
        match drive
            .grove
            .get_raw(
                path.into(),
                key,
                None,
                &PlatformVersion::latest().drive.grove_version,
            )
            .unwrap()
        {
            Ok(_) => true,
            Err(grovedb::Error::PathKeyNotFound(_)) => false,
            Err(error) => panic!("unexpected error: {error}"),
        }
    }

    fn vote_tree_path() -> Vec<Vec<u8>> {
        let mut path = vote_contested_resource_tree_path_vec();
        path.push(VOTE_TREE_KEY.to_vec());
        path
    }

    /// The tree that holds the vote, where the real paths have the trees of a contender.
    fn insert_vote_tree(drive: &Drive) {
        let path = vote_contested_resource_tree_path_vec();
        let path: Vec<&[u8]> = path.iter().map(|part| part.as_slice()).collect();
        insert(drive, &path, VOTE_TREE_KEY, Element::empty_tree());
    }

    fn insert_vote(drive: &Drive) {
        let path = vote_tree_path();
        let path: Vec<&[u8]> = path.iter().map(|part| part.as_slice()).collect();
        // A sum item in the real paths; a delete only needs it not to be a tree
        insert(drive, &path, &IDENTITY_ID, Element::new_item(vec![1]));
    }

    fn vote_exists(drive: &Drive) -> bool {
        let path = vote_tree_path();
        let path: Vec<&[u8]> = path.iter().map(|part| part.as_slice()).collect();
        exists(drive, &path, &IDENTITY_ID)
    }

    /// The references of `IDENTITY_ID` to its vote, one under each of `vote_ids`.
    fn insert_vote_references(drive: &Drive, vote_ids: &[[u8; 32]]) {
        let path = vote_contested_resource_identity_votes_tree_path_vec();
        let path: Vec<&[u8]> = path.iter().map(|part| part.as_slice()).collect();
        insert(drive, &path, &IDENTITY_ID, Element::empty_tree());

        let storage_form = ContestedDocumentResourceVoteReferenceStorageForm {
            reference_path_type:
                ReferencePathType::UpstreamRootHeightWithParentPathAdditionReference(
                    2,
                    vec![VOTE_TREE_KEY.to_vec()],
                ),
            identity_vote_times: 1,
        };
        let encoded_reference = bincode::encode_to_vec(storage_form, PLATFORM_BINCODE_CONFIG)
            .expect("expected to encode the reference");

        for vote_id in vote_ids {
            insert(
                drive,
                vote_contested_resource_identity_votes_tree_path_for_identity(&IDENTITY_ID)
                    .as_slice(),
                vote_id,
                Element::new_item(encoded_reference.clone()),
            );
        }
    }

    fn vote_reference_exists(drive: &Drive, vote_id: &[u8; 32]) -> bool {
        exists(
            drive,
            vote_contested_resource_identity_votes_tree_path_for_identity(&IDENTITY_ID).as_slice(),
            vote_id,
        )
    }

    fn remove_votes(drive: &Drive) {
        drive
            .remove_all_votes_given_by_identities(
                vec![IDENTITY_ID.to_vec()],
                1,
                Network::Testnet,
                "test",
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to remove the votes");
    }

    #[test]
    fn should_remove_a_vote_and_its_reference() {
        let drive = setup_drive_with_initial_state_structure(None);
        insert_vote_tree(&drive);
        insert_vote(&drive);
        insert_vote_references(&drive, &[VOTE_ID]);

        remove_votes(&drive);

        assert!(!vote_reference_exists(&drive, &VOTE_ID));
        assert!(!vote_exists(&drive));
    }

    #[test]
    fn should_remove_a_reference_whose_vote_tree_is_gone() {
        let drive = setup_drive_with_initial_state_structure(None);
        insert_vote_references(&drive, &[VOTE_ID]);

        remove_votes(&drive);

        assert!(!vote_reference_exists(&drive, &VOTE_ID));
    }

    #[test]
    fn should_remove_a_reference_whose_vote_is_gone_from_its_tree() {
        let drive = setup_drive_with_initial_state_structure(None);
        insert_vote_tree(&drive);
        insert_vote_references(&drive, &[VOTE_ID]);

        remove_votes(&drive);

        assert!(!vote_reference_exists(&drive, &VOTE_ID));
    }

    #[test]
    fn should_remove_a_vote_two_references_point_at() {
        let drive = setup_drive_with_initial_state_structure(None);
        insert_vote_tree(&drive);
        insert_vote(&drive);
        insert_vote_references(&drive, &[VOTE_ID, OTHER_VOTE_ID]);

        remove_votes(&drive);

        assert!(!vote_reference_exists(&drive, &VOTE_ID));
        assert!(!vote_reference_exists(&drive, &OTHER_VOTE_ID));
        assert!(!vote_exists(&drive));
    }
}
