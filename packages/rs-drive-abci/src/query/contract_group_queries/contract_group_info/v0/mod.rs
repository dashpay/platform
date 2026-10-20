use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_group_queries::identifier_from_request;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_group_info_request::GetContractGroupInfoRequestV0;
use dapi_grpc::platform::v0::get_contract_group_info_response::{
    get_contract_group_info_response_v0, ContractGroupInfo as ContractGroupInfoProto,
    GetContractGroupInfoResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::contract_group::ContractGroupInfo;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

/// The wire form of a group's stored information. Admins are empty for a single owner.
pub(crate) fn contract_group_info_to_proto(info: &ContractGroupInfo) -> ContractGroupInfoProto {
    let owner = info.owner();
    ContractGroupInfoProto {
        owner_id: owner.owner_id().to_vec(),
        admin_ids: owner
            .admin_ids()
            .map(|admins| admins.iter().map(|admin| admin.to_vec()).collect())
            .unwrap_or_default(),
        name: info.name().map(str::to_string),
        description: info.description().map(str::to_string),
    }
}

impl<C> Platform<C> {
    /// Returns a contract group's stored information, or no result when no group has the id.
    /// The proved form proves the information item, or its absence.
    pub(super) fn query_contract_group_info_v0(
        &self,
        GetContractGroupInfoRequestV0 {
            contract_group_id,
            prove,
        }: GetContractGroupInfoRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractGroupInfoResponseV0>, Error> {
        let contract_group_id = check_validation_result_with_data!(identifier_from_request(
            contract_group_id,
            "contract_group_id"
        ));

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_contract_group_info(
                contract_group_id,
                None,
                platform_version
            ));

            GetContractGroupInfoResponseV0 {
                result: Some(get_contract_group_info_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let result = check_validation_result_with_data!(self.drive.fetch_contract_group_info(
                contract_group_id,
                None,
                platform_version
            ))
            .map(|info| {
                get_contract_group_info_response_v0::Result::ContractGroupInfo(
                    contract_group_info_to_proto(&info),
                )
            });

            GetContractGroupInfoResponseV0 {
                result,
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
    use crate::query::tests::setup_platform;
    use crate::test::helpers::contract_groups::{
        owner_and_admins_info, register_group, single_owner_info,
    };
    use dpp::dashcore::Network;
    use dpp::identifier::Identifier;
    use drive::drive::Drive;

    fn request(contract_group_id: Vec<u8>, prove: bool) -> GetContractGroupInfoRequestV0 {
        GetContractGroupInfoRequestV0 {
            contract_group_id,
            prove,
        }
    }

    #[test]
    fn test_invalid_contract_group_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform
            .query_contract_group_info_v0(request(vec![0; 8], false), &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("contract_group_id")
        ));
    }

    #[test]
    fn test_absent_group_has_no_result() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let result = platform
            .query_contract_group_info_v0(request(vec![0; 32], false), &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.expect("expected data");
        assert!(data.result.is_none());
        assert!(data.metadata.is_some());
    }

    #[test]
    fn test_returns_single_owner_info() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let group_id = Identifier::from([1; 32]);
        let owner = Identifier::from([2; 32]);
        register_group(
            &platform,
            group_id,
            &single_owner_info(owner, Some("alpha"), None),
            version,
        );

        let result = platform
            .query_contract_group_info_v0(request(group_id.to_vec(), false), &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let info = match result.data.expect("expected data").result {
            Some(get_contract_group_info_response_v0::Result::ContractGroupInfo(info)) => info,
            other => panic!("expected the group info, got {other:?}"),
        };
        assert_eq!(info.owner_id, owner.to_vec());
        assert!(info.admin_ids.is_empty(), "a single owner has no admins");
        assert_eq!(info.name.as_deref(), Some("alpha"));
        assert_eq!(info.description, None);
    }

    #[test]
    fn test_returns_owner_and_admins_info() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let group_id = Identifier::from([1; 32]);
        let owner = Identifier::from([2; 32]);
        let admins = [Identifier::from([3; 32]), Identifier::from([4; 32])];
        register_group(
            &platform,
            group_id,
            &owner_and_admins_info(owner, &admins, None, Some("shared group")),
            version,
        );

        let result = platform
            .query_contract_group_info_v0(request(group_id.to_vec(), false), &state, version)
            .expect("expected query to succeed");

        let info = match result.data.expect("expected data").result {
            Some(get_contract_group_info_response_v0::Result::ContractGroupInfo(info)) => info,
            other => panic!("expected the group info, got {other:?}"),
        };
        assert_eq!(info.owner_id, owner.to_vec());
        assert_eq!(
            info.admin_ids,
            admins
                .iter()
                .map(|admin| admin.to_vec())
                .collect::<Vec<_>>()
        );
        assert_eq!(info.name, None);
        assert_eq!(info.description.as_deref(), Some("shared group"));
    }

    #[test]
    fn test_proof_verifies_presence_and_absence() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let group_id = Identifier::from([1; 32]);
        let absent_id = Identifier::from([9; 32]);
        let info = single_owner_info(Identifier::from([2; 32]), Some("alpha"), Some("first"));
        register_group(&platform, group_id, &info, version);

        for (id, expected) in [(group_id, Some(info.clone())), (absent_id, None)] {
            let result = platform
                .query_contract_group_info_v0(request(id.to_vec(), true), &state, version)
                .expect("expected query to succeed");
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            let proof = match result.data.expect("expected data").result {
                Some(get_contract_group_info_response_v0::Result::Proof(proof)) => proof,
                other => panic!("expected a proof, got {other:?}"),
            };
            let (_root_hash, verified) =
                Drive::verify_contract_group_info(&proof.grovedb_proof, id, version)
                    .expect("expected the proof to verify");
            assert_eq!(verified, expected);
        }
    }
}
