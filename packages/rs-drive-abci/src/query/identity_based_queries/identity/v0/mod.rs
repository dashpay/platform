use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_request::GetIdentityRequestV0;
use dapi_grpc::platform::v0::get_identity_response::{
    get_identity_response_v0, GetIdentityResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_identity_v0(
        &self,
        GetIdentityRequestV0 { id, prove }: GetIdentityRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityResponseV0>, Error> {
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

            GetIdentityResponseV0 {
                result: Some(get_identity_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
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

            GetIdentityResponseV0 {
                result: Some(get_identity_response_v0::Result::Identity(
                    serialized_identity,
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};

    #[test]
    fn test_invalid_identity_id() {
        let (platform, state, _unused_version) = setup_platform(false);

        let version = PlatformVersion::latest();

        let request = GetIdentityRequestV0 {
            id: vec![0; 8],
            prove: false,
        };

        let result = platform
            .query_identity_v0(request, &state, version)
            .expect("should execute query");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_identity_not_found() {
        let (platform, state, version) = setup_platform(false);

        let id = vec![0; 32];
        let request = GetIdentityRequestV0 {
            id: id.clone(),
            prove: false,
        };

        let result = platform
            .query_identity_v0(request, &state, version)
            .expect("should execute query");

        let validation_error = result.first_error().unwrap();

        assert!(matches!(
            validation_error,
            QueryError::NotFound(msg) if msg.contains("identity")
        ));
    }

    #[test]
    fn test_identity_absence_proof() {
        let (platform, state, version) = setup_platform(false);

        let id = vec![0; 32];
        let request = GetIdentityRequestV0 {
            id: id.clone(),
            prove: true,
        };

        let result = platform
            .query_identity_v0(request, &state, version)
            .expect("should execute query");

        assert!(matches!(
            result.data,
            Some(GetIdentityResponseV0 {
                result: Some(get_identity_response_v0::Result::Proof(_)),
                metadata: Some(_)
            })
        ))
    }
}
