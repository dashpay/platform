use crate::drive::votes::paths::{VotePollPaths, RESOURCE_STORED_INFO_KEY_U8_32};
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo::PathKeyElement;
use dpp::serialization::PlatformSerializable;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn insert_stored_info_for_contested_resource_vote_poll_v0(
        &self,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        finalized_contested_document_vote_poll_stored_info: ContestedDocumentVotePollStoredInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self
            .insert_stored_info_for_contested_resource_vote_poll_operations_v0(
                vote_poll,
                finalized_contested_document_vote_poll_stored_info,
                platform_version,
            )?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok(())
    }

    pub(super) fn insert_stored_info_for_contested_resource_vote_poll_operations_v0(
        &self,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        finalized_contested_document_vote_poll_stored_info: ContestedDocumentVotePollStoredInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let serialization =
            finalized_contested_document_vote_poll_stored_info.serialize_consume_to_bytes()?;
        let vote_poll_root_path = vote_poll.contenders_path(platform_version)?;

        self.batch_insert::<0>(
            PathKeyElement((
                vote_poll_root_path.clone(),
                RESOURCE_STORED_INFO_KEY_U8_32.to_vec(),
                Element::new_item(serialization),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::object_size_info::DataContractOwnedResolvedInfo;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;

    fn dpns_vote_poll(
        document_type_name: &str,
        index_name: &str,
    ) -> ContestedDocumentResourceVotePollWithContractInfo {
        let pv = PlatformVersion::latest();
        let data_contract =
            get_dpns_data_contract_fixture(None, 0, pv.protocol_version).data_contract_owned();
        ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(data_contract),
            document_type_name: document_type_name.to_string(),
            index_name: index_name.to_string(),
            index_values: vec![],
        }
    }

    fn stored_info() -> ContestedDocumentVotePollStoredInfo {
        let pv = PlatformVersion::latest();
        ContestedDocumentVotePollStoredInfo::new(BlockInfo::default(), pv)
            .expect("construct default stored info")
    }

    /// insert_stored_info_operations propagates a ContestedIndexNotFound when the
    /// vote poll references an index name that does not exist on the document type.
    #[test]
    fn operations_rejects_missing_index_name() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let poll = dpns_vote_poll("domain", "no_such_index");
        let stored = stored_info();

        let err = drive
            .insert_stored_info_for_contested_resource_vote_poll_operations_v0(
                &poll,
                stored,
                platform_version,
            )
            .expect_err("missing index must fail");
        match err {
            Error::Drive(DriveError::ContestedIndexNotFound(_)) => {}
            other => panic!("expected ContestedIndexNotFound, got {:?}", other),
        }
    }

    /// When the document type on the poll does not exist on the contract, the
    /// operations path surfaces a DataContractError — not a panic.
    #[test]
    fn operations_rejects_missing_document_type() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let poll = dpns_vote_poll("no_such_doc_type", "parentNameAndLabel");
        let stored = stored_info();

        let err = drive
            .insert_stored_info_for_contested_resource_vote_poll_operations_v0(
                &poll,
                stored,
                platform_version,
            )
            .expect_err("missing document type must fail");
        // Either Error::Drive or Error::Protocol, but NEVER a panic.
        match err {
            Error::Drive(_) | Error::Protocol(_) => {}
            other => panic!("unexpected error variant: {:?}", other),
        }
    }
}
