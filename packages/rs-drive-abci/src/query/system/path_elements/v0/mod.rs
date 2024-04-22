use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_path_elements_request::GetPathElementsRequestV0;
use dapi_grpc::platform::v0::get_path_elements_response::get_path_elements_response_v0::Elements;
use dapi_grpc::platform::v0::get_path_elements_response::{
    get_path_elements_response_v0, GetPathElementsResponseV0,
};
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dpp::check_validation_result_with_data;

use crate::error::query::QueryError;
use crate::platform_types::platform_state::PlatformState;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;

impl<C> Platform<C> {
    pub(super) fn query_path_elements_v0(
        &self,
        GetPathElementsRequestV0 { path, keys, prove }: GetPathElementsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetPathElementsResponseV0>, Error> {
        if keys.len() > platform_version.drive_abci.query.max_returned_elements as usize {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!(
                    "trying to get {} values, maximum is {}",
                    keys.len(),
                    platform_version.drive_abci.query.max_returned_elements
                )),
            )));
        }
        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_elements(
                path,
                keys,
                None,
                platform_version
            ));

            GetPathElementsResponseV0 {
                result: Some(get_path_elements_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let result = check_validation_result_with_data!(self.drive.fetch_elements(
                path,
                keys,
                None,
                platform_version
            ));

            let serialized = result
                .into_iter()
                .map(|element| {
                    element
                        .serialize()
                        .map_err(|e| Error::Drive(drive::error::Error::GroveDB(e)))
                })
                .collect::<Result<Vec<Vec<u8>>, Error>>()?;

            GetPathElementsResponseV0 {
                result: Some(get_path_elements_response_v0::Result::Elements(Elements {
                    elements: serialized,
                })),
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
    use drive::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY;
    use drive::drive::RootTree;
    use drive::grovedb::Element;
    use integer_encoding::VarInt;

    #[test]
    fn test_query_total_system_credits_from_path_elements_query() {
        let (platform, state, version) = setup_platform();

        let platform_version = PlatformVersion::latest();

        platform
            .drive
            .add_to_system_credits(100, None, platform_version)
            .expect("expected to insert identity");

        let request = GetPathElementsRequestV0 {
            path: vec![vec![RootTree::Misc as u8]],
            keys: vec![TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec()],
            prove: false,
        };

        let response = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        let response_data = response.into_data().expect("expected data");

        let get_path_elements_response_v0::Result::Elements(mut elements) =
            response_data.result.expect("expected a result")
        else {
            panic!("expected elements")
        };

        assert_eq!(elements.elements.len(), 1);

        let element = Element::deserialize(elements.elements.remove(0).as_slice())
            .expect("expected to deserialize element");

        let Element::Item(value, _) = element else {
            panic!("expected item")
        };

        let (amount, _) = u64::decode_var(value.as_slice()).expect("expected amount");

        assert_eq!(amount, 100);
    }
}
