use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_moderation_queries::{identifier_from_request, reason_to_response};
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_document_removals_request::get_contract_document_removals_request_v0::Selection;
use dapi_grpc::platform::v0::get_contract_document_removals_request::{
    DocumentIds, GetContractDocumentRemovalsRequestV0, Page,
};
use dapi_grpc::platform::v0::get_contract_document_removals_response::{
    get_contract_document_removals_response_v0,
    ContractDocumentRemoval as ContractDocumentRemovalProto, ContractDocumentRemovals,
    ContractDocumentRestoration as ContractDocumentRestorationProto,
    GetContractDocumentRemovalsResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract::moderation::types::{
    ContractDocumentRemovalsQuery, ContractDocumentRemovalsSelection,
};
use drive::drive::Drive;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Returns the records of the documents a contract's moderators deleted, within one
    /// document type: the ones of the document ids named, or one page in document id order.
    /// The document type must be one whose documents moderators may delete: no other keeps
    /// records, and a proof over a tree that does not exist could not be built.
    pub(super) fn query_contract_document_removals_v0(
        &self,
        GetContractDocumentRemovalsRequestV0 {
            contract_id,
            document_type_name,
            selection,
            prove,
        }: GetContractDocumentRemovalsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractDocumentRemovalsResponseV0>, Error> {
        let contract_id =
            check_validation_result_with_data!(identifier_from_request(contract_id, "contract_id"));
        let selection = match selection {
            None => {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "either document_ids or page must be set".to_string(),
                    ),
                ))
            }
            Some(Selection::DocumentIds(DocumentIds { document_ids })) => {
                ContractDocumentRemovalsSelection::DocumentIds(check_validation_result_with_data!(
                    document_ids
                        .into_iter()
                        .map(|bytes| identifier_from_request(bytes, "document_ids"))
                        .collect::<Result<Vec<Identifier>, _>>()
                ))
            }
            Some(Selection::Page(Page { start_after, limit })) => {
                ContractDocumentRemovalsSelection::Page {
                    start_after: check_validation_result_with_data!(start_after
                        .map(|bytes| identifier_from_request(bytes, "start_after"))
                        .transpose()),
                    // The page size when the request names none: the largest page, the number
                    // the proof verifier assumes as well. A limit no u16 holds is past every
                    // bound, and is refused below as the largest u16 is.
                    limit: limit.map_or(
                        platform_version.drive_abci.query.max_returned_elements,
                        |limit| u16::try_from(limit).unwrap_or(u16::MAX),
                    ),
                }
            }
        };
        let query = ContractDocumentRemovalsQuery {
            document_type_name,
            selection,
        };
        // The bounds of a read are Drive's, which the proof verifier calls too. Refused here as
        // an invalid argument: left to the fetch or the proof below, the same refusal would
        // reach the client as an unknown node failure.
        if let Err(error) = Drive::check_contract_document_removals_query(&query, platform_version)
        {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(error.to_string()),
            ));
        }

        let Some(contract_fetch_info) = self.drive.get_contract_with_fetch_info(
            contract_id.to_buffer(),
            false,
            None,
            platform_version,
        )?
        else {
            return Ok(QueryValidationResult::new_with_error(QueryError::NotFound(
                format!("contract {} not found", contract_id),
            )));
        };
        let deletable_by_moderators = contract_fetch_info
            .contract
            .document_type_optional_for_name(&query.document_type_name)
            .is_some_and(|document_type| document_type.documents_can_be_deleted_by_moderators());
        if !deletable_by_moderators {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "contract {} has no document type {} whose documents moderators can delete",
                    contract_id, query.document_type_name
                )),
            ));
        }

        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_contract_document_removals(contract_id, &query, None, platform_version));

            GetContractDocumentRemovalsResponseV0 {
                result: Some(get_contract_document_removals_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let entries = check_validation_result_with_data!(self
                .drive
                .fetch_contract_document_removals(contract_id, &query, None, platform_version));

            GetContractDocumentRemovalsResponseV0 {
                result: Some(
                    get_contract_document_removals_response_v0::Result::Removals(
                        ContractDocumentRemovals {
                            removals: entries
                                .into_iter()
                                .map(|entry| ContractDocumentRemovalProto {
                                    document_id: entry.document_id.to_vec(),
                                    document_owner_id: entry.removal.document_owner_id.to_vec(),
                                    moderator_id: entry.removal.moderator_id.to_vec(),
                                    removed_at: entry.removal.removed_at,
                                    reason: Some(reason_to_response(entry.removal.reason)),
                                    document_hash: entry.removal.document_hash.to_vec(),
                                    restoration: entry.removal.restoration.map(|restoration| {
                                        ContractDocumentRestorationProto {
                                            moderator_id: restoration.moderator_id.to_vec(),
                                            restored_at: restoration.restored_at,
                                        }
                                    }),
                                })
                                .collect(),
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{setup_platform, store_data_contract};
    use dapi_grpc::platform::v0::ContractModerationReason as ContractModerationReasonProto;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::config::moderation::{
        ContractDocumentRemoval, ContractDocumentRestoration, ContractModerationConfig,
        ContractModerationReason, ContractModerators,
    };
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::platform_value;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::drive::Drive;
    use drive::util::batch::{ContractModerationOperationType, DriveOperation};

    const POST: &str = "post";

    fn contract_with_posts() -> DataContract {
        let platform_version = PlatformVersion::latest();
        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_config(contract.config().clone().with_moderation(Some(
            ContractModerationConfig {
                banlist: false,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            },
        )));
        contract
            .set_document_schema(
                POST,
                platform_value!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "maxLength": 50, "position": 0 },
                    },
                    "additionalProperties": false,
                    "canBeDeletedByModerators": true,
                }),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("expected to add the post type");
        contract
    }

    fn removal(seed: u8) -> ContractDocumentRemoval {
        ContractDocumentRemoval {
            document_owner_id: Identifier::from([seed + 0x10; 32]),
            moderator_id: Identifier::from([0x77; 32]),
            reason: ContractModerationReason {
                code: Some(seed as u16),
                text: "spam".to_string(),
                documents: vec![],
                reason_document_id: None,
            },
            removed_at: 1_000 + seed as u64,
            document_hash: [seed + 0x20; 32],
            // Every other record was restored: the response carries the restoration too.
            restoration: seed.is_multiple_of(2).then(|| ContractDocumentRestoration {
                moderator_id: Identifier::from([0x78; 32]),
                restored_at: 2_000 + seed as u64,
            }),
        }
    }

    fn record(drive: &Drive, contract: &DataContract, seed: u8) {
        drive
            .apply_drive_operations(
                vec![DriveOperation::ContractModerationOperation(
                    ContractModerationOperationType::AddDocumentRemoval {
                        contract_id: contract.id(),
                        document_type_name: POST.to_string(),
                        document_id: Identifier::from([seed; 32]),
                        removal: removal(seed),
                        replaces_existing: false,
                        moderator_id: Identifier::from([0x77; 32]),
                    },
                )],
                true,
                &BlockInfo::default(),
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("expected to record a removal");
    }

    fn request(
        contract: &DataContract,
        document_type_name: &str,
        selection: Option<Selection>,
        prove: bool,
    ) -> GetContractDocumentRemovalsRequestV0 {
        GetContractDocumentRemovalsRequestV0 {
            contract_id: contract.id().to_vec(),
            document_type_name: document_type_name.to_string(),
            selection,
            prove,
        }
    }

    fn page(start_after: Option<u8>, limit: Option<u32>) -> Option<Selection> {
        Some(Selection::Page(Page {
            start_after: start_after.map(|seed| vec![seed; 32]),
            limit,
        }))
    }

    fn ids(seeds: &[u8]) -> Option<Selection> {
        Some(Selection::DocumentIds(DocumentIds {
            document_ids: seeds.iter().map(|seed| vec![*seed; 32]).collect(),
        }))
    }

    fn proto(seed: u8) -> ContractDocumentRemovalProto {
        let removal = removal(seed);
        ContractDocumentRemovalProto {
            document_id: vec![seed; 32],
            document_owner_id: removal.document_owner_id.to_vec(),
            moderator_id: removal.moderator_id.to_vec(),
            removed_at: removal.removed_at,
            reason: Some(ContractModerationReasonProto {
                code: Some(seed as u32),
                text: "spam".to_string(),
                documents: vec![],
                reason_document_id: None,
            }),
            document_hash: removal.document_hash.to_vec(),
            restoration: removal
                .restoration
                .map(|restoration| ContractDocumentRestorationProto {
                    moderator_id: restoration.moderator_id.to_vec(),
                    restored_at: restoration.restored_at,
                }),
        }
    }

    #[test]
    fn should_return_the_records_by_ids_and_by_page() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = contract_with_posts();
        store_data_contract(&platform, &contract, version);
        for seed in [1, 2, 3] {
            record(&platform.drive, &contract, seed);
        }

        let removals = |selection| {
            let result = platform
                .query_contract_document_removals_v0(
                    request(&contract, POST, selection, false),
                    &state,
                    version,
                )
                .expect("expected the query to run");
            assert!(result.is_valid(), "{:?}", result.errors);
            match result.into_data().expect("expected data").result {
                Some(get_contract_document_removals_response_v0::Result::Removals(removals)) => {
                    removals.removals
                }
                other => panic!("expected removals, got {other:?}"),
            }
        };

        // An id with no record is left out.
        assert_eq!(removals(ids(&[3, 9, 1])), vec![proto(1), proto(3)]);
        assert_eq!(removals(page(None, Some(2))), vec![proto(1), proto(2)]);
        assert_eq!(removals(page(Some(2), None)), vec![proto(3)]);
        assert_eq!(removals(page(Some(3), None)), vec![]);
    }

    #[test]
    fn should_prove_the_records_the_verifier_reads_back() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = contract_with_posts();
        store_data_contract(&platform, &contract, version);
        record(&platform.drive, &contract, 1);

        for (selection, query) in [
            (
                ids(&[1, 9]),
                ContractDocumentRemovalsQuery {
                    document_type_name: POST.to_string(),
                    selection: ContractDocumentRemovalsSelection::DocumentIds(vec![
                        Identifier::from([1; 32]),
                        Identifier::from([9; 32]),
                    ]),
                },
            ),
            (
                // No limit named: the verifier assumes the largest page, as the node does.
                page(None, None),
                ContractDocumentRemovalsQuery {
                    document_type_name: POST.to_string(),
                    selection: ContractDocumentRemovalsSelection::Page {
                        start_after: None,
                        limit: version.drive_abci.query.max_returned_elements,
                    },
                },
            ),
        ] {
            let result = platform
                .query_contract_document_removals_v0(
                    request(&contract, POST, selection, true),
                    &state,
                    version,
                )
                .expect("expected the query to run");
            assert!(result.is_valid(), "{:?}", result.errors);
            let Some(get_contract_document_removals_response_v0::Result::Proof(proof)) =
                result.into_data().expect("expected data").result
            else {
                panic!("expected a proof");
            };
            let (_, entries) = Drive::verify_contract_document_removals(
                &proof.grovedb_proof,
                contract.id(),
                &query,
                false,
                version,
            )
            .expect("expected the proof to verify");
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].document_id, Identifier::from([1; 32]));
            assert_eq!(entries[0].removal, removal(1));
        }
    }

    #[test]
    fn should_refuse_a_request_it_can_not_answer() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = contract_with_posts();
        store_data_contract(&platform, &contract, version);
        let max = version.drive_abci.query.max_returned_elements as u32;

        let errors = |request| {
            platform
                .query_contract_document_removals_v0(request, &state, version)
                .expect("expected the query to run")
                .errors
        };
        let invalid = |request| {
            let errors = errors(request);
            assert!(
                matches!(errors.as_slice(), [QueryError::InvalidArgument(_)]),
                "{errors:?}"
            );
        };

        invalid(request(&contract, POST, None, false));
        invalid(request(&contract, POST, ids(&[]), false));
        invalid(request(&contract, POST, ids(&[1, 1]), false));
        invalid(request(&contract, POST, page(None, Some(0)), true));
        invalid(request(&contract, POST, page(None, Some(max + 1)), true));
        invalid(request(
            &contract,
            POST,
            Some(Selection::DocumentIds(DocumentIds {
                document_ids: vec![vec![1; 31]],
            })),
            false,
        ));
        // A document type that keeps no records, and one the contract does not have: a proof
        // over either would run into a tree that does not exist.
        invalid(request(&contract, "niceDocument", page(None, None), true));
        invalid(request(&contract, "comment", page(None, None), true));

        let mut unknown = request(&contract, POST, page(None, None), false);
        unknown.contract_id = vec![0x5a; 32];
        assert!(matches!(
            errors(unknown).as_slice(),
            [QueryError::NotFound(_)]
        ));
    }
}
