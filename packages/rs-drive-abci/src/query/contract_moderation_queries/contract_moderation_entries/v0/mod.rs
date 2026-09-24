use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_moderation_queries::{
    identifier_from_request, list_from_request, reason_to_response, warnings_to_response,
};
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_moderation_entries_request::GetContractModerationEntriesRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_entries_response::{
    get_contract_moderation_entries_response_v0, ContractModerationEntries,
    ContractModerationEntry as ContractModerationEntryProto,
    GetContractModerationEntriesResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract::moderation::types::ContractModerationEntriesQuery;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Returns one page of a moderated contract's banlist, suspension list or warning list, in
    /// identity id order. The list must be one the contract keeps.
    pub(super) fn query_contract_moderation_entries_v0(
        &self,
        GetContractModerationEntriesRequestV0 {
            contract_id,
            list,
            start_after,
            limit,
            prove,
        }: GetContractModerationEntriesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractModerationEntriesResponseV0>, Error> {
        let contract_id =
            check_validation_result_with_data!(identifier_from_request(contract_id, "contract_id"));
        let list = check_validation_result_with_data!(list_from_request(list, "list"));
        let start_after = check_validation_result_with_data!(start_after
            .map(|bytes| identifier_from_request(bytes, "start_after"))
            .transpose());
        let limit = match limit {
            // The page size when the request names none: the largest page, the number the
            // proof verifier assumes as well.
            None => platform_version.drive_abci.query.max_returned_elements,
            // Refused here, as an invalid argument: Drive refuses it too, but as an error of its
            // own that would reach the client as an unknown node failure.
            Some(limit) => check_validation_result_with_data!(u16::try_from(limit)
                .ok()
                .filter(|limit| {
                    (1..=platform_version.drive_abci.query.max_returned_elements).contains(limit)
                })
                .ok_or_else(|| {
                    QueryError::InvalidArgument(format!(
                        "limit {limit} is out of bounds, it must be between 1 and {}",
                        platform_version.drive_abci.query.max_returned_elements
                    ))
                })),
        };

        let kept = check_validation_result_with_data!(
            self.kept_moderation_lists(contract_id, platform_version)?
        );
        if !kept.contains(&list) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "contract {} does not keep a {}",
                    contract_id, list
                )),
            ));
        }

        let query = ContractModerationEntriesQuery {
            list,
            start_after,
            limit,
        };

        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_contract_moderation_entries(contract_id, &query, None, platform_version));

            GetContractModerationEntriesResponseV0 {
                result: Some(get_contract_moderation_entries_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let entries = check_validation_result_with_data!(self
                .drive
                .fetch_contract_moderation_entries(contract_id, &query, None, platform_version));

            GetContractModerationEntriesResponseV0 {
                result: Some(
                    get_contract_moderation_entries_response_v0::Result::Entries(
                        ContractModerationEntries {
                            entries: entries
                                .into_iter()
                                .map(|entry| ContractModerationEntryProto {
                                    identity_id: entry.identity_id.to_vec(),
                                    until: entry.until,
                                    // A warning entry's reason is its latest warning's, which
                                    // travels with the warnings: sent once.
                                    reason: entry
                                        .warnings
                                        .is_empty()
                                        .then(|| reason_to_response(entry.reason)),
                                    warnings: warnings_to_response(entry.warnings),
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
    use crate::query::contract_moderation_queries::tests::{
        ban, store_contract, store_contract_keeping, suspend, warn, BANLIST, BAN_REASON,
        SUSPENSIONS, SUSPENSION_REASON, SUSPENSION_REASON_CODE, WARNINGS, WARNING_REASON,
    };
    use crate::query::tests::setup_platform;
    use dapi_grpc::platform::v0::ContractModerationReason as ContractModerationReasonProto;
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::config::moderation::{
        ContractModerationList, ContractModerationReason,
    };
    use dpp::identifier::Identifier;
    use drive::drive::contract::moderation::types::ContractModerationEntry;
    use drive::drive::Drive;

    fn request(
        contract_id: Vec<u8>,
        list: i32,
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
        prove: bool,
    ) -> GetContractModerationEntriesRequestV0 {
        GetContractModerationEntriesRequestV0 {
            contract_id,
            list,
            start_after,
            limit,
            prove,
        }
    }

    fn assert_invalid_argument(
        result: QueryValidationResult<GetContractModerationEntriesResponseV0>,
        needle: &str,
    ) {
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::InvalidArgument(msg)] if msg.contains(needle)
            ),
            "expected an invalid argument naming {needle}, got {:?}",
            result.errors
        );
    }

    #[test]
    fn should_refuse_malformed_requests() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let id = vec![1; 32];
        let query = |request| {
            platform
                .query_contract_moderation_entries_v0(request, &state, version)
                .expect("expected query to succeed")
        };

        assert_invalid_argument(
            query(request(vec![0; 8], BANLIST, None, None, false)),
            "contract_id",
        );
        assert_invalid_argument(
            query(request(id.clone(), 7, None, None, false)),
            "not a moderation list",
        );
        assert_invalid_argument(
            query(request(id.clone(), BANLIST, Some(vec![0; 8]), None, false)),
            "start_after",
        );
        for limit in [0, 101, 70_000] {
            assert_invalid_argument(
                query(request(id.clone(), BANLIST, None, Some(limit), false)),
                "out of bounds",
            );
        }
    }

    #[test]
    fn should_refuse_an_unknown_contract_and_an_unmoderated_one() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform
            .query_contract_moderation_entries_v0(
                request(vec![9; 32], BANLIST, None, None, false),
                &state,
                version,
            )
            .expect("expected query to succeed");
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::NotFound(_)]
        ));

        let unmoderated = store_contract(&platform, false, false, version);
        assert_invalid_argument(
            platform
                .query_contract_moderation_entries_v0(
                    request(unmoderated.id().to_vec(), BANLIST, None, None, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
            "is not moderated",
        );
    }

    #[test]
    fn should_refuse_a_list_the_contract_does_not_keep() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = store_contract(&platform, false, true, version);
        assert_invalid_argument(
            platform
                .query_contract_moderation_entries_v0(
                    request(contract.id().to_vec(), BANLIST, None, None, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
            "does not keep",
        );
    }

    #[test]
    fn should_return_and_prove_the_warning_list_with_every_warning() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = store_contract_keeping(&platform, false, false, true, version);
        let once = Identifier::from([0x21; 32]);
        let twice = Identifier::from([0x22; 32]);
        warn(&platform, &contract, once, 1_000, version);
        warn(&platform, &contract, twice, 1_000, version);
        warn(&platform, &contract, twice, 2_000, version);

        let result = platform
            .query_contract_moderation_entries_v0(
                request(contract.id().to_vec(), WARNINGS, None, None, false),
                &state,
                version,
            )
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let Some(get_contract_moderation_entries_response_v0::Result::Entries(page)) =
            result.data.expect("expected data").result
        else {
            panic!("expected entries");
        };
        let reason = || {
            Some(ContractModerationReasonProto {
                code: None,
                text: WARNING_REASON.to_string(),
                documents: vec![],
                reason_document_id: None,
            })
        };
        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.entries[0].identity_id, once.to_vec());
        assert_eq!(page.entries[0].until, None);
        // The entry's reason is the latest warning's, which travels with the warnings alone.
        assert_eq!(page.entries[0].reason, None);
        assert_eq!(page.entries[0].warnings.len(), 1);
        assert_eq!(page.entries[0].warnings[0].reason, reason());
        assert_eq!(
            page.entries[1]
                .warnings
                .iter()
                .map(|warning| warning.warned_at)
                .collect::<Vec<_>>(),
            vec![1_000, 2_000]
        );
        assert_eq!(page.entries[1].warnings[1].reason, reason());

        let result = platform
            .query_contract_moderation_entries_v0(
                request(contract.id().to_vec(), WARNINGS, None, None, true),
                &state,
                version,
            )
            .expect("expected query to succeed");
        let Some(get_contract_moderation_entries_response_v0::Result::Proof(proof)) =
            result.data.expect("expected data").result
        else {
            panic!("expected a proof");
        };
        let query = ContractModerationEntriesQuery {
            list: ContractModerationList::Warnings,
            start_after: None,
            limit: version.drive_abci.query.max_returned_elements,
        };
        let (_, proved) = Drive::verify_contract_moderation_entries(
            &proof.grovedb_proof,
            contract.id(),
            &query,
            version,
        )
        .expect("expected the proof to verify");
        assert_eq!(
            proved
                .iter()
                .map(|entry: &ContractModerationEntry| (entry.identity_id, entry.warnings.len()))
                .collect::<Vec<_>>(),
            vec![(once, 1), (twice, 2)]
        );
    }

    #[test]
    fn should_page_and_prove_the_entries() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = store_contract(&platform, true, true, version);
        let first = Identifier::from([0x21; 32]);
        let second = Identifier::from([0x22; 32]);
        let third = Identifier::from([0x23; 32]);
        for target in [third, first, second] {
            ban(&platform, &contract, target, version);
        }
        suspend(&platform, &contract, second, 1_234, version);

        let entries = |list, start_after: Option<Identifier>, limit| {
            let result = platform
                .query_contract_moderation_entries_v0(
                    request(
                        contract.id().to_vec(),
                        list,
                        start_after.map(|id| id.to_vec()),
                        limit,
                        false,
                    ),
                    &state,
                    version,
                )
                .expect("expected query to succeed");
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            let data = result.data.expect("expected data");
            assert!(data.metadata.is_some());
            let Some(get_contract_moderation_entries_response_v0::Result::Entries(page)) =
                data.result
            else {
                panic!("expected entries");
            };
            page.entries
                .into_iter()
                .map(|entry| (entry.identity_id, entry.until, entry.reason))
                .collect::<Vec<_>>()
        };
        let ban_reason = || {
            Some(ContractModerationReasonProto {
                code: None,
                text: BAN_REASON.to_string(),
                documents: vec![],
                reason_document_id: None,
            })
        };

        // No limit is the default page, in identity id order.
        assert_eq!(
            entries(BANLIST, None, None),
            vec![
                (first.to_vec(), None, ban_reason()),
                (second.to_vec(), None, ban_reason()),
                (third.to_vec(), None, ban_reason())
            ]
        );
        assert_eq!(
            entries(BANLIST, None, Some(2)),
            vec![
                (first.to_vec(), None, ban_reason()),
                (second.to_vec(), None, ban_reason())
            ]
        );
        assert_eq!(
            entries(BANLIST, Some(second), Some(2)),
            vec![(third.to_vec(), None, ban_reason())]
        );
        assert_eq!(
            entries(SUSPENSIONS, None, None),
            vec![(
                second.to_vec(),
                Some(1_234),
                Some(ContractModerationReasonProto {
                    code: Some(SUSPENSION_REASON_CODE as u32),
                    text: SUSPENSION_REASON.to_string(),
                    documents: vec![],
                    reason_document_id: None,
                })
            )]
        );

        let result = platform
            .query_contract_moderation_entries_v0(
                request(
                    contract.id().to_vec(),
                    BANLIST,
                    Some(first.to_vec()),
                    Some(2),
                    true,
                ),
                &state,
                version,
            )
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let Some(get_contract_moderation_entries_response_v0::Result::Proof(proof)) =
            result.data.expect("expected data").result
        else {
            panic!("expected a proof");
        };
        let (_, proved) = Drive::verify_contract_moderation_entries(
            &proof.grovedb_proof,
            contract.id(),
            &ContractModerationEntriesQuery {
                list: ContractModerationList::Banlist,
                start_after: Some(first),
                limit: 2,
            },
            version,
        )
        .expect("expected the proof to verify");
        assert_eq!(
            proved,
            vec![
                ContractModerationEntry {
                    identity_id: second,
                    until: None,
                    reason: ContractModerationReason::from_text(BAN_REASON),
                    warnings: vec![],
                },
                ContractModerationEntry {
                    identity_id: third,
                    until: None,
                    reason: ContractModerationReason::from_text(BAN_REASON),
                    warnings: vec![],
                }
            ]
        );
    }
}
