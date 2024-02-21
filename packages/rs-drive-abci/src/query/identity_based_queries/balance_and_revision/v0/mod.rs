use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryError;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_response::{GetIdentityBalanceAndRevisionResponseV0};
use dapi_grpc::platform::v0::{get_identity_balance_and_revision_response, GetIdentityBalanceAndRevisionRequest, GetIdentityBalanceAndRevisionResponse, Proof};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dapi_grpc::Message;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_request::GetIdentityBalanceAndRevisionRequestV0;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_response::get_identity_balance_and_revision_response_v0::BalanceAndRevision;
use dapi_grpc::platform::v0::get_identity_request::GetIdentityRequestV0;

impl<C> Platform<C> {
    pub(super) fn query_balance_and_revision_v0(
        &self,
        GetIdentityBalanceAndRevisionRequestV0 { id, prove }: GetIdentityBalanceAndRevisionRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityBalanceAndRevisionResponse>, Error> {
        let identity_id: Identifier =
            check_validation_result_with_data!(id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let response = if prove {
            let proof = self.drive.prove_identity_balance_and_revision(
                identity_id.into_buffer(),
                None,
                &platform_version.drive,
            )?;

            let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

            GetIdentityBalanceAndRevisionResponse {
                version: Some(get_identity_balance_and_revision_response::Version::V0(GetIdentityBalanceAndRevisionResponseV0 {
                    result: Some(get_identity_balance_and_revision_response::get_identity_balance_and_revision_response_v0::Result::Proof(proof)),
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
                    "No Identity balance found".to_string(),
                )));
            };

            let maybe_revision = self.drive.fetch_identity_revision(
                identity_id.into_buffer(),
                true,
                None,
                platform_version,
            )?;

            let Some(revision) = maybe_revision else {
                return Ok(ValidationResult::new_with_error(QueryError::NotFound(
                    "No Identity revision found".to_string(),
                )));
            };

            GetIdentityBalanceAndRevisionResponse {
                version: Some(get_identity_balance_and_revision_response::Version::V0(GetIdentityBalanceAndRevisionResponseV0 {
                    result: Some(
                        get_identity_balance_and_revision_response::get_identity_balance_and_revision_response_v0::Result::BalanceAndRevision(
                            BalanceAndRevision { balance, revision },
                        ),
                    ),
                    metadata: Some(self.response_metadata_v0()),
                })),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
