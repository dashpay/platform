use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_data_contracts_by_range_request::get_data_contracts_by_range_request_v0::Start;
use dapi_grpc::platform::v0::get_data_contracts_by_range_request::GetDataContractsByRangeRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_response::{
    get_data_contracts_response_v0, DataContractEntry, DataContracts, GetDataContractsResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Returns one page of data contracts in ascending contract id order.
    ///
    /// `limit` defaults to and is capped by `max_returned_elements`. Both the default and the
    /// cap are protocol constants rather than node configuration because a client verifying
    /// the proof rebuilds the page query from the request it sent: an omitted limit has to
    /// mean the same page on every node.
    pub(super) fn query_data_contracts_by_range_v0(
        &self,
        GetDataContractsByRangeRequestV0 {
            limit,
            start,
            ids_only,
            prove,
        }: GetDataContractsByRangeRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDataContractsResponseV0>, Error> {
        let max_returned_elements = platform_version.drive_abci.query.max_returned_elements;

        let limit = match limit {
            None => max_returned_elements,
            Some(requested) => match u16::try_from(requested) {
                Ok(limit) if limit >= 1 && limit <= max_returned_elements => limit,
                _ => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        QuerySyntaxError::InvalidLimit(format!(
                            "limit {requested} is out of bounds, it must be between 1 and {max_returned_elements}"
                        )),
                    )));
                }
            },
        };

        let start_at: Option<([u8; 32], bool)> = match start {
            None => None,
            Some(Start::StartAfter(cursor)) => {
                let contract_id: [u8; 32] =
                    check_validation_result_with_data!(cursor.try_into().map_err(|_| {
                        QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(
                            "start_after should be a 32 byte contract id",
                        ))
                    }));
                Some((contract_id, false))
            }
            Some(Start::StartAt(cursor)) => {
                let contract_id: [u8; 32] =
                    check_validation_result_with_data!(cursor.try_into().map_err(|_| {
                        QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(
                            "start_at should be a 32 byte contract id",
                        ))
                    }));
                Some((contract_id, true))
            }
        };

        let response = if prove {
            let proof = self.drive.prove_contracts_by_range(
                start_at,
                limit,
                ids_only,
                None,
                platform_version,
            )?;

            GetDataContractsResponseV0 {
                result: Some(get_data_contracts_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let data_contract_entries = if ids_only {
                self.drive
                    .fetch_contract_ids(start_at, limit, None, platform_version)?
                    .into_iter()
                    .map(|contract_id| DataContractEntry {
                        identifier: contract_id.to_vec(),
                        data_contract: None,
                    })
                    .collect()
            } else {
                self.drive
                    .fetch_contracts(start_at, limit, None, platform_version)?
                    .into_iter()
                    .map(|contract| {
                        Ok(DataContractEntry {
                            identifier: contract.id().to_vec(),
                            data_contract: Some(
                                contract
                                    .serialize_to_bytes_with_platform_version(platform_version)?,
                            ),
                        })
                    })
                    .collect::<Result<Vec<DataContractEntry>, ProtocolError>>()?
            };

            GetDataContractsResponseV0 {
                result: Some(get_data_contracts_response_v0::Result::DataContracts(
                    DataContracts {
                        data_contract_entries,
                    },
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
    use crate::query::tests::{setup_platform, store_data_contract};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::DataContract;
    use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::drive::Drive;

    /// Stores contracts with ids `[1; 32]`, `[2; 32]`, `[3; 32]` and returns those ids.
    fn store_three_contracts(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> Vec<[u8; 32]> {
        (1u8..=3)
            .map(|seed| {
                let mut contract =
                    get_data_contract_fixture(None, 0, platform_version.protocol_version)
                        .data_contract_owned();
                let id = [seed; 32];
                contract.set_id(id.into());
                store_data_contract(platform, &contract, platform_version);
                id
            })
            .collect()
    }

    fn page_request(
        limit: Option<u32>,
        start: Option<Start>,
        ids_only: bool,
        prove: bool,
    ) -> GetDataContractsByRangeRequestV0 {
        GetDataContractsByRangeRequestV0 {
            limit,
            start,
            ids_only,
            prove,
        }
    }

    fn entries(
        result: QueryValidationResult<GetDataContractsResponseV0>,
    ) -> Vec<DataContractEntry> {
        assert!(
            result.errors.is_empty(),
            "unexpected errors {:?}",
            result.errors
        );
        match result.data.expect("expected data").result {
            Some(get_data_contracts_response_v0::Result::DataContracts(contracts)) => {
                contracts.data_contract_entries
            }
            other => panic!("expected data contract entries, got {other:?}"),
        }
    }

    #[test]
    fn should_reject_short_start_after() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform
            .query_data_contracts_by_range_v0(
                page_request(None, Some(Start::StartAfter(vec![0; 8])), false, false),
                &state,
                version,
            )
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(
                QuerySyntaxError::InvalidStartsWithClause(_)
            )]
        ));
    }

    #[test]
    fn should_reject_zero_limit() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform
            .query_data_contracts_by_range_v0(
                page_request(Some(0), None, false, false),
                &state,
                version,
            )
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidLimit(_))]
        ));
    }

    #[test]
    fn should_reject_limit_above_max() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let above_max = version.drive_abci.query.max_returned_elements as u32 + 1;

        let result = platform
            .query_data_contracts_by_range_v0(
                page_request(Some(above_max), None, false, false),
                &state,
                version,
            )
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidLimit(_))]
        ));
    }

    #[test]
    fn should_accept_limit_at_max() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let max = version.drive_abci.query.max_returned_elements as u32;

        let result = platform
            .query_data_contracts_by_range_v0(
                page_request(Some(max), None, false, false),
                &state,
                version,
            )
            .expect("expected query to succeed");

        assert!(entries(result).is_empty());
    }

    #[test]
    fn should_return_empty_page_on_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let full = platform
            .query_data_contracts_by_range_v0(
                page_request(None, None, false, false),
                &state,
                version,
            )
            .expect("expected query to succeed");
        assert!(entries(full).is_empty());

        let ids_only = platform
            .query_data_contracts_by_range_v0(
                page_request(None, None, true, false),
                &state,
                version,
            )
            .expect("expected query to succeed");
        assert!(entries(ids_only).is_empty());
    }

    #[test]
    fn should_page_stored_contracts_in_id_order() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let ids = store_three_contracts(&platform, version);

        let page1 = entries(
            platform
                .query_data_contracts_by_range_v0(
                    page_request(Some(2), None, false, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert_eq!(
            page1
                .iter()
                .map(|entry| entry.identifier.clone())
                .collect::<Vec<_>>(),
            vec![ids[0].to_vec(), ids[1].to_vec()]
        );
        for entry in &page1 {
            let bytes = entry
                .data_contract
                .as_ref()
                .expect("a full page carries contract bytes");
            let contract = DataContract::versioned_deserialize_trusted(bytes, false, version)
                .expect("contract should deserialize");
            assert_eq!(contract.id().to_vec(), entry.identifier);
        }

        let page2 = entries(
            platform
                .query_data_contracts_by_range_v0(
                    page_request(
                        Some(2),
                        Some(Start::StartAfter(ids[1].to_vec())),
                        false,
                        false,
                    ),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert_eq!(
            page2
                .iter()
                .map(|entry| entry.identifier.clone())
                .collect::<Vec<_>>(),
            vec![ids[2].to_vec()]
        );

        let from_second = entries(
            platform
                .query_data_contracts_by_range_v0(
                    page_request(Some(2), Some(Start::StartAt(ids[1].to_vec())), false, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert_eq!(
            from_second
                .iter()
                .map(|entry| entry.identifier.clone())
                .collect::<Vec<_>>(),
            vec![ids[1].to_vec(), ids[2].to_vec()]
        );
    }

    #[test]
    fn should_return_ids_only_entries() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let ids = store_three_contracts(&platform, version);

        let page = entries(
            platform
                .query_data_contracts_by_range_v0(
                    page_request(None, None, true, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );

        assert_eq!(
            page.iter()
                .map(|entry| entry.identifier.clone())
                .collect::<Vec<_>>(),
            ids.iter().map(|id| id.to_vec()).collect::<Vec<_>>()
        );
        assert!(page.iter().all(|entry| entry.data_contract.is_none()));
    }

    #[test]
    fn should_return_verifiable_proof_when_requested() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let ids = store_three_contracts(&platform, version);

        let result = platform
            .query_data_contracts_by_range_v0(
                page_request(
                    Some(2),
                    Some(Start::StartAfter(ids[0].to_vec())),
                    false,
                    true,
                ),
                &state,
                version,
            )
            .expect("expected query to succeed");
        assert!(
            result.errors.is_empty(),
            "unexpected errors {:?}",
            result.errors
        );

        let response = result.data.expect("expected data");
        assert!(response.metadata.is_some());
        let proof = match response.result {
            Some(get_data_contracts_response_v0::Result::Proof(proof)) => proof,
            other => panic!("expected a proof, got {other:?}"),
        };

        let (_root_hash, page) = Drive::verify_contracts_by_range(
            &proof.grovedb_proof,
            Some((ids[0], false)),
            2,
            false,
            version,
        )
        .expect("the returned proof should verify against the request");
        assert_eq!(
            page.iter()
                .map(|(id, _)| id.to_buffer())
                .collect::<Vec<_>>(),
            vec![ids[1], ids[2]]
        );
        assert!(page.iter().all(|(_, contract)| contract.is_some()));
    }
}
