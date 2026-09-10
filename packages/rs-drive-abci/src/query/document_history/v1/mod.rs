use crate::error::{query::QueryError, Error};
use crate::platform_types::{platform::Platform, platform_state::PlatformState};
use crate::query::{response_metadata::CheckpointUsed, QueryValidationResult};
use dapi_grpc::platform::v0::get_document_history_request::{
    get_document_history_request_v1::Selector, GetDocumentHistoryRequestV1,
};
use dapi_grpc::platform::v0::get_document_history_response::{
    get_document_history_response_v1::{lifecycle::State, Entry, Lifecycle},
    GetDocumentHistoryResponseV1,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::document::history::{
    DocumentHistoryQueryV1, DocumentHistorySelector, DocumentHistoryState,
};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_document_history_v1(
        &self,
        request: GetDocumentHistoryRequestV1,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentHistoryResponseV1>, Error> {
        let contract_id =
            check_validation_result_with_data!(request.data_contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument("data_contract_id must be 32 bytes".to_owned())
            }));
        let document_id = check_validation_result_with_data!(request
            .document_id
            .try_into()
            .map_err(|_| QueryError::InvalidArgument("document_id must be 32 bytes".to_owned())));
        let selector = check_validation_result_with_data!(request.selector.ok_or_else(|| {
            QueryError::InvalidArgument("exactly one history selector is required".to_owned())
        }));
        let selector = match selector {
            Selector::StartAtMs(time) => DocumentHistorySelector::StartAtTime(time),
            Selector::StartAfter(cursor) => DocumentHistorySelector::StartAfter {
                time_ms: cursor.time_ms,
                revision: cursor.revision,
            },
            Selector::StartAtRevision(revision) => {
                DocumentHistorySelector::StartAtRevision(revision)
            }
            Selector::Revision(revision) => DocumentHistorySelector::Revision(revision),
        };
        let limit = check_validation_result_with_data!(request
            .limit
            .map(u16::try_from)
            .transpose()
            .map_err(|_| QueryError::InvalidArgument("history limit out of bounds".to_owned())));
        let query = DocumentHistoryQueryV1 {
            contract_id,
            document_type_name: request.document_type_name,
            document_id,
            selector,
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
        let (history, entries_proof, metadata_proof) = if request.prove {
            let (history, proofs) = self.drive.prove_document_history_v1(
                &query,
                document_type,
                None,
                platform_version,
            )?;
            let entries_proof = proofs
                .entries_proof
                .map(|proof| {
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)
                })
                .transpose()?;
            let metadata_proof = Some(
                self.response_proof_v0(
                    platform_state,
                    proofs.metadata_proof,
                    GroveDBToUse::Current,
                )?
                .1,
            );
            (history, entries_proof, metadata_proof)
        } else {
            (
                self.drive.fetch_document_history_v1(
                    &query,
                    document_type,
                    None,
                    platform_version,
                )?,
                None,
                None,
            )
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
        Ok(QueryValidationResult::new_with_data(
            GetDocumentHistoryResponseV1 {
                entries,
                lifecycle: Some(Lifecycle {
                    state: match history.lifecycle.state {
                        DocumentHistoryState::Active => State::Active,
                        DocumentHistoryState::Absent => State::Absent,
                    } as i32,
                    remaining_revisions: history.lifecycle.remaining_revisions,
                }),
                entries_proof,
                metadata_proof,
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
        ))
    }
}

#[cfg(test)]
mod tests;
