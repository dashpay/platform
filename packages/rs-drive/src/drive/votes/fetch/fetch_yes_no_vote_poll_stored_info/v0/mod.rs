use crate::drive::votes::paths::{
    vote_decisions_active_polls_poll_tree_path_vec, YES_NO_VOTE_POLL_STORED_INFO_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializableTrusted;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStoredInfo;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_yes_no_vote_poll_stored_info_v0(
        &self,
        vote_poll_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<YesNoVotePollStoredInfo>, Error> {
        let path = vote_decisions_active_polls_poll_tree_path_vec(vote_poll_id.as_bytes());
        self.grove_get_raw_optional_item(
            path.as_slice().into(),
            &[YES_NO_VOTE_POLL_STORED_INFO_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?
        .map(|bytes| YesNoVotePollStoredInfo::deserialize_from_bytes_trusted(&bytes))
        .transpose()
        .map_err(Error::from)
    }
}
