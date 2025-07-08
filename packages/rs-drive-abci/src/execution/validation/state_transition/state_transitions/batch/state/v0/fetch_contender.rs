use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::ContenderWithSerializedDocument;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQueryResultType, ResolvedContestedDocumentVotePollDriveQuery,
};

pub(crate) fn fetch_contender(
    drive: &Drive,
    vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
    contender_id: Identifier,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<(Option<ContenderWithSerializedDocument>, FeeResult), Error> {
    let contender_query = ResolvedContestedDocumentVotePollDriveQuery {
        vote_poll: vote_poll.into(),
        result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally, // Cheaper than anything else
        offset: None,
        limit: Some(1),
        start_at: Some((contender_id.0 .0, true)),
        allow_include_locked_and_abstaining_vote_tally: false,
    };

    let mut drive_operations = vec![];

    let mut result =
        contender_query.execute(drive, transaction, &mut drive_operations, platform_version)?;

    let fee = Drive::calculate_fee(
        None,
        Some(drive_operations),
        &block_info.epoch,
        drive.config.epochs_per_era,
        platform_version,
        None,
    )?;
    match result.contenders.pop() {
        None => Ok((None, fee)),
        Some(contender) => {
            if contender.identity_id() == contender_id {
                Ok((Some(contender), fee))
            } else {
                Ok((None, fee))
            }
        }
    }
}
