use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_proofs_request::GetProofsRequestV0;
use dapi_grpc::platform::v0::get_proofs_response::GetProofsResponseV0;
use dapi_grpc::platform::v0::{get_proofs_response, GetProofsResponse, Proof};
use dpp::check_validation_result_with_data;
use dpp::platform_value::Bytes32;
use dpp::prelude::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::identity::{IdentityDriveQuery, IdentityProveRequestType};
use drive::query::SingleDocumentDriveQuery;
use prost::Message;

impl<C> Platform<C> {
    pub(super) fn query_proofs_v0(
        &self,
        state: &PlatformState,
        request: GetProofsRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.validator_set_quorum_type() as u32;
        let GetProofsRequestV0 {
            identities,
            contracts,
            documents,
        } = request;
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

        let response_data = GetProofsResponse {
            version: Some(get_proofs_response::Version::V0(GetProofsResponseV0 {
                result: Some(get_proofs_response::get_proofs_response_v0::Result::Proof(
                    Proof {
                        grovedb_proof: proof,
                        quorum_hash: state.last_committed_quorum_hash().to_vec(),
                        quorum_type,
                        block_id_hash: state.last_committed_block_id_hash().to_vec(),
                        signature: state.last_committed_block_signature().to_vec(),
                        round: state.last_committed_block_round(),
                    },
                )),
                metadata: Some(metadata),
            })),
        }
        .encode_to_vec();

        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
