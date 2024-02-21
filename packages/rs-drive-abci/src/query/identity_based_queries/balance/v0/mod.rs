use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_balance_request::GetIdentityBalanceRequestV0;
use dapi_grpc::platform::v0::get_identity_balance_response::GetIdentityBalanceResponseV0;
use dapi_grpc::platform::v0::{get_identity_balance_response, GetIdentityBalanceResponse};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_balance_v0(
        &self,
        GetIdentityBalanceRequestV0 { id, prove }: GetIdentityBalanceRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityBalanceResponse>, Error> {
        let identity_id: Identifier =
            check_validation_result_with_data!(id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_identity_balance(
                identity_id.into_buffer(),
                None,
                &platform_version.drive
            ));

            let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

            GetIdentityBalanceResponse {
                    version: Some(get_identity_balance_response::Version::V0(GetIdentityBalanceResponseV0 {
                        result: Some(get_identity_balance_response::get_identity_balance_response_v0::Result::Proof(proof)),
                        metadata: Some(metadata),
                    })),
                }
        } else {
            let maybe_balance = self.drive.fetch_identity_balance(
                identity_id.into_buffer(),
                None,
                platform_version,
            )?;

            let Some(balance) = maybe_balance else {
                return Ok(ValidationResult::new_with_error(QueryError::NotFound(
                    "No Identity found".to_string(),
                )));
            };

            GetIdentityBalanceResponse {
                    version: Some(get_identity_balance_response::Version::V0(GetIdentityBalanceResponseV0 {
                        result: Some(get_identity_balance_response::get_identity_balance_response_v0::Result::Balance(balance)),
                        metadata: Some(self.response_metadata_v0()),
                    })),
                }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
