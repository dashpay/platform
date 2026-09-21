use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_keys_remaining_budgets_request::GetIdentityKeysRemainingBudgetsRequestV0;
use dapi_grpc::platform::v0::get_identity_keys_remaining_budgets_response::get_identity_keys_remaining_budgets_response_v0::{
    KeyRemainingBudgetEntry, KeysRemainingBudgets,
};
use dapi_grpc::platform::v0::get_identity_keys_remaining_budgets_response::{
    get_identity_keys_remaining_budgets_response_v0, GetIdentityKeysRemainingBudgetsResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;
use std::collections::BTreeSet;

impl<C> Platform<C> {
    pub(super) fn query_identity_keys_remaining_budgets_v0(
        &self,
        GetIdentityKeysRemainingBudgetsRequestV0 {
            identity_id,
            key_ids,
            prove,
        }: GetIdentityKeysRemainingBudgetsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityKeysRemainingBudgetsResponseV0>, Error> {
        if key_ids.is_empty() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument("key_ids must name at least one key".to_string()),
            ));
        }
        if key_ids.len() > platform_version.drive_abci.query.max_returned_elements as usize {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!(
                    "trying to get the remaining budgets of {} keys, maximum is {}",
                    key_ids.len(),
                    platform_version.drive_abci.query.max_returned_elements
                )),
            )));
        }
        // The verifier rebuilds the path query from the request and expects one answer per key
        // id it asked for. A query holds each key once, so a repeated id could never verify.
        if key_ids.iter().collect::<BTreeSet<_>>().len() != key_ids.len() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument("key_ids must not repeat a key id".to_string()),
            ));
        }
        let identity_id: Identifier =
            check_validation_result_with_data!(identity_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "identity_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_identity_keys_remaining_budgets(
                    identity_id.into_buffer(),
                    key_ids.as_slice(),
                    None,
                    platform_version,
                ));

            GetIdentityKeysRemainingBudgetsResponseV0 {
                result: Some(
                    get_identity_keys_remaining_budgets_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                            .map(|(_, proof)| proof)?,
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let entries = check_validation_result_with_data!(self
                .drive
                .fetch_identity_keys_remaining_budgets(
                    identity_id.into_buffer(),
                    key_ids.as_slice(),
                    None,
                    platform_version,
                ))
            .into_iter()
            .map(|(key_id, remaining_budget)| KeyRemainingBudgetEntry {
                key_id,
                remaining_budget,
            })
            .collect();

            GetIdentityKeysRemainingBudgetsResponseV0 {
                result: Some(
                    get_identity_keys_remaining_budgets_response_v0::Result::KeysRemainingBudgets(
                        KeysRemainingBudgets { entries },
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
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::fee::Credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyID};
    use drive::drive::Drive;
    use std::collections::BTreeMap;

    const BUDGETED_KEY_ID: KeyID = 5;
    const BUDGET: Credits = 1_000_000;

    /// An identity with ordinary keys 0 to 4 and the budgeted key 5.
    fn seed_identity(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> [u8; 32] {
        let mut identity = Identity::random_identity(5, Some(11), platform_version)
            .expect("expected a random identity");
        identity.add_public_key(
            IdentityPublicKey::random_authentication_keys(
                BUDGETED_KEY_ID,
                1,
                Some(12),
                platform_version,
            )
            .remove(0)
            .with_limits(Some(BUDGET), None),
        );
        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the identity");
        identity.id().to_buffer()
    }

    fn request(
        identity_id: Vec<u8>,
        key_ids: Vec<KeyID>,
        prove: bool,
    ) -> GetIdentityKeysRemainingBudgetsRequestV0 {
        GetIdentityKeysRemainingBudgetsRequestV0 {
            identity_id,
            key_ids,
            prove,
        }
    }

    #[test]
    fn should_reject_a_malformed_request() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let too_many = version.drive_abci.query.max_returned_elements as u32 + 1;

        for (bad_request, expected) in [
            (request(vec![0; 8], vec![1], false), "identity_id"),
            (request(vec![0; 32], vec![], false), "at least one key"),
            (request(vec![0; 32], vec![1, 2, 1], true), "must not repeat"),
            (
                request(vec![0; 32], (0..too_many).collect(), false),
                "maximum is",
            ),
        ] {
            let result = platform
                .query_identity_keys_remaining_budgets_v0(bad_request, &state, version)
                .expect("expected a validation result, not an internal error");
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [QueryError::InvalidArgument(message)] | [QueryError::Query(QuerySyntaxError::InvalidLimit(message))]
                        if message.contains(expected)
                ),
                "expected an error about `{expected}`, got {:?}",
                result.errors
            );
        }
    }

    #[test]
    fn should_answer_the_remaining_budget_of_each_requested_key() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let identity_id = seed_identity(&platform, version);
        platform
            .drive
            .deduct_from_identity_key_budget(identity_id, BUDGETED_KEY_ID, 250_000, None, version)
            .expect("expected to deduct");

        let result = platform
            .query_identity_keys_remaining_budgets_v0(
                request(identity_id.to_vec(), vec![BUDGETED_KEY_ID, 0, 42], false),
                &state,
                version,
            )
            .expect("expected the query to succeed");
        let Some(GetIdentityKeysRemainingBudgetsResponseV0 {
            result:
                Some(get_identity_keys_remaining_budgets_response_v0::Result::KeysRemainingBudgets(
                    KeysRemainingBudgets { entries },
                )),
            metadata: Some(_),
        }) = result.data
        else {
            panic!("expected remaining budgets, got errors {:?}", result.errors);
        };
        let answered: BTreeMap<KeyID, Option<Credits>> = entries
            .into_iter()
            .map(|entry| (entry.key_id, entry.remaining_budget))
            .collect();
        assert_eq!(
            answered,
            BTreeMap::from([(0, None), (BUDGETED_KEY_ID, Some(750_000)), (42, None)])
        );
    }

    #[test]
    fn should_answer_with_a_proof_the_drive_verifier_accepts() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let identity_id = seed_identity(&platform, version);

        // An identity with a budgeted key, and one that does not exist at all.
        for (queried_identity, key_ids, expected) in [
            (
                identity_id,
                vec![BUDGETED_KEY_ID, 0],
                BTreeMap::from([(0, None), (BUDGETED_KEY_ID, Some(BUDGET))]),
            ),
            ([7u8; 32], vec![3], BTreeMap::from([(3, None)])),
        ] {
            let result = platform
                .query_identity_keys_remaining_budgets_v0(
                    request(queried_identity.to_vec(), key_ids.clone(), true),
                    &state,
                    version,
                )
                .expect("expected the query to succeed");
            let Some(GetIdentityKeysRemainingBudgetsResponseV0 {
                result: Some(get_identity_keys_remaining_budgets_response_v0::Result::Proof(proof)),
                metadata: Some(_),
            }) = result.data
            else {
                panic!("expected a proof, got errors {:?}", result.errors);
            };

            let (_, proved): (_, BTreeMap<KeyID, Option<Credits>>) =
                Drive::verify_identity_keys_remaining_budgets(
                    proof.grovedb_proof.as_slice(),
                    queried_identity,
                    key_ids.as_slice(),
                    false,
                    version,
                )
                .expect("expected the proof to verify");
            assert_eq!(proved, expected);
        }
    }

    #[test]
    fn should_refuse_the_query_before_protocol_version_14() {
        // Keys cannot carry a budget before protocol version 14 and the subtree does not exist:
        // the query answers with an error instead of an internal failure.
        let (platform, state, version) = setup_platform(None, Network::Testnet, Some(13));
        for prove in [false, true] {
            let result = platform
                .query_identity_keys_remaining_budgets_v0(
                    request(vec![0; 32], vec![1], prove),
                    &state,
                    version,
                )
                .expect("expected a validation result, not an internal error");
            assert!(!result.errors.is_empty());
        }
    }
}
