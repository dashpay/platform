use crate::drive::votes::paths::vote_contested_resource_identity_votes_tree_path_for_identity;
use crate::drive::votes::storage_form::contested_document_resource_reference_storage_form::ContestedDocumentResourceVoteReferenceStorageForm;
use crate::drive::votes::storage_form::contested_document_resource_storage_form::ContestedDocumentResourceVoteStorageForm;
use crate::drive::votes::tree_path_storage_form::TreePathStorageForm;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::identity::masternode_vote::v0::PreviousVoteCount;
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches the identities voting for contenders.
    pub fn fetch_identity_contested_resource_vote_v0(
        &self,
        masternode_pro_tx_hash: Identifier,
        vote_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(ResourceVoteChoice, PreviousVoteCount)>, Error> {
        let path = vote_contested_resource_identity_votes_tree_path_for_identity(
            masternode_pro_tx_hash.as_bytes(),
        );

        let optional_element = self.grove_get_raw_optional(
            (&path).into(),
            vote_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;

        optional_element
            .map(|element| {
                let serialized_reference = element.into_item_bytes()?;
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
                let absolute_path = reference
                    .reference_path_type
                    .absolute_path(path.as_slice(), Some(vote_id.as_slice()))?;
                let vote_storage_form =
                    ContestedDocumentResourceVoteStorageForm::try_from_tree_path(absolute_path)?;
                Ok((
                    vote_storage_form.resource_vote_choice,
                    reference.identity_vote_times,
                ))
            })
            .transpose()
    }
}
