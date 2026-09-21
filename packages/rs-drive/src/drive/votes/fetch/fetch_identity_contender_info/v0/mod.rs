use crate::drive::votes::paths::IDENTITY_CONTENDER_INFO_KEY;
use crate::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderVotePollPaths;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::GroveError;
use crate::util::grove_operations::DirectQueryType;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializableTrusted;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::IdentityContenderInfo;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_identity_contender_info_v0(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        identity_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityContenderInfo>, Error> {
        let path = vote_poll.choice_path_vec(&ResourceVoteChoice::TowardsIdentity(identity_id))?;
        let maybe_element = match self.grove_get_raw_optional(
            path.as_slice().into(),
            &[IDENTITY_CONTENDER_INFO_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        ) {
            Ok(maybe_element) => maybe_element,
            // Neither the contender's tree nor, before any poll, the branch need exist
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
        maybe_element
            .map(|element| {
                let bytes = element.into_item_bytes()?;
                Ok(IdentityContenderInfo::deserialize_from_bytes_trusted(
                    &bytes,
                )?)
            })
            .transpose()
    }
}
