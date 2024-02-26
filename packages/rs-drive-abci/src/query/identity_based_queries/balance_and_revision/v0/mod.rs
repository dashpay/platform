use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryError;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_response::{get_identity_balance_and_revision_response_v0, GetIdentityBalanceAndRevisionResponseV0};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_request::GetIdentityBalanceAndRevisionRequestV0;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_response::get_identity_balance_and_revision_response_v0::BalanceAndRevision;

impl<C> Platform<C> {
    pub(super) fn query_balance_and_revision_v0(
        &self,
        GetIdentityBalanceAndRevisionRequestV0 { id, prove }: GetIdentityBalanceAndRevisionRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityBalanceAndRevisionResponseV0>, Error> {
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

            GetIdentityBalanceAndRevisionResponseV0 {
                result: Some(get_identity_balance_and_revision_response_v0::Result::Proof(proof)),
                metadata: Some(metadata),
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

            GetIdentityBalanceAndRevisionResponseV0 {
                result: Some(
                    get_identity_balance_and_revision_response_v0::Result::BalanceAndRevision(
                        BalanceAndRevision { balance, revision },
                    ),
                ),
                metadata: Some(self.response_metadata_v0()),
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
        let (platform, version) = setup_platform();

        let request = GetIdentityBalanceAndRevisionRequestV0 {
            id: vec![0; 8],
            prove: false,
        };

        let result = platform
            .query_balance_and_revision_v0(request, version)
            .expect("should query balance and revision");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_identity_not_found_when_querying_balance_and_revision() {
        let (platform, version) = setup_platform();

        let id = vec![0; 32];

        let request = GetIdentityBalanceAndRevisionRequestV0 {
            id: id.clone(),
            prove: false,
        };

        let result = platform
            .query_balance_and_revision_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::NotFound(_)]
        ));
    }

    #[test]
    fn test_identity_balance_and_revision_absence_proof() {
        let (platform, version) = setup_platform();

        let id = vec![0; 32];

        let request = GetIdentityBalanceAndRevisionRequestV0 {
            id: id.clone(),
            prove: true,
        };

        let result = platform
            .query_balance_and_revision_v0(request, version)
            .expect("should query balance and revision");

        assert!(matches!(
            result.data,
            Some(GetIdentityBalanceAndRevisionResponseV0 {
                result: Some(get_identity_balance_and_revision_response_v0::Result::Proof(_)),
                metadata: Some(_)
            })
        ));
    }
}
