use crate::drive::votes::paths::RESOURCE_STORED_INFO_KEY_U8_32;
use crate::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderVotePollPaths;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::GroveError;
use crate::util::grove_operations::DirectQueryType;
use dpp::serialization::PlatformDeserializableTrusted;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::IdentityContenderVotePollStoredInfo;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_identity_contender_vote_poll_stored_info_v0(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityContenderVotePollStoredInfo>, Error> {
        let path = vote_poll.poll_path_vec()?;
        let maybe_element = match self.grove_get_raw_optional(
            path.as_slice().into(),
            RESOURCE_STORED_INFO_KEY_U8_32.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        ) {
            Ok(maybe_element) => maybe_element,
            // The branch, and the poll's tree, exist only once a poll opened
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
        let Some(element) = maybe_element else {
            return Ok(None);
        };
        let bytes = element.into_item_bytes()?;
        Ok(Some(
            IdentityContenderVotePollStoredInfo::deserialize_from_bytes_trusted(&bytes)?,
        ))
    }
}
