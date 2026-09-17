use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_group_queries::identifier_from_request;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_groups_for_contract_request::GetContractGroupsForContractRequestV0;
use dapi_grpc::platform::v0::get_contract_groups_for_contract_response::{
    get_contract_groups_for_contract_response_v0, ContractGroupMemberships,
    DocumentTypeMemberships, GetContractGroupsForContractResponseV0, TokenMemberships,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract_groups::types::ContractGroupMembershipsForContract;
use drive::util::grove_operations::GroveDBToUse;

/// The wire form of a contract's memberships: every list is in key order.
pub(crate) fn memberships_to_proto(
    memberships: ContractGroupMembershipsForContract,
) -> ContractGroupMemberships {
    ContractGroupMemberships {
        contract_group_ids: memberships
            .contract
            .into_iter()
            .map(|group_id| group_id.to_vec())
            .collect(),
        document_types: memberships
            .document_types
            .into_iter()
            .map(|(document_type_name, group_ids)| DocumentTypeMemberships {
                document_type_name,
                contract_group_ids: group_ids.into_iter().map(|id| id.to_vec()).collect(),
            })
            .collect(),
        tokens: memberships
            .tokens
            .into_iter()
            .map(|(token_position, group_ids)| TokenMemberships {
                token_position: token_position as u32,
                contract_group_ids: group_ids.into_iter().map(|id| id.to_vec()).collect(),
            })
            .collect(),
    }
}

impl<C> Platform<C> {
    /// Returns the contract groups a contract belongs to, as a whole, through its document
    /// types and through its tokens. A contract in no group, or no contract at all, answers
    /// with empty lists; the proved form proves that too.
    pub(super) fn query_contract_groups_for_contract_v0(
        &self,
        GetContractGroupsForContractRequestV0 { contract_id, prove }: GetContractGroupsForContractRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractGroupsForContractResponseV0>, Error> {
        let contract_id =
            check_validation_result_with_data!(identifier_from_request(contract_id, "contract_id"));

        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_contract_group_memberships_for_contract(
                    contract_id,
                    None,
                    platform_version
                ));

            GetContractGroupsForContractResponseV0 {
                result: Some(get_contract_groups_for_contract_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let memberships = check_validation_result_with_data!(self
                .drive
                .fetch_contract_group_memberships_for_contract(
                    contract_id,
                    None,
                    platform_version
                ));

            GetContractGroupsForContractResponseV0 {
                result: Some(
                    get_contract_groups_for_contract_response_v0::Result::ContractGroupMemberships(
                        memberships_to_proto(memberships),
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
    use crate::error::query::QueryError;
    use crate::query::contract_group_queries::tests::{
        join_group, register_group, single_owner_info,
    };
    use crate::query::tests::setup_platform;
    use dpp::contract_group::ContractGroupMember;
    use dpp::dashcore::Network;
    use dpp::identifier::Identifier;
    use drive::drive::Drive;

    fn request(contract_id: Vec<u8>, prove: bool) -> GetContractGroupsForContractRequestV0 {
        GetContractGroupsForContractRequestV0 { contract_id, prove }
    }

    fn memberships(
        result: QueryValidationResult<GetContractGroupsForContractResponseV0>,
    ) -> ContractGroupMemberships {
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        match result.data.expect("expected data").result {
            Some(
                get_contract_groups_for_contract_response_v0::Result::ContractGroupMemberships(
                    memberships,
                ),
            ) => memberships,
            other => panic!("expected memberships, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform
            .query_contract_groups_for_contract_v0(request(vec![0; 8], false), &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("contract_id")
        ));
    }

    #[test]
    fn test_contract_in_no_group_has_empty_memberships() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let memberships = memberships(
            platform
                .query_contract_groups_for_contract_v0(request(vec![0; 32], false), &state, version)
                .expect("expected query to succeed"),
        );

        assert_eq!(memberships, ContractGroupMemberships::default());
    }

    #[test]
    fn test_returns_every_kind_of_membership() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let owner = Identifier::from([2; 32]);
        let group_a = Identifier::from([10; 32]);
        let group_b = Identifier::from([11; 32]);
        let contract = Identifier::from([20; 32]);
        for group in [group_a, group_b] {
            register_group(
                &platform,
                group,
                &single_owner_info(owner, None, None),
                version,
            );
        }
        join_group(
            &platform,
            contract,
            &[
                (group_a, ContractGroupMember::Contract),
                (
                    group_b,
                    ContractGroupMember::DocumentType("note".to_string()),
                ),
                (group_a, ContractGroupMember::Token(0)),
                (group_b, ContractGroupMember::Token(0)),
                (group_a, ContractGroupMember::Token(3)),
            ],
            version,
        );

        let memberships = memberships(
            platform
                .query_contract_groups_for_contract_v0(
                    request(contract.to_vec(), false),
                    &state,
                    version,
                )
                .expect("expected query to succeed"),
        );

        assert_eq!(memberships.contract_group_ids, vec![group_a.to_vec()]);
        assert_eq!(
            memberships.document_types,
            vec![DocumentTypeMemberships {
                document_type_name: "note".to_string(),
                contract_group_ids: vec![group_b.to_vec()],
            }]
        );
        assert_eq!(
            memberships.tokens,
            vec![
                TokenMemberships {
                    token_position: 0,
                    contract_group_ids: vec![group_a.to_vec(), group_b.to_vec()],
                },
                TokenMemberships {
                    token_position: 3,
                    contract_group_ids: vec![group_a.to_vec()],
                },
            ]
        );
    }

    #[test]
    fn test_proof_verifies_memberships_and_their_absence() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let owner = Identifier::from([2; 32]);
        let group = Identifier::from([10; 32]);
        let contract = Identifier::from([20; 32]);
        let other_contract = Identifier::from([21; 32]);
        register_group(
            &platform,
            group,
            &single_owner_info(owner, None, None),
            version,
        );
        join_group(
            &platform,
            contract,
            &[(group, ContractGroupMember::DocumentType("note".to_string()))],
            version,
        );

        let mut expected = ContractGroupMembershipsForContract::default();
        expected
            .document_types
            .entry("note".to_string())
            .or_default()
            .insert(group);

        for (id, expected) in [
            (contract, expected),
            (
                other_contract,
                ContractGroupMembershipsForContract::default(),
            ),
        ] {
            let result = platform
                .query_contract_groups_for_contract_v0(request(id.to_vec(), true), &state, version)
                .expect("expected query to succeed");
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            let proof = match result.data.expect("expected data").result {
                Some(get_contract_groups_for_contract_response_v0::Result::Proof(proof)) => proof,
                other => panic!("expected a proof, got {other:?}"),
            };
            let (_root_hash, verified) = Drive::verify_contract_group_memberships_for_contract(
                &proof.grovedb_proof,
                id,
                version,
            )
            .expect("expected the proof to verify");
            assert_eq!(verified, expected);
        }
    }
}
