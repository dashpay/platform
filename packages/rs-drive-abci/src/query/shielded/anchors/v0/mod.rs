use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_anchors_request::GetShieldedAnchorsRequestV0;
use dapi_grpc::platform::v0::get_shielded_anchors_response::get_shielded_anchors_response_v0::Anchors;
use dapi_grpc::platform::v0::get_shielded_anchors_response::{
    get_shielded_anchors_response_v0, GetShieldedAnchorsResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::shielded_credit_pool_anchors_path_vec;
use drive::grovedb::query_result_type::QueryResultType;
use drive::grovedb::{PathQuery, Query, SizedQuery};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_shielded_anchors_v0(
        &self,
        GetShieldedAnchorsRequestV0 { prove }: GetShieldedAnchorsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedAnchorsResponseV0>, Error> {
        let path_query = PathQuery {
            path: shielded_credit_pool_anchors_path_vec(),
            query: SizedQuery {
                query: Query::new_range_full(),
                limit: None,
                offset: None,
            },
        };

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.grove_get_proved_path_query(
                &path_query,
                None,
                &mut vec![],
                &platform_version.drive,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetShieldedAnchorsResponseV0 {
                result: Some(get_shielded_anchors_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let (results, _) = self.drive.grove_get_raw_path_query(
                &path_query,
                None,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )?;

            // Anchors are stored as block_height_be → anchor_bytes; extract values
            let anchors: Vec<Vec<u8>> = results
                .to_key_elements()
                .into_iter()
                .filter_map(|(_key, element)| element.into_item_bytes().ok())
                .collect();

            GetShieldedAnchorsResponseV0 {
                result: Some(get_shielded_anchors_response_v0::Result::Anchors(Anchors {
                    anchors,
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
