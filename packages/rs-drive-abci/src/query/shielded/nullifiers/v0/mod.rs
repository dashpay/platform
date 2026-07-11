use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_shielded_nullifiers_request::GetShieldedNullifiersRequestV0;
use dapi_grpc::platform::v0::get_shielded_nullifiers_response::get_shielded_nullifiers_response_v0::{
    NullifierStatus, NullifierStatuses,
};
use dapi_grpc::platform::v0::get_shielded_nullifiers_response::{
    get_shielded_nullifiers_response_v0, GetShieldedNullifiersResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::shielded::paths::{
    shielded_credit_pool_nullifiers_path, shielded_credit_pool_nullifiers_path_vec,
};
use drive::error::query::QuerySyntaxError;
use drive::grovedb::{PathQuery, Query, SizedQuery};
use drive::util::grove_operations::{DirectQueryType, GroveDBToUse};

impl<C> Platform<C> {
    pub(super) fn query_shielded_nullifiers_v0(
        &self,
        GetShieldedNullifiersRequestV0 { nullifiers, prove }: GetShieldedNullifiersRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetShieldedNullifiersResponseV0>, Error> {
        let max_elements = platform_version.drive_abci.query.max_returned_elements as usize;
        if nullifiers.len() > max_elements {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!(
                    "trying to check {} nullifiers, maximum is {}",
                    nullifiers.len(),
                    max_elements
                )),
            )));
        }

        if nullifiers.is_empty() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument("nullifiers list must not be empty".to_string()),
            ));
        }

        // Validate that all nullifiers are exactly 32 bytes
        for (i, nullifier) in nullifiers.iter().enumerate() {
            if nullifier.len() != 32 {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(format!(
                        "nullifier at index {} has invalid length: expected 32 bytes, got {}",
                        i,
                        nullifier.len()
                    )),
                ));
            }
        }

        let response = if prove {
            let path_query = PathQuery {
                path: shielded_credit_pool_nullifiers_path_vec(),
                query: SizedQuery {
                    query: {
                        let mut q = Query::new();
                        q.insert_keys(nullifiers.clone());
                        q
                    },
                    limit: None,
                    offset: None,
                },
            };

            let proof = check_validation_result_with_data!(self.drive.grove_get_proved_path_query(
                &path_query,
                None,
                &mut vec![],
                &platform_version.drive,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetShieldedNullifiersResponseV0 {
                result: Some(get_shielded_nullifiers_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let nullifiers_path = shielded_credit_pool_nullifiers_path();

            let entries: Vec<NullifierStatus> = nullifiers
                .into_iter()
                .map(|nullifier| {
                    let is_spent = self.drive.grove_has_raw(
                        (&nullifiers_path).into(),
                        &nullifier,
                        DirectQueryType::StatefulDirectQuery,
                        None,
                        &mut vec![],
                        &platform_version.drive,
                    )?;

                    Ok(NullifierStatus {
                        nullifier,
                        is_spent,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;

            GetShieldedNullifiersResponseV0 {
                result: Some(
                    get_shielded_nullifiers_response_v0::Result::NullifierStatuses(
                        NullifierStatuses { entries },
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
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;

    fn nullifier(byte: u8) -> Vec<u8> {
        vec![byte; 32]
    }

    #[test]
    fn empty_nullifier_list_returns_invalid_argument() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedNullifiersRequestV0 {
            nullifiers: vec![],
            prove: false,
        };

        let result = platform
            .query_shielded_nullifiers_v0(request, &state, version)
            .expect("expected query to succeed with validation errors");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("must not be empty")
        ));
    }

    #[test]
    fn invalid_nullifier_length_returns_invalid_argument() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Second nullifier has only 16 bytes, should be rejected by per-item validation.
        let request = GetShieldedNullifiersRequestV0 {
            nullifiers: vec![nullifier(1), vec![0u8; 16]],
            prove: false,
        };

        let result = platform
            .query_shielded_nullifiers_v0(request, &state, version)
            .expect("expected query to succeed with validation errors");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg.contains("index 1") && msg.contains("32 bytes") && msg.contains("16")
        ));
    }

    #[test]
    fn too_many_nullifiers_returns_invalid_limit() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let max = version.drive_abci.query.max_returned_elements as usize;
        let nullifiers = (0..=max).map(|i| nullifier(i as u8)).collect::<Vec<_>>();

        let request = GetShieldedNullifiersRequestV0 {
            nullifiers,
            prove: false,
        };

        let result = platform
            .query_shielded_nullifiers_v0(request, &state, version)
            .expect("expected query to succeed with validation errors");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidLimit(msg))] if msg.contains("maximum is")
        ));
    }

    #[test]
    fn unspent_nullifier_returns_is_spent_false() {
        // With an empty shielded pool (no insertions), every queried nullifier must
        // come back with is_spent = false, and the set of returned statuses must
        // match the input order.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let queried = vec![nullifier(0xAA), nullifier(0xBB), nullifier(0xCC)];
        let request = GetShieldedNullifiersRequestV0 {
            nullifiers: queried.clone(),
            prove: false,
        };

        let result = platform
            .query_shielded_nullifiers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "expected no validation errors");
        let response = result.data.expect("expected response data");
        assert!(response.metadata.is_some());

        match response.result {
            Some(get_shielded_nullifiers_response_v0::Result::NullifierStatuses(statuses)) => {
                assert_eq!(statuses.entries.len(), queried.len());
                for (entry, expected) in statuses.entries.iter().zip(queried.iter()) {
                    assert_eq!(&entry.nullifier, expected);
                    assert!(
                        !entry.is_spent,
                        "expected nullifier {:?} to not be spent in empty pool",
                        entry.nullifier
                    );
                }
            }
            other => panic!("expected NullifierStatuses, got {:?}", other),
        }
    }

    #[test]
    fn prove_branch_returns_proof_bytes() {
        // With prove=true, the handler should produce a response whose result is the
        // Proof variant and metadata is populated.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetShieldedNullifiersRequestV0 {
            nullifiers: vec![nullifier(0x01)],
            prove: true,
        };

        let result = platform
            .query_shielded_nullifiers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_shielded_nullifiers_response_v0::Result::Proof(proof)) => {
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "proof bytes should not be empty"
                );
            }
            other => panic!("expected Proof result, got {:?}", other),
        }
        assert!(response.metadata.is_some(), "expected metadata");
    }
}
