use crate::drive::Drive;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfo,
    ContestedDocumentVotePollStoredInfoV0Getters,
};
use platform_version::version::ProtocolVersion;

impl Drive {
    /// The protocol version of the epoch a contest under way started in, when that epoch is
    /// not the block's
    #[inline(always)]
    pub(super) fn fetch_contest_opening_protocol_version_v0(
        &self,
        stored_info: &ContestedDocumentVotePollStoredInfo,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, Option<ProtocolVersion>), Error> {
        let ContestedDocumentVotePollStatus::Started(start_block) = stored_info.vote_poll_status()
        else {
            return Ok((FeeResult::default(), None));
        };
        // The block's own epoch runs the block's protocol version, and records it only at the
        // end of its first block
        if start_block.epoch.index == epoch.index {
            return Ok((FeeResult::default(), None));
        }
        // An earlier epoch that started a contest had a block, so its first block recorded its
        // protocol version, nothing removes that record, and it was committed before this
        // block started, so it is read without a transaction
        let mut cost_operations = vec![];
        let opening_protocol_version = self.get_epoch_protocol_version_operations(
            &start_block.epoch,
            None,
            &mut cost_operations,
            platform_version,
        )?;
        let fee_result =
            self.calculate_fee(None, Some(cost_operations), epoch, platform_version)?;
        Ok((fee_result, Some(opening_protocol_version)))
    }
}
