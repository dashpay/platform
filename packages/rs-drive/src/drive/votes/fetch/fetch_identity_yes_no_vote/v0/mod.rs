use crate::drive::votes::paths::vote_decisions_identity_votes_tree_path_for_identity;
use crate::drive::votes::resolved::votes::resolved_yes_no_vote::PreviousYesNoVoteCount;
use crate::drive::votes::storage_form::yes_no_vote_reference_storage_form::YesNoVoteReferenceStorageForm;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_identity_yes_no_vote_v0(
        &self,
        masternode_pro_tx_hash: Identifier,
        vote_poll_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(YesNoAbstainVoteChoice, PreviousYesNoVoteCount)>, Error> {
        let path =
            vote_decisions_identity_votes_tree_path_for_identity(masternode_pro_tx_hash.as_bytes());
        let maybe_element = self.grove_get_raw_optional(
            (&path).into(),
            vote_poll_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;
        maybe_element
            .map(|element| {
                let bytes = element.into_item_bytes()?;
                let storage_form =
                    YesNoVoteReferenceStorageForm::deserialize(&bytes).map_err(|e| {
                        Error::Drive(DriveError::CorruptedSerialization(format!(
                            "serialization of yes/no vote reference {} is corrupted: {}",
                            hex::encode(bytes),
                            e
                        )))
                    })?;
                Ok((storage_form.vote_choice, storage_form.identity_vote_times))
            })
            .transpose()
    }
}
