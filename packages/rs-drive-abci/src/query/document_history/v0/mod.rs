use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_document_history_request::GetDocumentHistoryRequestV0;
use dapi_grpc::platform::v0::get_document_history_response::get_document_history_response_v0::DocumentHistoryEntry;
use dapi_grpc::platform::v0::get_document_history_response::{
    get_document_history_response_v0, GetDocumentHistoryResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::document::MAX_DOCUMENT_HISTORY_FETCH_LIMIT;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_document_history_v0(
        &self,
        GetDocumentHistoryRequestV0 {
            data_contract_id,
            document_type_name,
            document_id,
            limit,
            offset,
            start_at_ms,
            prove,
        }: GetDocumentHistoryRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentHistoryResponseV0>, Error> {
        let contract_id: Identifier =
            check_validation_result_with_data!(data_contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "data_contract_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));
        let document_id: Identifier =
            check_validation_result_with_data!(document_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "document_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let limit = check_validation_result_with_data!(limit
            .map(|limit| {
                let limit = u16::try_from(limit)
                    .map_err(|_| QueryError::InvalidArgument("limit out of bounds".to_string()))?;

                if !(1..=MAX_DOCUMENT_HISTORY_FETCH_LIMIT).contains(&limit) {
                    return Err(QueryError::InvalidArgument(format!(
                        "limit {} out of bounds of [1, {}]",
                        limit, MAX_DOCUMENT_HISTORY_FETCH_LIMIT,
                    )));
                }

                Ok(limit)
            })
            .transpose());

        let offset = check_validation_result_with_data!(offset
            .map(|offset| {
                u16::try_from(offset)
                    .map_err(|_| QueryError::InvalidArgument("offset out of bounds".to_string()))
            })
            .transpose());

        let maybe_contract_fetch_info = self
            .drive
            .fetch_contract(contract_id.to_buffer(), None, None, None, platform_version)
            .unwrap()?;
        let contract_fetch_info = check_validation_result_with_data!(maybe_contract_fetch_info
            .ok_or_else(|| {
                QueryError::NotFound(format!("data contract {} not found", contract_id))
            }));
        let contract = &contract_fetch_info.contract;
        let document_type = check_validation_result_with_data!(contract
            .document_type_for_name(&document_type_name)
            .map_err(|_| QueryError::NotFound(format!(
                "document type {} not found in data contract {}",
                document_type_name, contract_id
            ))));

        let response = if prove {
            let proof = self.drive.prove_document_history(
                contract_id.to_buffer(),
                &document_type_name,
                document_id.to_buffer(),
                None,
                start_at_ms,
                limit,
                offset,
                platform_version,
            )?;

            GetDocumentHistoryResponseV0 {
                result: Some(get_document_history_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let documents = self.drive.fetch_document_history(
                contract_id.to_buffer(),
                &document_type_name,
                document_type,
                document_id.to_buffer(),
                None,
                start_at_ms,
                limit,
                offset,
                platform_version,
            )?;

            if documents.is_empty() {
                return Ok(QueryValidationResult::new_with_error(QueryError::NotFound(
                    format!("document {} history not found", document_id),
                )));
            }

            let document_entries = documents
                .into_iter()
                .map(|(date, document)| {
                    Ok(DocumentHistoryEntry {
                        date,
                        value: document
                            .serialize(document_type, contract, platform_version)
                            .map_err(Error::Protocol)?,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;

            GetDocumentHistoryResponseV0 {
                result: Some(get_document_history_response_v0::Result::DocumentHistory(
                    get_document_history_response_v0::DocumentHistory { document_entries },
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
