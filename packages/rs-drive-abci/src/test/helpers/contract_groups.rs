//! Contract group fixtures shared by the query tests and the validation tests.

use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::block::block_info::BlockInfo;
use dpp::contract_group::{
    ContractGroupInfo, ContractGroupInfoV0, ContractGroupMember, ContractGroupMembership,
    ContractGroupOwner,
};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use std::collections::BTreeSet;

/// The information of a group with one owner.
pub fn single_owner_info(
    owner: Identifier,
    name: Option<&str>,
    description: Option<&str>,
) -> ContractGroupInfo {
    ContractGroupInfo::V0(ContractGroupInfoV0 {
        owner: ContractGroupOwner::SingleOwner(owner),
        name: name.map(str::to_string),
        description: description.map(str::to_string),
    })
}

/// The information of a group with an owner and admins.
pub fn owner_and_admins_info(
    owner: Identifier,
    admins: &[Identifier],
    name: Option<&str>,
    description: Option<&str>,
) -> ContractGroupInfo {
    ContractGroupInfo::V0(ContractGroupInfoV0 {
        owner: ContractGroupOwner::OwnerAndAdmins {
            owner,
            admins: admins.iter().copied().collect::<BTreeSet<_>>(),
        },
        name: name.map(str::to_string),
        description: description.map(str::to_string),
    })
}

/// Registers a group straight into Drive's state.
pub fn register_group(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract_group_id: Identifier,
    info: &ContractGroupInfo,
    platform_version: &PlatformVersion,
) {
    platform
        .drive
        .insert_contract_group(
            contract_group_id,
            info,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to register the group");
}

/// Records a new contract's memberships straight into Drive's state.
pub fn join_group(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract_id: Identifier,
    memberships: &[(Identifier, ContractGroupMember)],
    platform_version: &PlatformVersion,
) {
    let memberships: Vec<ContractGroupMembership> = memberships
        .iter()
        .map(|(contract_group_id, member)| ContractGroupMembership {
            contract_group_id: *contract_group_id,
            member: member.clone(),
        })
        .collect();
    platform
        .drive
        .insert_contract_group_memberships(
            contract_id,
            &memberships,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("expected to record the memberships");
}
