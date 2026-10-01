use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_moderation_queries::{identifier_from_request, team_action_to_response};
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_team_actions_request::{
    ActionStatus, GetContractTeamActionsRequestV0, StartAtActionId,
};
use dapi_grpc::platform::v0::get_contract_team_actions_response::{
    get_contract_team_actions_response_v0, ContractTeamActions, GetContractTeamActionsResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract::moderation::types::ContractTeamActionsQuery;
use drive::drive::Drive;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Returns one page of the actions a contract's seated moderation team votes on, active or
    /// closed, in action id order. The contract must keep team actions (a document type of it
    /// sets `moderatorAbilities.deleteSettled`): no other has a tree to read or prove.
    pub(super) fn query_contract_team_actions_v0(
        &self,
        GetContractTeamActionsRequestV0 {
            contract_id,
            status,
            start_at_action_id,
            count,
            prove,
        }: GetContractTeamActionsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractTeamActionsResponseV0>, Error> {
        let contract_id =
            check_validation_result_with_data!(identifier_from_request(contract_id, "contract_id"));
        let status = match ActionStatus::try_from(status) {
            Ok(ActionStatus::Active) => GroupActionStatus::ActionActive,
            Ok(ActionStatus::Closed) => GroupActionStatus::ActionClosed,
            Err(_) => {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(format!("status {status} is not an action status")),
                ))
            }
        };
        let start_at = check_validation_result_with_data!(start_at_action_id
            .map(
                |StartAtActionId {
                     start_action_id,
                     start_action_id_included,
                 }| {
                    identifier_from_request(start_action_id, "start_action_id")
                        .map(|action_id| (action_id, start_action_id_included))
                }
            )
            .transpose());
        let query = ContractTeamActionsQuery {
            status,
            start_at,
            // The page size when the request names none: the largest page, the number the
            // proof verifier assumes as well. A count no u16 holds is past every bound, and is
            // refused below as the largest u16 is.
            limit: count.map_or(
                platform_version.drive_abci.query.max_returned_elements,
                |count| u16::try_from(count).unwrap_or(u16::MAX),
            ),
        };
        // The bounds of a page are Drive's, which the proof verifier calls too. Refused here as
        // an invalid argument: left to the fetch or the proof below, the same refusal would
        // reach the client as an unknown node failure.
        if let Err(error) = Drive::check_contract_team_actions_query(&query, platform_version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(error.to_string()),
            ));
        }
        if let Err(error) = self.check_keeps_team_actions(contract_id, platform_version)? {
            return Ok(QueryValidationResult::new_with_error(error));
        }

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_contract_team_actions(
                contract_id,
                &query,
                None,
                platform_version
            ));

            GetContractTeamActionsResponseV0 {
                result: Some(get_contract_team_actions_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let entries = check_validation_result_with_data!(self
                .drive
                .fetch_contract_team_actions(contract_id, &query, None, platform_version));

            GetContractTeamActionsResponseV0 {
                result: Some(get_contract_team_actions_response_v0::Result::Actions(
                    ContractTeamActions {
                        actions: entries.into_iter().map(team_action_to_response).collect(),
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
        contract_with_settled_posts, propose_settled_deletion, settled_deletion_proposal,
    };
    use crate::query::tests::{setup_platform, store_data_contract};
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::DataContract;
    use dpp::identifier::Identifier;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::drive::contract::moderation::types::ContractTeamActionEntry;

    fn request(
        contract: &DataContract,
        status: ActionStatus,
        start_at: Option<(u8, bool)>,
        count: Option<u32>,
        prove: bool,
    ) -> GetContractTeamActionsRequestV0 {
        GetContractTeamActionsRequestV0 {
            contract_id: contract.id().to_vec(),
            status: status as i32,
            start_at_action_id: start_at.map(|(seed, included)| StartAtActionId {
                start_action_id: vec![seed; 32],
                start_action_id_included: included,
            }),
            count,
            prove,
        }
    }

    /// A proposal as the page returns it: the proposer's approval is its one approval.
    fn entry(seed: u8) -> ContractTeamActionEntry {
        ContractTeamActionEntry {
            action_id: Identifier::from([seed; 32]),
            action: settled_deletion_proposal(seed),
            approval_count: 1,
        }
    }

    #[test]
    fn should_return_a_page_of_the_active_or_the_closed_actions() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = contract_with_settled_posts();
        store_data_contract(&platform, &contract, version);
        for seed in [1, 2, 3] {
            propose_settled_deletion(&platform.drive, &contract, seed, false);
        }
        propose_settled_deletion(&platform.drive, &contract, 4, true);

        let actions = |status, start_at, count| {
            let result = platform
                .query_contract_team_actions_v0(
                    request(&contract, status, start_at, count, false),
                    &state,
                    version,
                )
                .expect("expected the query to run");
            assert!(result.is_valid(), "{:?}", result.errors);
            match result.into_data().expect("expected data").result {
                Some(get_contract_team_actions_response_v0::Result::Actions(actions)) => {
                    actions.actions
                }
                other => panic!("expected actions, got {other:?}"),
            }
        };

        let proto = |seed| team_action_to_response(entry(seed));
        assert_eq!(
            actions(ActionStatus::Active, None, Some(2)),
            vec![proto(1), proto(2)]
        );
        assert_eq!(
            actions(ActionStatus::Active, Some((2, false)), None),
            vec![proto(3)]
        );
        assert_eq!(
            actions(ActionStatus::Active, Some((2, true)), None),
            vec![proto(2), proto(3)]
        );
        assert_eq!(actions(ActionStatus::Closed, None, None), vec![proto(4)]);
    }

    #[test]
    fn should_prove_the_actions_the_verifier_reads_back() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = contract_with_settled_posts();
        store_data_contract(&platform, &contract, version);
        propose_settled_deletion(&platform.drive, &contract, 1, false);

        let result = platform
            .query_contract_team_actions_v0(
                request(&contract, ActionStatus::Active, None, None, true),
                &state,
                version,
            )
            .expect("expected the query to run");
        assert!(result.is_valid(), "{:?}", result.errors);
        let Some(get_contract_team_actions_response_v0::Result::Proof(proof)) =
            result.into_data().expect("expected data").result
        else {
            panic!("expected a proof");
        };
        // No count named: the verifier assumes the largest page, as the node does.
        let (_, entries) = Drive::verify_contract_team_actions(
            &proof.grovedb_proof,
            contract.id(),
            &ContractTeamActionsQuery {
                status: GroupActionStatus::ActionActive,
                start_at: None,
                limit: version.drive_abci.query.max_returned_elements,
            },
            false,
            version,
        )
        .expect("expected the proof to verify");
        assert_eq!(entries, vec![entry(1)]);
    }

    #[test]
    fn should_refuse_a_contract_without_team_actions_and_a_page_out_of_bounds() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = contract_with_settled_posts();
        store_data_contract(&platform, &contract, version);
        let mut unsettled =
            get_data_contract_fixture(None, 0, version.protocol_version).data_contract_owned();
        unsettled.set_id(Identifier::from([0x55; 32]));
        store_data_contract(&platform, &unsettled, version);

        let refusal = |request| {
            platform
                .query_contract_team_actions_v0(request, &state, version)
                .expect("expected the query to run")
                .errors
        };
        assert!(matches!(
            refusal(request(&unsettled, ActionStatus::Active, None, None, false)).as_slice(),
            [QueryError::InvalidArgument(_)]
        ));
        assert!(matches!(
            refusal(request(
                &contract,
                ActionStatus::Active,
                None,
                Some(0),
                true
            ))
            .as_slice(),
            [QueryError::InvalidArgument(_)]
        ));
        let mut unknown_status = request(&contract, ActionStatus::Active, None, None, false);
        unknown_status.status = 7;
        assert!(matches!(
            refusal(unknown_status).as_slice(),
            [QueryError::InvalidArgument(_)]
        ));
    }
}
