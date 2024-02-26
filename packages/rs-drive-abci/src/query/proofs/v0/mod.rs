use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_proofs_request::GetProofsRequestV0;
use dapi_grpc::platform::v0::get_proofs_response::{get_proofs_response_v0, GetProofsResponseV0};
use dpp::check_validation_result_with_data;
use dpp::platform_value::Bytes32;
use dpp::prelude::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::identity::{IdentityDriveQuery, IdentityProveRequestType};
use drive::query::SingleDocumentDriveQuery;

impl<C> Platform<C> {
    pub(super) fn query_proofs_v0(
        &self,
        GetProofsRequestV0 {
            identities,
            contracts,
            documents,
        }: GetProofsRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetProofsResponseV0>, Error> {
        let contract_ids = check_validation_result_with_data!(contracts
            .into_iter()
            .map(|contract_request| {
                Bytes32::from_vec(contract_request.contract_id)
                    .map(|bytes| (bytes.0, None))
                    .map_err(|_| {
                        QueryError::InvalidArgument(
                            "id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })
            })
            .collect::<Result<Vec<([u8; 32], Option<bool>)>, QueryError>>());

        let identity_requests = check_validation_result_with_data!(identities
            .into_iter()
            .map(|identity_request| {
                Ok(IdentityDriveQuery {
                    identity_id: Bytes32::from_vec(identity_request.identity_id)
                        .map(|bytes| bytes.0)
                        .map_err(|_| {
                            QueryError::InvalidArgument(
                                "id must be a valid identifier (32 bytes long)".to_string(),
                            )
                        })?,
                    prove_request_type: IdentityProveRequestType::try_from(
                        identity_request.request_type as u8,
                    )
                    .map_err(|_| {
                        QueryError::InvalidArgument(
                            format!(
                                "invalid prove request type '{}'",
                                identity_request.request_type
                            )
                            .to_string(),
                        )
                    })?,
                })
            })
            .collect::<Result<Vec<IdentityDriveQuery>, QueryError>>());

        let document_queries = check_validation_result_with_data!(documents
            .into_iter()
            .map(|document_proof_request| {
                let contract_id: Identifier =
                    document_proof_request.contract_id.try_into().map_err(|_| {
                        QueryError::InvalidArgument(
                            "id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?;
                let document_id: Identifier =
                    document_proof_request.document_id.try_into().map_err(|_| {
                        QueryError::InvalidArgument(
                            "id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?;

                Ok(SingleDocumentDriveQuery {
                    contract_id: contract_id.into_buffer(),
                    document_type_name: document_proof_request.document_type,
                    document_type_keeps_history: document_proof_request.document_type_keeps_history,
                    document_id: document_id.into_buffer(),
                    block_time_ms: None, //None because we want latest
                })
            })
            .collect::<Result<Vec<_>, QueryError>>());

        let proof = self.drive.prove_multiple(
            &identity_requests,
            &contract_ids,
            &document_queries,
            None,
            platform_version,
        )?;

        let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

        let response = GetProofsResponseV0 {
            result: Some(get_proofs_response_v0::Result::Proof(proof)),
            metadata: Some(metadata),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};
    use dapi_grpc::platform::v0::get_proofs_request::get_proofs_request_v0::{
        ContractRequest, DocumentRequest, IdentityRequest,
    };

    #[test]
    fn test_invalid_identity_ids() {
        let (platform, version) = setup_platform();

        let request = GetProofsRequestV0 {
            identities: vec![IdentityRequest {
                identity_id: vec![0; 8],
                request_type: 0,
            }],
            contracts: vec![],
            documents: vec![],
        };

        let result = platform
            .query_proofs_v0(request, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_identity_prove_request_type() {
        let (platform, version) = setup_platform();

        let request_type = 10;

        let request = GetProofsRequestV0 {
            identities: vec![IdentityRequest {
                identity_id: vec![0; 32],
                request_type,
            }],
            contracts: vec![],
            documents: vec![],
        };

        let result = platform
            .query_proofs_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg == &format!(
                "invalid prove request type '{}'",
                request_type
            )
        ))
    }

    #[test]
    fn test_invalid_contract_ids() {
        let (platform, version) = setup_platform();

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![ContractRequest {
                contract_id: vec![0; 8],
            }],
            documents: vec![],
        };

        let result = platform
            .query_proofs_v0(request, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_contract_id_for_documents_proof() {
        let (platform, version) = setup_platform();

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![DocumentRequest {
                contract_id: vec![0; 8],
                document_type: "niceDocument".to_string(),
                document_type_keeps_history: false,
                document_id: vec![0; 32],
            }],
        };

        let result = platform
            .query_proofs_v0(request, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_document_id() {
        let (platform, version) = setup_platform();

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![DocumentRequest {
                contract_id: vec![0; 32],
                document_type: "niceDocument".to_string(),
                document_type_keeps_history: false,
                document_id: vec![0; 8],
            }],
        };

        let result = platform
            .query_proofs_v0(request, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_proof_of_absence() {
        let (platform, version) = setup_platform();

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![DocumentRequest {
                contract_id: vec![0; 32],
                document_type: "niceDocument".to_string(),
                document_type_keeps_history: false,
                document_id: vec![0; 32],
            }],
        };

        let validation_result = platform
            .query_proofs_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(validation_result.data, Some(GetProofsResponseV0 {
            result: Some(get_proofs_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) if !proof.grovedb_proof.is_empty()));
    }
}
