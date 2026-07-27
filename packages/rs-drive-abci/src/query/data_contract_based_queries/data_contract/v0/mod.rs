use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_data_contract_request::GetDataContractRequestV0;
use dapi_grpc::platform::v0::get_data_contract_response::{
    get_data_contract_response_v0, GetDataContractResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_data_contract_v0(
        &self,
        GetDataContractRequestV0 { id, prove }: GetDataContractRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDataContractResponseV0>, Error> {
        let contract_id: Identifier =
            check_validation_result_with_data!(id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let response = if prove {
            let proof =
                self.drive
                    .prove_contract(contract_id.into_buffer(), None, platform_version)?;

            GetDataContractResponseV0 {
                result: Some(get_data_contract_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let maybe_data_contract_fetch_info = self
                .drive
                .fetch_contract(
                    contract_id.into_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()?;

            let data_contract_fetch_info =
                check_validation_result_with_data!(maybe_data_contract_fetch_info.ok_or_else(
                    || { QueryError::NotFound(format!("data contract {} not found", contract_id)) }
                ));

            let serialized_data_contract = data_contract_fetch_info
                .contract
                .serialize_to_bytes_with_platform_version(platform_version)
                .map_err(Error::Protocol)?;

            GetDataContractResponseV0 {
                result: Some(get_data_contract_response_v0::Result::DataContract(
                    serialized_data_contract,
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
    use crate::query::tests::{assert_invalid_identifier, setup_platform, store_data_contract};
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
    use dpp::tests::fixtures::get_data_contract_fixture;

    #[test]
    fn test_invalid_data_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetDataContractRequestV0 {
            id: vec![0; 8],
            prove: false,
        };

        let result = platform.query_data_contract_v0(request, &state, version);

        assert_invalid_identifier(result.unwrap());
    }

    #[test]
    fn test_data_contract_not_found() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let id = vec![0; 32];
        let request = GetDataContractRequestV0 {
            id: id.clone(),
            prove: false,
        };

        let result = platform
            .query_data_contract_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::NotFound(msg)] if msg.contains("data contract")
        ));
    }

    #[test]
    fn test_data_contract_absence_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let id = vec![0; 32];
        let request = GetDataContractRequestV0 {
            id: id.clone(),
            prove: true,
        };

        let validation_result = platform
            .query_data_contract_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            validation_result.data,
            Some(GetDataContractResponseV0 {
                result: Some(get_data_contract_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_invalid_data_contract_id_zero_length() {
        // Completely empty id bytes should fall through the same identifier
        // validation path.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetDataContractRequestV0 {
            id: vec![],
            prove: false,
        };

        let result = platform
            .query_data_contract_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("32 bytes long")
        ));
    }

    #[test]
    fn test_invalid_data_contract_id_too_long() {
        // More than 32 bytes is still invalid.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetDataContractRequestV0 {
            id: vec![1; 33],
            prove: false,
        };

        let result = platform
            .query_data_contract_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("32 bytes long")
        ));
    }

    #[test]
    fn test_invalid_data_contract_id_for_proof() {
        // Even with prove=true the identifier check must come first.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetDataContractRequestV0 {
            id: vec![0; 7],
            prove: true,
        };

        let result = platform.query_data_contract_v0(request, &state, version);

        assert_invalid_identifier(result.unwrap());
    }

    #[test]
    fn test_data_contract_found_returns_serialized() {
        // Happy path: store a contract via helpers, then fetch it without
        // proof. We should get a non-empty DataContract back.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created.data_contract(), version);
        let contract_id = created.data_contract().id();

        let request = GetDataContractRequestV0 {
            id: contract_id.to_vec(),
            prove: false,
        };

        let result = platform
            .query_data_contract_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(
            result.errors.is_empty(),
            "expected no errors, got {:?}",
            result.errors
        );
        let data = result.data.expect("expected data");
        match data.result {
            Some(get_data_contract_response_v0::Result::DataContract(bytes)) => {
                assert!(
                    !bytes.is_empty(),
                    "expected a non-empty serialized data contract"
                );
                // Round-trip through the deserializer to prove the bytes are valid.
                let round_tripped =
                    dpp::data_contract::DataContract::versioned_deserialize(&bytes, false, version)
                        .expect("contract should deserialize");
                assert_eq!(round_tripped.id(), contract_id);
            }
            other => panic!("expected DataContract result, got {:?}", other),
        }
        assert!(data.metadata.is_some(), "expected metadata present");
    }

    #[test]
    fn test_data_contract_found_with_proof() {
        // Prove path with an actually-stored contract should return a Proof
        // variant with metadata.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created.data_contract(), version);
        let contract_id = created.data_contract().id();

        let request = GetDataContractRequestV0 {
            id: contract_id.to_vec(),
            prove: true,
        };

        let result = platform
            .query_data_contract_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(
            result.errors.is_empty(),
            "expected no errors, got {:?}",
            result.errors
        );
        let data = result.data.expect("expected data");
        assert!(matches!(
            data.result,
            Some(get_data_contract_response_v0::Result::Proof(_))
        ));
        assert!(data.metadata.is_some());
    }

    #[test]
    fn test_data_contract_not_found_proof_is_still_proof() {
        // A nonexistent but well-formed identifier with prove=true should not
        // fail, but should return an (absence) Proof wrapped in the response.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetDataContractRequestV0 {
            id: vec![0xAB; 32],
            prove: true,
        };

        let result = platform
            .query_data_contract_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let data = result.data.expect("expected data");
        assert!(matches!(
            data.result,
            Some(get_data_contract_response_v0::Result::Proof(_))
        ));
    }
}
