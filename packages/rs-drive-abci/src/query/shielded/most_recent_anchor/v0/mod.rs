use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_most_recent_shielded_anchor_request::GetMostRecentShieldedAnchorRequestV0;
use dapi_grpc::platform::v0::get_most_recent_shielded_anchor_response::{
    get_most_recent_shielded_anchor_response_v0, GetMostRecentShieldedAnchorResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_credit_pool_path, shielded_credit_pool_path_vec, SHIELDED_MOST_RECENT_ANCHOR_KEY,
};
use drive::grovedb::{Element, PathQuery, Query, SizedQuery};
use drive::util::grove_operations::{DirectQueryType, GroveDBToUse};

impl<C> Platform<C> {
    pub(super) fn query_most_recent_shielded_anchor_v0(
        &self,
        GetMostRecentShieldedAnchorRequestV0 { prove }: GetMostRecentShieldedAnchorRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetMostRecentShieldedAnchorResponseV0>, Error> {
        let response = if prove {
            let path_query = PathQuery {
                path: shielded_credit_pool_path_vec(),
                query: SizedQuery {
                    query: Query::new_single_key(vec![SHIELDED_MOST_RECENT_ANCHOR_KEY]),
                    limit: Some(1),
                    offset: None,
                },
            };

            let proof = check_validation_result_with_data!(self.drive.grove_get_proved_path_query(
                &path_query,
                None,
                &mut vec![],
                &platform_version.drive,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetMostRecentShieldedAnchorResponseV0 {
                result: Some(get_most_recent_shielded_anchor_response_v0::Result::Proof(
                    proof,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let pool_path = shielded_credit_pool_path();

            let maybe_element = self.drive.grove_get_raw(
                (&pool_path).into(),
                &[SHIELDED_MOST_RECENT_ANCHOR_KEY],
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &platform_version.drive,
            )?;

            let anchor_bytes = match maybe_element {
                Some(Element::Item(bytes, _)) => bytes,
                _ => vec![0u8; 32],
            };

            GetMostRecentShieldedAnchorResponseV0 {
                result: Some(get_most_recent_shielded_anchor_response_v0::Result::Anchor(
                    anchor_bytes,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
