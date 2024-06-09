use crate::drive::grove_operations::DirectQueryType;
use crate::drive::votes::paths::{VotePollPaths, RESOURCE_STORED_INFO_KEY_U8};
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::serialization::PlatformDeserializable;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches the contested document vote poll stored info
    pub(super) fn fetch_contested_document_vote_poll_stored_info_v0(
        &self,
        contested_document_resource_vote_poll_with_contract_info: &ContestedDocumentResourceVotePollWithContractInfo,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, Option<ContestedDocumentVotePollStoredInfo>), Error> {
        let path = contested_document_resource_vote_poll_with_contract_info
            .contenders_path(platform_version)?;
        let mut cost_operations = vec![];
        let maybe_element = self.grove_get_raw_optional(
            path.as_slice().into(),
            vec![RESOURCE_STORED_INFO_KEY_U8].as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut cost_operations,
            &platform_version.drive,
        )?;
        let fee_result = Drive::calculate_fee(
            None,
            Some(cost_operations),
            epoch,
            self.config.epochs_per_era,
            platform_version,
        )?;
        let Some(element) = maybe_element else {
            return Ok((fee_result, None));
        };
        let contested_start_info_bytes = element.into_item_bytes()?;
        let start_info = ContestedDocumentVotePollStoredInfo::deserialize_from_bytes(
            &contested_start_info_bytes,
        )?;
        Ok((fee_result, Some(start_info)))
    }
}
