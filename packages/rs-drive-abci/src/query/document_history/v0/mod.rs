use crate::error::{query::QueryError, Error};
use crate::platform_types::{platform::Platform, platform_state::PlatformState};
use crate::query::{response_metadata::CheckpointUsed, QueryValidationResult};
use dapi_grpc::platform::v0::get_document_history_request::{
    get_document_history_request_v0::Filter, GetDocumentHistoryRequestV0,
};
use dapi_grpc::platform::v0::get_document_history_response::{
    get_document_history_response_v0::{
        lifecycle::State, Entry, History, Lifecycle, Result as ResponseResult,
    },
    GetDocumentHistoryResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::query::document_history_drive_query::{
    DocumentHistoryDriveQuery, DocumentHistoryFilter, DocumentHistoryState,
};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_document_history_v0(
        &self,
        request: GetDocumentHistoryRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentHistoryResponseV0>, Error> {
        let contract_id =
            check_validation_result_with_data!(request.data_contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument("data_contract_id must be 32 bytes".to_owned())
            }));
        let document_id = check_validation_result_with_data!(request
            .document_id
            .try_into()
            .map_err(|_| QueryError::InvalidArgument("document_id must be 32 bytes".to_owned())));
        let filter = check_validation_result_with_data!(request.filter.ok_or_else(|| {
            QueryError::InvalidArgument("exactly one history filter is required".to_owned())
        }));
        let filter = match filter {
            Filter::StartAtMs(time) => DocumentHistoryFilter::StartAtTime(time),
            Filter::StartAfter(cursor) => DocumentHistoryFilter::StartAfter {
                time_ms: cursor.time_ms,
                revision: cursor.revision,
            },
            Filter::StartAtRevision(revision) => DocumentHistoryFilter::StartAtRevision(revision),
            Filter::Revision(revision) => DocumentHistoryFilter::Revision(revision),
        };
        let limit = check_validation_result_with_data!(request
            .limit
            .map(u16::try_from)
            .transpose()
            .map_err(|_| QueryError::InvalidArgument("history limit out of bounds".to_owned())));
        let query = DocumentHistoryDriveQuery {
            contract_id,
            document_type_name: request.document_type_name,
            document_id,
            filter,
            limit,
        };
        check_validation_result_with_data!(query
            .validate()
            .map_err(|error| QueryError::InvalidArgument(error.to_string())));
        let fetched = self
            .drive
            .fetch_contract(contract_id, None, None, None, platform_version)
            .value?;
        let fetched = check_validation_result_with_data!(
            fetched.ok_or_else(|| QueryError::NotFound("data contract not found".to_owned()))
        );
        let contract = &fetched.contract;
        let document_type = check_validation_result_with_data!(contract
            .document_type_for_name(&query.document_type_name)
            .map_err(|_| QueryError::NotFound("document type not found".to_owned())));
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        if !document_type.documents_keep_history() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument("document type does not keep history".to_owned()),
            ));
        }
        // A query Drive refuses (a revision filter over a gapped history, for
        // one) is the caller's mistake and answers as an invalid argument;
        // storage and corruption errors still propagate.
        let result = if request.prove {
            let proof = match self.drive.prove_document_history(
                &query,
                document_type,
                None,
                platform_version,
            ) {
                Ok(proved) => proved,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(error) => return Err(error.into()),
            };
            // The proof travels in the layout the protocol version stores,
            // signed once; from protocol version 15 that is one proof object
            // carrying both GroveDB proofs.
            let proof = self
                .response_proof_v0(platform_state, proof, GroveDBToUse::Current)?
                .1;
            ResponseResult::Proof(proof)
        } else {
            let history = match self.drive.fetch_document_history(
                &query,
                document_type,
                None,
                platform_version,
            ) {
                Ok(history) => history,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(error) => return Err(error.into()),
            };
            let entries = history
                .entries
                .into_iter()
                .map(|entry| {
                    Ok(Entry {
                        time_ms: entry.time_ms,
                        revision: entry.revision,
                        document: entry
                            .document
                            .serialize(document_type, contract, platform_version)
                            .map_err(Error::Protocol)?,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            ResponseResult::History(History {
                entries,
                lifecycle: history.lifecycle.map(|lifecycle| Lifecycle {
                    state: match lifecycle.state {
                        DocumentHistoryState::Active => State::Active,
                        DocumentHistoryState::Absent => State::Absent,
                        DocumentHistoryState::Deleted | DocumentHistoryState::Erasing => {
                            State::Absent
                        }
                    } as i32,
                    remaining_revisions: lifecycle.remaining_revisions,
                    ..Default::default()
                }),
            })
        };
        Ok(QueryValidationResult::new_with_data(
            GetDocumentHistoryResponseV0 {
                result: Some(result),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
        ))
    }
}

#[cfg(test)]
mod tests;
