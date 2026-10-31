use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_moderation_queries::identifier_from_request;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_team_action_signers_request::{
    ActionStatus, GetContractTeamActionSignersRequestV0,
};
use dapi_grpc::platform::v0::get_contract_team_action_signers_response::{
    get_contract_team_action_signers_response_v0, ContractTeamActionSigners,
    GetContractTeamActionSignersResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Returns who approved one of the actions a contract's seated moderation team votes on,
    /// active or closed as the request says, the proposer among them unless it left the team
    /// and its approval was dropped. None when there is no such action. The contract must keep
    /// team actions (a document type of it sets `moderatorAbilities.deleteSettled`): no other
    /// has a tree to read or prove.
    pub(super) fn query_contract_team_action_signers_v0(
        &self,
        GetContractTeamActionSignersRequestV0 {
            contract_id,
            status,
            action_id,
            prove,
        }: GetContractTeamActionSignersRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractTeamActionSignersResponseV0>, Error> {
        let contract_id =
            check_validation_result_with_data!(identifier_from_request(contract_id, "contract_id"));
        let action_id =
            check_validation_result_with_data!(identifier_from_request(action_id, "action_id"));
        let status = match ActionStatus::try_from(status) {
            Ok(ActionStatus::Active) => GroupActionStatus::ActionActive,
            Ok(ActionStatus::Closed) => GroupActionStatus::ActionClosed,
            Err(_) => {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(format!("status {status} is not an action status")),
                ))
            }
        };
        if let Err(error) = self.check_keeps_team_actions(contract_id, platform_version)? {
            return Ok(QueryValidationResult::new_with_error(error));
        }

        let response = if prove {
            let proof =
                check_validation_result_with_data!(self.drive.prove_contract_team_action_signers(
                    contract_id,
                    status,
                    action_id,
                    None,
                    platform_version
                ));

            GetContractTeamActionSignersResponseV0 {
                result: Some(get_contract_team_action_signers_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let signers =
                check_validation_result_with_data!(self.drive.fetch_contract_team_action_signers(
                    contract_id,
                    status,
                    action_id,
                    None,
                    platform_version
                ));

            GetContractTeamActionSignersResponseV0 {
                result: Some(
                    get_contract_team_action_signers_response_v0::Result::Signers(
                        ContractTeamActionSigners {
                            signer_ids: signers.iter().map(|signer| signer.to_vec()).collect(),
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
        contract_with_settled_posts, propose_settled_deletion,
    };
    use crate::query::tests::{setup_platform, store_data_contract};
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use drive::drive::Drive;

    fn request(
        contract_id: Identifier,
        status: ActionStatus,
        action: u8,
        prove: bool,
    ) -> GetContractTeamActionSignersRequestV0 {
        GetContractTeamActionSignersRequestV0 {
            contract_id: contract_id.to_vec(),
            status: status as i32,
            action_id: vec![action; 32],
            prove,
        }
    }

    #[test]
    fn should_return_and_prove_who_approved_an_action() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = contract_with_settled_posts();
        store_data_contract(&platform, &contract, version);
        propose_settled_deletion(&platform.drive, &contract, 1, false);

        let signers = |status, action| {
            let result = platform
                .query_contract_team_action_signers_v0(
                    request(contract.id(), status, action, false),
                    &state,
                    version,
                )
                .expect("expected the query to run");
            assert!(result.is_valid(), "{:?}", result.errors);
            match result.into_data().expect("expected data").result {
                Some(get_contract_team_action_signers_response_v0::Result::Signers(signers)) => {
                    signers.signer_ids
                }
                other => panic!("expected signers, got {other:?}"),
            }
        };
        assert_eq!(signers(ActionStatus::Active, 1), vec![vec![0x77; 32]]);
        // An action that is not there, or not with that status, has no approvals
        assert!(signers(ActionStatus::Closed, 1).is_empty());
        assert!(signers(ActionStatus::Active, 9).is_empty());

        let result = platform
            .query_contract_team_action_signers_v0(
                request(contract.id(), ActionStatus::Active, 1, true),
                &state,
                version,
            )
            .expect("expected the query to run");
        assert!(result.is_valid(), "{:?}", result.errors);
        let Some(get_contract_team_action_signers_response_v0::Result::Proof(proof)) =
            result.into_data().expect("expected data").result
        else {
            panic!("expected a proof");
        };
        let (_, proved) = Drive::verify_contract_team_action_signers(
            &proof.grovedb_proof,
            contract.id(),
            GroupActionStatus::ActionActive,
            Identifier::from([1; 32]),
            false,
            version,
        )
        .expect("expected the proof to verify");
        assert_eq!(proved, vec![Identifier::from([0x77; 32])]);
    }

    #[test]
    fn should_refuse_an_action_id_that_is_not_one() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = contract_with_settled_posts();
        store_data_contract(&platform, &contract, version);
        let mut short = request(contract.id(), ActionStatus::Active, 1, false);
        short.action_id = vec![1; 31];
        assert!(matches!(
            platform
                .query_contract_team_action_signers_v0(short, &state, version)
                .expect("expected the query to run")
                .errors
                .as_slice(),
            [QueryError::InvalidArgument(_)]
        ));
    }
}
