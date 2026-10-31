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
use drive::error::query::QuerySyntaxError;
use drive::query::refuse_a_private_document_type;
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

        if prove && offset.is_some_and(|offset| offset > 0) {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::RequestingProofWithOffset(
                    "proof requests do not support offsets".to_string(),
                ),
            )));
        }

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
        // A private document type keeps encrypted entries at its key `[0]`, no documents and
        // no history; refused as a document query is, proved or not.
        if let Err(refusal) = refuse_a_private_document_type(document_type) {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                refusal,
            )));
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::DocumentV0Getters;
    use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
    use dpp::tests::utils::generate_random_identifier_struct;
    use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use drive::util::storage_flags::StorageFlags;

    const DOCUMENT_TYPE_NAME: &str = "profile";

    /// A private document type keeps encrypted entries at its key `[0]`, no documents and no
    /// history: a history request naming one is refused, proved or not, as a document query is.
    #[test]
    fn should_refuse_the_history_of_a_document_of_a_private_type() {
        let (platform, state, version) = setup_platform(
            None,
            Network::Testnet,
            Some(PlatformVersion::latest().protocol_version),
        );
        let contract = json_document_to_contract(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/supporting_files/contract/reference-validation/reference-validation-contract-private-foreign.json"
            ),
            false,
            version,
        )
        .expect("expected contract");
        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                version,
            )
            .expect("apply contract");

        for prove in [false, true] {
            let request = GetDocumentHistoryRequestV0 {
                data_contract_id: contract.id().to_vec(),
                document_type_name: "secret".to_string(),
                document_id: generate_random_identifier_struct().to_vec(),
                limit: Some(10),
                offset: None,
                start_at_ms: 0,
                prove,
            };
            let result = platform
                .query_document_history_v0(request, &state, version)
                .expect("query document history");
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [QueryError::Query(QuerySyntaxError::Unsupported(message))]
                        if message.contains("private document type")
                ),
                "prove {prove}: {:?}",
                result.errors
            );
        }
    }

    #[test]
    fn should_return_empty_document_history_page_without_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = json_document_to_contract(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json"
            ),
            false,
            version,
        )
        .expect("expected contract");

        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                version,
            )
            .expect("apply contract");

        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE_NAME)
            .expect("profile document type");
        let document = json_document_to_document(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../rs-drive/tests/supporting_files/contract/dashpay/profile0.json"
            ),
            Some(generate_random_identifier_struct()),
            document_type,
            version,
        )
        .expect("expected document");

        platform
            .drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::default_with_time(1000),
                true,
                None,
                version,
                None,
            )
            .expect("put document");

        let request = GetDocumentHistoryRequestV0 {
            data_contract_id: contract.id().to_vec(),
            document_type_name: DOCUMENT_TYPE_NAME.to_string(),
            document_id: document.id().to_vec(),
            limit: Some(10),
            offset: None,
            start_at_ms: 1000,
            prove: false,
        };

        let result = platform
            .query_document_history_v0(request, &state, version)
            .expect("query document history");

        assert!(
            result.errors.is_empty(),
            "expected empty history page to be successful"
        );

        let response = result.data.expect("expected data");
        let GetDocumentHistoryResponseV0 {
            result:
                Some(get_document_history_response_v0::Result::DocumentHistory(document_history)),
            metadata: Some(_),
        } = response
        else {
            panic!("expected document history response");
        };

        assert!(document_history.document_entries.is_empty());
    }
}
