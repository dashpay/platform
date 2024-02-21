use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_request::GetIdentityRequestV0;
use dapi_grpc::platform::v0::get_identity_response::GetIdentityResponseV0;
use dapi_grpc::platform::v0::{get_identity_response, GetIdentityResponse, Proof};
use dapi_grpc::Message;
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_identity_v0(
        &self,
        GetIdentityRequestV0 { id, prove }: GetIdentityRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityResponse>, Error> {
        let identity_id: Identifier =
            check_validation_result_with_data!(id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let response = if prove {
            let proof = self.drive.prove_full_identity(
                identity_id.into_buffer(),
                None,
                &platform_version.drive,
            )?;

            let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

            GetIdentityResponse {
                version: Some(get_identity_response::Version::V0(GetIdentityResponseV0 {
                    result: Some(
                        get_identity_response::get_identity_response_v0::Result::Proof(proof),
                    ),
                    metadata: Some(metadata),
                })),
            }
        } else {
            let maybe_identity = self.drive.fetch_full_identity(
                identity_id.into_buffer(),
                None,
                platform_version,
            )?;

            let identity = check_validation_result_with_data!(maybe_identity.ok_or_else(|| {
                QueryError::NotFound(format!("identity {} not found", identity_id))
            }));

            let serialized_identity = identity
                .serialize_consume_to_bytes()
                .map_err(Error::Protocol)?;

            GetIdentityResponse {
                version: Some(get_identity_response::Version::V0(GetIdentityResponseV0 {
                    result: Some(
                        get_identity_response::get_identity_response_v0::Result::Identity(
                            serialized_identity,
                        ),
                    ),
                    metadata: Some(self.response_metadata_v0()),
                })),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
