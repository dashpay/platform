use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::shielded::ShieldedPoolSelector;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_pool_state_request::GetShieldedPoolStateRequestV0;
use dapi_grpc::platform::v0::get_shielded_pool_state_response::{
    get_shielded_pool_state_response_v0, GetShieldedPoolStateResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::SHIELDED_TOTAL_BALANCE_KEY;
use drive::grovedb::{PathQuery, Query, SizedQuery};
use drive::util::grove_operations::{DirectQueryType, GroveDBToUse};

impl<C> Platform<C> {
    pub(super) fn query_shielded_pool_state_v0(
        &self,
        GetShieldedPoolStateRequestV0 { prove, token_id }: GetShieldedPoolStateRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedPoolStateResponseV0>, Error> {
        let pool = match ShieldedPoolSelector::from_request(token_id, platform_version) {
            Ok(pool) => pool,
            Err(error) => return Ok(QueryValidationResult::new_with_error(error)),
        };
        let response = if prove {
            let path_query = PathQuery {
                path: pool.pool_path_vec(),
                query: SizedQuery {
                    query: Query::new_single_key(vec![SHIELDED_TOTAL_BALANCE_KEY]),
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

            GetShieldedPoolStateResponseV0 {
                result: Some(get_shielded_pool_state_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let pool_path = pool.pool_path_vec();

            let total_balance = self
                .drive
                .grove_get_raw_value_u64_from_encoded_var_vec(
                    pool_path.as_slice().into(),
                    &[SHIELDED_TOTAL_BALANCE_KEY],
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )?
                .unwrap_or(0);

            GetShieldedPoolStateResponseV0 {
                result: Some(get_shielded_pool_state_response_v0::Result::TotalBalance(
                    total_balance,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
