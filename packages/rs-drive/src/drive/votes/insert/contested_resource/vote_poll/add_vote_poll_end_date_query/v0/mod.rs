use crate::drive::Drive;
use crate::error::Error;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::voting::votes::Vote;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any votes poll should be closed.
    pub(super) fn add_vote_poll_end_date_query_v0(
        &self,
        contract_id: Vote,
        end_date: TimestampMillis,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
    }
}
