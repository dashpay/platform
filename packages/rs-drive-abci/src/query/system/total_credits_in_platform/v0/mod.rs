use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_total_credits_in_platform_request::GetTotalCreditsInPlatformRequestV0;
use dapi_grpc::platform::v0::get_total_credits_in_platform_response::{get_total_credits_in_platform_response_v0, GetTotalCreditsInPlatformResponseV0};
use dpp::check_validation_result_with_data;

use crate::platform_types::platform_state::PlatformState;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::balances::{total_credits_on_platform_path_query, TOTAL_SYSTEM_CREDITS_STORAGE_KEY};
use drive::drive::system::misc_path;
use drive::util::grove_operations::DirectQueryType;

impl<C> Platform<C> {
    pub(super) fn query_total_credits_in_platform_v0(
        &self,
        GetTotalCreditsInPlatformRequestV0 { prove }: GetTotalCreditsInPlatformRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTotalCreditsInPlatformResponseV0>, Error> {
        let path_query = total_credits_on_platform_path_query();

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.grove_get_proved_path_query(
            &path_query,
            None,
            &mut vec![],
            &platform_version.drive,
        ));

            GetTotalCreditsInPlatformResponseV0 {
                result: Some(get_total_credits_in_platform_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let path_holding_total_credits = misc_path();
            let total_credits_in_platform = self.drive
                .grove_get_raw_value_u64_from_encoded_var_vec(
                    (&path_holding_total_credits).into(),
                    TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )?.unwrap_or_default(); // 0  would mean we haven't initialized yet

            GetTotalCreditsInPlatformResponseV0 {
                result: Some(get_total_credits_in_platform_response_v0::Result::Credits(total_credits_in_platform)),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    #[test]
    fn test_query_total_system_credits() {
        let (platform, state, version) = setup_platform(false);

        let platform_version = PlatformVersion::latest();

        platform
            .drive
            .add_to_system_credits(100, None, platform_version)
            .expect("expected to insert identity");

        let request = GetTotalCreditsInPlatformRequestV0 {
            prove: false,
        };

        let response = platform
            .query_total_credits_in_platform_v0(request, &state, version)
            .expect("expected query to succeed");

        let response_data = response.into_data().expect("expected data");

        let get_total_credits_in_platform_response_v0::Result::Credits(credits) =
            response_data.result.expect("expected a result")
        else {
            panic!("expected credits")
        };

        assert_eq!(credits, 100);
    }
}
