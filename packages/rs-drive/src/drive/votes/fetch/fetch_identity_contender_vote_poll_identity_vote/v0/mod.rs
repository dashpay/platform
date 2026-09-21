use crate::drive::votes::paths::vote_identity_contender_identity_votes_tree_path_for_identity;
use crate::drive::votes::storage_form::identity_contender_vote_storage_form::IdentityContenderVoteStorageForm;
use crate::drive::votes::tree_path_storage_form::TreePathStorageForm;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::GroveError;
use crate::state_transition_action::identity::masternode_vote::v0::PreviousVoteCount;
use crate::util::grove_operations::DirectQueryType;
use crate::verify::bounded_decode::decode_vote_reference;
use dpp::identifier::Identifier;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn fetch_identity_contender_vote_poll_identity_vote_v0(
        &self,
        masternode_pro_tx_hash: Identifier,
        vote_poll_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(ResourceVoteChoice, PreviousVoteCount)>, Error> {
        let path = vote_identity_contender_identity_votes_tree_path_for_identity(
            masternode_pro_tx_hash.as_bytes(),
        );
        let optional_element = match self.grove_get_raw_optional(
            (&path).into(),
            vote_poll_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(optional_element) => optional_element,
            // The masternode's tree, and the branch itself, exist only once someone voted
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    GroveError::PathKeyNotFound(_)
                        | GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                ) =>
            {
                None
            }
            Err(e) => return Err(e),
        };
        optional_element
            .map(|element| {
                let serialized_reference = element.into_item_bytes()?;
                let reference = decode_vote_reference(&serialized_reference)?;
                let absolute_path = reference
                    .reference_path_type
                    .absolute_path(path.as_slice(), Some(vote_poll_id.as_slice()))?;
                let vote_storage_form =
                    IdentityContenderVoteStorageForm::try_from_tree_path(absolute_path)?;
                Ok((
                    vote_storage_form.resource_vote_choice,
                    reference.identity_vote_times,
                ))
            })
            .transpose()
    }
}
