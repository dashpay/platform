use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_nullifiers_request::GetShieldedNullifiersRequestV0;
use dapi_grpc::platform::v0::get_shielded_nullifiers_response::get_shielded_nullifiers_response_v0::{
    NullifierStatus, NullifierStatuses,
};
use dapi_grpc::platform::v0::get_shielded_nullifiers_response::{
    get_shielded_nullifiers_response_v0, GetShieldedNullifiersResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_credit_pool_nullifiers_path, shielded_credit_pool_nullifiers_path_vec,
};
use drive::error::query::QuerySyntaxError;
use drive::grovedb::{PathQuery, Query, SizedQuery};
use drive::util::grove_operations::{DirectQueryType, GroveDBToUse};

impl<C> Platform<C> {
    pub(super) fn query_shielded_nullifiers_v0(
        &self,
        GetShieldedNullifiersRequestV0 { nullifiers, prove }: GetShieldedNullifiersRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedNullifiersResponseV0>, Error> {
        let max_elements = platform_version.drive_abci.query.max_returned_elements as usize;
        if nullifiers.len() > max_elements {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!(
                    "trying to check {} nullifiers, maximum is {}",
                    nullifiers.len(),
                    max_elements
                )),
            )));
        }

        if nullifiers.is_empty() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument("nullifiers list must not be empty".to_string()),
            ));
        }

        let response = if prove {
            let path_query = PathQuery {
                path: shielded_credit_pool_nullifiers_path_vec(),
                query: SizedQuery {
                    query: {
                        let mut q = Query::new();
                        q.insert_keys(nullifiers.clone());
                        q
                    },
                    limit: None,
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

            GetShieldedNullifiersResponseV0 {
                result: Some(get_shielded_nullifiers_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let nullifiers_path = shielded_credit_pool_nullifiers_path();

            let entries: Vec<NullifierStatus> = nullifiers
                .into_iter()
                .map(|nullifier| {
                    let is_spent = self.drive.grove_has_raw(
                        (&nullifiers_path).into(),
                        &nullifier,
                        DirectQueryType::StatefulDirectQuery,
                        None,
                        &mut vec![],
                        &platform_version.drive,
                    )?;

                    Ok(NullifierStatus {
                        nullifier,
                        is_spent,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;

            GetShieldedNullifiersResponseV0 {
                result: Some(
                    get_shielded_nullifiers_response_v0::Result::NullifierStatuses(
                        NullifierStatuses { entries },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
