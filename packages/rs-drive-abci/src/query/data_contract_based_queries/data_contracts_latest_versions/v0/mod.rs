use crate::error::execution::ExecutionError;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_data_contracts_latest_versions_request::GetDataContractsLatestVersionsRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_latest_versions_response::{
    get_data_contracts_latest_versions_response_v0, DataContractLatestVersionEntry,
    DataContractsLatestVersions, GetDataContractsLatestVersionsResponseV0,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Bytes32;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;
use std::collections::BTreeSet;

impl<C> Platform<C> {
    /// Returns the current version number of each requested data contract, and the serialized
    /// contracts only when `include_contracts` is set. Every requested id gets an entry; an id
    /// no contract has gets an entry without a version.
    ///
    /// From protocol version 14 every contract carries a four-byte version item beside it
    /// (`latest_versions_read` helper version 1). Without `include_contracts`, the unproved
    /// form answers from Drive's contract cache when it holds the contract and from the
    /// version item otherwise, never loading a contract; the proved form proves the version
    /// items, a few hundred bytes of hash path per contract instead of the contract.
    ///
    /// With `include_contracts`, and on state without version items (helper version 0), the
    /// unproved form reads the contracts through the cache, loading a miss into it as the
    /// document queries do, and the proved form is the multi-contract proof `getDataContracts`
    /// returns, which carries the contracts.
    pub(super) fn query_data_contracts_latest_versions_v0(
        &self,
        GetDataContractsLatestVersionsRequestV0 {
            ids,
            include_contracts,
            prove,
        }: GetDataContractsLatestVersionsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDataContractsLatestVersionsResponseV0>, Error> {
        let max_returned_elements = platform_version.drive_abci.query.max_returned_elements;
        if ids.len() > max_returned_elements as usize {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!(
                    "trying to get {} data contract versions, maximum is {}",
                    ids.len(),
                    max_returned_elements
                )),
            )));
        }
        if ids.is_empty() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument("ids must contain at least one identifier".to_string()),
            ));
        }

        let contract_ids = check_validation_result_with_data!(ids
            .into_iter()
            .map(|contract_id_vec| {
                Bytes32::from_vec(contract_id_vec)
                    .map(|bytes| bytes.0)
                    .map_err(|_| {
                        QueryError::InvalidArgument(
                            "id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })
            })
            .collect::<Result<Vec<[u8; 32]>, QueryError>>());

        let from_version_items = match platform_version
            .drive_abci
            .query
            .data_contract_query_helpers
            .latest_versions_read
        {
            0 => false,
            1 => !include_contracts,
            version => {
                return Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                    method: "query_data_contracts_latest_versions_v0::latest_versions_read"
                        .to_string(),
                    known_versions: vec![0, 1],
                    received: version,
                }))
            }
        };

        let response = if prove {
            let proof = if from_version_items {
                self.drive.prove_contracts_versions(
                    contract_ids.as_slice(),
                    None,
                    platform_version,
                )?
            } else {
                self.drive
                    .prove_contracts(contract_ids.as_slice(), None, platform_version)?
            };

            GetDataContractsLatestVersionsResponseV0 {
                result: Some(
                    get_data_contracts_latest_versions_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                            .map(|(_, proof)| proof)?,
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let entries = if from_version_items {
                self.data_contracts_latest_versions_from_version_items(
                    contract_ids,
                    platform_version,
                )?
            } else {
                self.data_contracts_latest_versions_from_contracts(
                    contract_ids,
                    include_contracts,
                    platform_version,
                )?
            };

            GetDataContractsLatestVersionsResponseV0 {
                result: Some(
                    get_data_contracts_latest_versions_response_v0::Result::DataContractsLatestVersions(
                        DataContractsLatestVersions { entries },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }

    /// One entry per distinct id, in id order: the version from the contract cache when it
    /// holds the contract, else from the version item beside the contract. A contract is
    /// never loaded from state for this.
    fn data_contracts_latest_versions_from_version_items(
        &self,
        contract_ids: Vec<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DataContractLatestVersionEntry>, Error> {
        contract_ids
            .into_iter()
            .collect::<BTreeSet<[u8; 32]>>()
            .into_iter()
            .map(|contract_id| {
                let version = match self.drive.get_cached_contract_with_fetch_info(
                    contract_id,
                    None,
                    &platform_version.drive,
                )? {
                    Some(contract_fetch_info) => Some(contract_fetch_info.contract.version()),
                    None => {
                        self.drive
                            .fetch_contract_version(contract_id, None, platform_version)?
                    }
                };
                Ok(DataContractLatestVersionEntry {
                    identifier: contract_id.to_vec(),
                    version,
                    data_contract: None,
                })
            })
            .collect()
    }

    /// One entry per distinct id, in id order, read from the contracts through the cache: a
    /// miss loads the contract into the cache, as the document queries do.
    fn data_contracts_latest_versions_from_contracts(
        &self,
        contract_ids: Vec<[u8; 32]>,
        include_contracts: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DataContractLatestVersionEntry>, Error> {
        let contracts = self.drive.get_contracts_with_fetch_info(
            contract_ids.as_slice(),
            true,
            None,
            platform_version,
        )?;

        contracts
            .into_iter()
            .map(|(contract_id, maybe_contract_fetch_info)| {
                let (version, data_contract) = match maybe_contract_fetch_info {
                    None => (None, None),
                    Some(contract_fetch_info) => {
                        let contract = &contract_fetch_info.contract;
                        let data_contract = include_contracts
                            .then(|| {
                                contract.serialize_to_bytes_with_platform_version(platform_version)
                            })
                            .transpose()?;
                        (Some(contract.version()), data_contract)
                    }
                };
                Ok(DataContractLatestVersionEntry {
                    identifier: contract_id.to_vec(),
                    version,
                    data_contract,
                })
            })
            .collect::<Result<Vec<DataContractLatestVersionEntry>, ProtocolError>>()
            .map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform, store_data_contract};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::DataContract;
    use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::drive::Drive;

    fn request(
        ids: Vec<Vec<u8>>,
        include_contracts: bool,
        prove: bool,
    ) -> GetDataContractsLatestVersionsRequestV0 {
        GetDataContractsLatestVersionsRequestV0 {
            ids,
            include_contracts,
            prove,
        }
    }

    /// Stores the fixture contract under `id` and returns it.
    fn store_contract_with_id(
        platform: &TempPlatform<MockCoreRPCLike>,
        id: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let mut contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_id(id.into());
        store_data_contract(platform, &contract, platform_version);
        contract
    }

    /// The entries of a successful unproved response.
    fn entries(
        result: QueryValidationResult<GetDataContractsLatestVersionsResponseV0>,
    ) -> Vec<DataContractLatestVersionEntry> {
        assert!(
            result.errors.is_empty(),
            "unexpected errors {:?}",
            result.errors
        );
        match result.data.expect("expected data").result {
            Some(
                get_data_contracts_latest_versions_response_v0::Result::DataContractsLatestVersions(
                    versions,
                ),
            ) => versions.entries,
            other => panic!("expected data contract versions, got {other:?}"),
        }
    }

    /// The proof of a successful proved response.
    fn proof(
        result: QueryValidationResult<GetDataContractsLatestVersionsResponseV0>,
    ) -> dapi_grpc::platform::v0::Proof {
        assert!(
            result.errors.is_empty(),
            "unexpected errors {:?}",
            result.errors
        );
        match result.data.expect("expected data").result {
            Some(get_data_contracts_latest_versions_response_v0::Result::Proof(proof)) => proof,
            other => panic!("expected a proof, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_data_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform.query_data_contracts_latest_versions_v0(
            request(vec![vec![0; 8]], false, false),
            &state,
            version,
        );

        assert_invalid_identifier(result.unwrap());
    }

    #[test]
    fn test_invalid_data_contract_id_for_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform.query_data_contracts_latest_versions_v0(
            request(vec![vec![1; 32], vec![0; 7]], false, true),
            &state,
            version,
        );

        assert_invalid_identifier(result.unwrap());
    }

    #[test]
    fn test_rejects_empty_ids() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform
            .query_data_contracts_latest_versions_v0(request(vec![], false, false), &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("at least one")
        ));
    }

    #[test]
    fn test_rejects_too_many_ids() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let too_many = version.drive_abci.query.max_returned_elements as usize + 1;

        let result = platform
            .query_data_contracts_latest_versions_v0(
                request(vec![vec![1; 32]; too_many], false, false),
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
    fn test_missing_contract_gets_entry_without_version() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let entries = entries(
            platform
                .query_data_contracts_latest_versions_v0(
                    request(vec![vec![0; 32]], true, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].identifier, vec![0; 32]);
        assert_eq!(entries[0].version, None);
        assert_eq!(entries[0].data_contract, None);
    }

    #[test]
    fn test_absence_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform
            .query_data_contracts_latest_versions_v0(
                request(vec![vec![0; 32]], false, true),
                &state,
                version,
            )
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetDataContractsLatestVersionsResponseV0 {
                result: Some(get_data_contracts_latest_versions_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_returns_versions_without_contracts() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let first = store_contract_with_id(&platform, [1; 32], version);
        let second = store_contract_with_id(&platform, [2; 32], version);

        let entries = entries(
            platform
                .query_data_contracts_latest_versions_v0(
                    request(vec![vec![2; 32], vec![1; 32], vec![3; 32]], false, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );

        // One entry per requested id, in id order, the unknown one without a version.
        let summary: Vec<(Vec<u8>, Option<u32>)> = entries
            .iter()
            .map(|entry| (entry.identifier.clone(), entry.version))
            .collect();
        assert_eq!(
            summary,
            vec![
                (vec![1; 32], Some(first.version())),
                (vec![2; 32], Some(second.version())),
                (vec![3; 32], None),
            ]
        );
        assert!(
            entries.iter().all(|entry| entry.data_contract.is_none()),
            "contracts are only returned on request"
        );
    }

    #[test]
    fn test_returns_contracts_when_asked() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = store_contract_with_id(&platform, [1; 32], version);

        let entries = entries(
            platform
                .query_data_contracts_latest_versions_v0(
                    request(vec![vec![1; 32]], true, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version, Some(contract.version()));
        let bytes = entries[0]
            .data_contract
            .as_ref()
            .expect("expected the serialized contract");
        let returned = DataContract::versioned_deserialize_trusted(bytes, false, version)
            .expect("expected the returned bytes to be a data contract");
        assert_eq!(returned, contract);
    }

    #[test]
    fn test_proof_is_returned_for_stored_contracts() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        store_contract_with_id(&platform, [1; 32], version);

        let result = platform
            .query_data_contracts_latest_versions_v0(
                request(vec![vec![1; 32], vec![2; 32]], false, true),
                &state,
                version,
            )
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetDataContractsLatestVersionsResponseV0 {
                result: Some(get_data_contracts_latest_versions_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    /// The first read warms Drive's contract cache; applying an update evicts the cached copy;
    /// the next read must report the new version rather than the cached one.
    #[test]
    fn test_reflects_contract_update_after_cached_read() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let mut contract = store_contract_with_id(&platform, [1; 32], version);

        let before = entries(
            platform
                .query_data_contracts_latest_versions_v0(
                    request(vec![vec![1; 32]], false, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert_eq!(before[0].version, Some(contract.version()));

        contract.increment_version();
        store_data_contract(&platform, &contract, version);

        let after = entries(
            platform
                .query_data_contracts_latest_versions_v0(
                    request(vec![vec![1; 32]], false, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert_eq!(after[0].version, Some(contract.version()));
        assert_eq!(after[0].version, before[0].version.map(|v| v + 1));
    }

    /// Without the contracts, the proof covers the version items: it verifies as such, reports
    /// the stored version and the absence, and is a small fraction of the multi-contract
    /// proof the same request with the contracts returns.
    #[test]
    fn test_proof_without_contracts_covers_the_version_items() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = store_contract_with_id(&platform, [1; 32], version);
        let ids = vec![vec![1; 32], vec![2; 32]];

        let versions_proof = proof(
            platform
                .query_data_contracts_latest_versions_v0(
                    request(ids.clone(), false, true),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        let (_root_hash, versions) = Drive::verify_contracts_versions(
            &versions_proof.grovedb_proof,
            &[[1; 32], [2; 32]],
            version,
        )
        .expect("the proof verifies as a version item proof");
        assert_eq!(versions.get(&[1; 32]), Some(&Some(contract.version())));
        assert_eq!(versions.get(&[2; 32]), Some(&None));

        let contracts_proof = proof(
            platform
                .query_data_contracts_latest_versions_v0(request(ids, true, true), &state, version)
                .expect("expected query to succeed"),
        );
        let (_root_hash, contracts) = Drive::verify_contracts(
            &contracts_proof.grovedb_proof,
            false,
            &[[1; 32], [2; 32]],
            version,
        )
        .expect("with the contracts the proof is the multi-contract proof");
        assert_eq!(
            contracts
                .get(&[1; 32])
                .and_then(|contract| contract.as_ref())
                .map(|contract| contract.version()),
            Some(contract.version())
        );
        assert!(
            versions_proof.grovedb_proof.len() * 4 < contracts_proof.grovedb_proof.len(),
            "the version item proof ({} bytes) must be well under a quarter of the contract proof ({} bytes)",
            versions_proof.grovedb_proof.len(),
            contracts_proof.grovedb_proof.len()
        );
    }

    /// Without the contracts, the unproved answer comes from the version item on a cache
    /// miss and never loads the contract; asking for the contracts does load it.
    #[test]
    fn test_unproved_without_contracts_does_not_load_the_contract() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = store_contract_with_id(&platform, [1; 32], version);
        let cached = || {
            platform
                .drive
                .get_cached_contract_with_fetch_info([1; 32], None, &version.drive)
                .expect("expected the cache lookup to succeed")
        };
        assert!(cached().is_none(), "applying a contract does not cache it");

        let entries_without = entries(
            platform
                .query_data_contracts_latest_versions_v0(
                    request(vec![vec![1; 32]], false, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert_eq!(entries_without[0].version, Some(contract.version()));
        assert!(
            cached().is_none(),
            "the version came from the version item, not from loading the contract"
        );

        let entries_with = entries(
            platform
                .query_data_contracts_latest_versions_v0(
                    request(vec![vec![1; 32]], true, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert!(entries_with[0].data_contract.is_some());
        assert!(
            cached().is_some(),
            "asking for the contract loads it into the cache"
        );
    }

    /// Before protocol version 14 there are no version items: without the contracts the
    /// proof is still the multi-contract proof, and the unproved answer loads the contract.
    #[test]
    fn test_proof_without_contracts_is_the_contract_proof_before_protocol_version_14() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, Some(13));
        assert_eq!(version.protocol_version, 13);
        let contract = store_contract_with_id(&platform, [1; 32], version);

        let proof = proof(
            platform
                .query_data_contracts_latest_versions_v0(
                    request(vec![vec![1; 32]], false, true),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        let (_root_hash, contracts) =
            Drive::verify_contracts(&proof.grovedb_proof, false, &[[1; 32]], version)
                .expect("the proof is the multi-contract proof");
        assert_eq!(
            contracts
                .get(&[1; 32])
                .and_then(|contract| contract.as_ref())
                .map(|contract| contract.version()),
            Some(contract.version())
        );
        assert!(
            Drive::verify_contracts_versions(&proof.grovedb_proof, &[[1; 32]], version).is_err(),
            "no version item proof can be read out of it"
        );

        let entries = entries(
            platform
                .query_data_contracts_latest_versions_v0(
                    request(vec![vec![1; 32]], false, false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );
        assert_eq!(entries[0].version, Some(contract.version()));
        assert!(
            platform
                .drive
                .get_cached_contract_with_fetch_info([1; 32], None, &version.drive)
                .expect("expected the cache lookup to succeed")
                .is_some(),
            "before the version items the unproved answer loads the contract"
        );
    }
}
