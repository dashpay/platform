use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_moderation_queries::{
    identifier_from_request, list_from_request, list_to_request,
};
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_moderation_status_request::GetContractModerationStatusRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_status_response::{
    get_contract_moderation_status_response_v0,
    ContractModerationStatus as ContractModerationStatusProto,
    GetContractModerationStatusResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Returns an identity's status on the lists of a moderated contract the request names. Each
    /// list must be one the contract keeps. The proved form proves the identity's entry, present
    /// or absent, on each of them.
    pub(super) fn query_contract_moderation_status_v0(
        &self,
        GetContractModerationStatusRequestV0 {
            contract_id,
            identity_id,
            lists,
            prove,
        }: GetContractModerationStatusRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractModerationStatusResponseV0>, Error> {
        let contract_id =
            check_validation_result_with_data!(identifier_from_request(contract_id, "contract_id"));
        let identity_id =
            check_validation_result_with_data!(identifier_from_request(identity_id, "identity_id"));

        let mut requested: Vec<ContractModerationList> = Vec::with_capacity(lists.len());
        for list in lists {
            let list = check_validation_result_with_data!(list_from_request(list, "lists"));
            if requested.contains(&list) {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(format!("lists names {} twice", list)),
                ));
            }
            requested.push(list);
        }
        if requested.is_empty() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument("lists must name at least one list".to_string()),
            ));
        }

        let kept = check_validation_result_with_data!(
            self.kept_moderation_lists(contract_id, platform_version)?
        );
        for list in &requested {
            if !kept.contains(list) {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(format!(
                        "contract {} does not keep a {}",
                        contract_id, list
                    )),
                ));
            }
        }

        let response = if prove {
            let proof =
                check_validation_result_with_data!(self.drive.prove_contract_moderation_status(
                    contract_id,
                    identity_id,
                    &requested,
                    None,
                    platform_version
                ));

            GetContractModerationStatusResponseV0 {
                result: Some(get_contract_moderation_status_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let status =
                check_validation_result_with_data!(self.drive.fetch_contract_moderation_status(
                    contract_id,
                    identity_id,
                    &requested,
                    None,
                    platform_version
                ));

            GetContractModerationStatusResponseV0 {
                result: Some(get_contract_moderation_status_response_v0::Result::Status(
                    // Only the lists read are reported: `banned` stays unset when the banlist was
                    // not read, rather than saying "not banned" about it.
                    ContractModerationStatusProto {
                        banned: requested
                            .contains(&ContractModerationList::Banlist)
                            .then_some(status.banned),
                        suspended_until: status.suspended_until,
                        lists: requested
                            .iter()
                            .map(|list| list_to_request(*list))
                            .collect(),
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
    use crate::query::contract_moderation_queries::tests::{
        ban, store_contract, suspend, BANLIST, SUSPENSIONS,
    };
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::config::moderation::{
        ContractModerationListStatuses, ContractModerationStatus,
    };
    use dpp::identifier::Identifier;
    use drive::drive::Drive;

    fn request(
        contract_id: Vec<u8>,
        identity_id: Vec<u8>,
        lists: Vec<i32>,
        prove: bool,
    ) -> GetContractModerationStatusRequestV0 {
        GetContractModerationStatusRequestV0 {
            contract_id,
            identity_id,
            lists,
            prove,
        }
    }

    fn assert_invalid_argument(
        result: QueryValidationResult<GetContractModerationStatusResponseV0>,
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
                .query_contract_moderation_status_v0(request, &state, version)
                .expect("expected query to succeed")
        };

        assert_invalid_argument(
            query(request(vec![0; 8], id.clone(), vec![BANLIST], false)),
            "contract_id",
        );
        assert_invalid_argument(
            query(request(id.clone(), vec![0; 8], vec![BANLIST], false)),
            "identity_id",
        );
        assert_invalid_argument(
            query(request(id.clone(), id.clone(), vec![7], false)),
            "not a moderation list",
        );
        assert_invalid_argument(
            query(request(
                id.clone(),
                id.clone(),
                vec![BANLIST, BANLIST],
                false,
            )),
            "twice",
        );
        assert_invalid_argument(
            query(request(id.clone(), id, vec![], false)),
            "at least one",
        );
    }

    #[test]
    fn should_refuse_an_unknown_contract_and_an_unmoderated_one() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let id = vec![1; 32];

        let result = platform
            .query_contract_moderation_status_v0(
                request(vec![9; 32], id.clone(), vec![BANLIST], false),
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
                .query_contract_moderation_status_v0(
                    request(unmoderated.id().to_vec(), id.clone(), vec![BANLIST], false),
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
        let contract = store_contract(&platform, true, false, version);
        assert_invalid_argument(
            platform
                .query_contract_moderation_status_v0(
                    request(
                        contract.id().to_vec(),
                        vec![1; 32],
                        vec![BANLIST, SUSPENSIONS],
                        false,
                    ),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
            "does not keep",
        );
    }

    #[test]
    fn should_return_and_prove_the_status() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = store_contract(&platform, true, true, version);
        let banned = Identifier::from([0x21; 32]);
        let suspended = Identifier::from([0x22; 32]);
        let clean = Identifier::from([0x23; 32]);
        ban(&platform, &contract, banned, version);
        suspend(&platform, &contract, suspended, 1_234, version);

        let cases = [
            (
                banned,
                ContractModerationStatus {
                    banned: true,
                    suspended_until: None,
                },
            ),
            (
                suspended,
                ContractModerationStatus {
                    banned: false,
                    suspended_until: Some(1_234),
                },
            ),
            (clean, ContractModerationStatus::default()),
        ];
        let lists = [
            ContractModerationList::Banlist,
            ContractModerationList::Suspensions,
        ];

        for (identity_id, expected) in cases {
            let result = platform
                .query_contract_moderation_status_v0(
                    request(
                        contract.id().to_vec(),
                        identity_id.to_vec(),
                        vec![BANLIST, SUSPENSIONS],
                        false,
                    ),
                    &state,
                    version,
                )
                .expect("expected query to succeed");
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            let data = result.data.expect("expected data");
            assert!(data.metadata.is_some());
            let Some(get_contract_moderation_status_response_v0::Result::Status(status)) =
                data.result
            else {
                panic!("expected a status");
            };
            assert_eq!(status.banned, Some(expected.banned));
            assert_eq!(status.lists, vec![BANLIST, SUSPENSIONS]);
            assert_eq!(status.suspended_until, expected.suspended_until);

            let result = platform
                .query_contract_moderation_status_v0(
                    request(
                        contract.id().to_vec(),
                        identity_id.to_vec(),
                        vec![BANLIST, SUSPENSIONS],
                        true,
                    ),
                    &state,
                    version,
                )
                .expect("expected query to succeed");
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            let Some(get_contract_moderation_status_response_v0::Result::Proof(proof)) =
                result.data.expect("expected data").result
            else {
                panic!("expected a proof");
            };
            let (_, proved) = Drive::verify_contract_moderation_status(
                &proof.grovedb_proof,
                contract.id(),
                identity_id,
                &lists,
                version,
            )
            .expect("expected the proof to verify");
            assert_eq!(
                proved,
                ContractModerationListStatuses::from_status(&lists, &expected)
            );
        }
    }
    #[test]
    fn should_leave_a_list_that_was_not_read_unset_on_the_wire() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = store_contract(&platform, true, true, version);
        let banned = Identifier::from([0x31; 32]);
        ban(&platform, &contract, banned, version);

        // Only the suspension list is read: a client that does not remember what it asked for
        // must not read "not banned" about a banned identity.
        let result = platform
            .query_contract_moderation_status_v0(
                request(
                    contract.id().to_vec(),
                    banned.to_vec(),
                    vec![SUSPENSIONS],
                    false,
                ),
                &state,
                version,
            )
            .expect("expected query to succeed");
        let Some(get_contract_moderation_status_response_v0::Result::Status(status)) =
            result.data.expect("expected data").result
        else {
            panic!("expected a status");
        };
        assert_eq!(status.banned, None);
        assert_eq!(status.suspended_until, None);
        assert_eq!(status.lists, vec![SUSPENSIONS]);
    }
}
