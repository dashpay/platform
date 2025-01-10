use crate::drive::RootTree;
use dpp::data_contract::GroupContractPosition;

mod estimated_costs;
mod fetch;
mod insert;

/// The key used to identify the group information in storage.
pub const GROUP_INFO_KEY: &[u8; 1] = b"I";

/// The key used to identify the group actions in storage.
pub const GROUP_ACTIONS_KEY: &[u8; 1] = b"M";

/// The key used to identify the action information in storage.
pub const ACTION_INFO_KEY: &[u8; 1] = b"I";

/// The key used to identify the action signers in storage.
pub const ACTION_SIGNERS_KEY: &[u8; 1] = b"S";

/// Group root path
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::GroupActions)]
}

/// Group root path vector
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::GroupActions as u8]]
}

/// Group path
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_contract_path(contract_id: &[u8]) -> [&[u8]; 2] {
    [Into::<&[u8; 1]>::into(RootTree::GroupActions), contract_id]
}

/// Group path vector
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_contract_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![vec![RootTree::GroupActions as u8], contract_id.to_vec()]
}

/// Group path
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_path<'a>(
    contract_id: &'a [u8],
    group_contract_position_bytes: &'a [u8],
) -> [&'a [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::GroupActions),
        contract_id,
        group_contract_position_bytes,
    ]
}

/// Group path vector
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_path_vec(
    contract_id: &[u8],
    group_contract_position: GroupContractPosition,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::GroupActions as u8],
        contract_id.to_vec(),
        group_contract_position.to_be_bytes().to_vec(),
    ]
}

/// Group action path
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_action_root_path<'a>(
    contract_id: &'a [u8],
    group_contract_position_bytes: &'a [u8],
) -> [&'a [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::GroupActions),
        contract_id,
        group_contract_position_bytes,
        GROUP_ACTIONS_KEY,
    ]
}

/// Group action path vector
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_action_root_path_vec(
    contract_id: &[u8],
    group_contract_position: GroupContractPosition,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::GroupActions as u8],
        contract_id.to_vec(),
        group_contract_position.to_be_bytes().to_vec(),
        GROUP_ACTIONS_KEY.to_vec(),
    ]
}

/// Group path
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_action_path<'a>(
    contract_id: &'a [u8],
    group_contract_position_bytes: &'a [u8],
    action_id: &'a [u8],
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::GroupActions),
        contract_id,
        group_contract_position_bytes,
        GROUP_ACTIONS_KEY,
        action_id,
    ]
}

/// Group path vector
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_action_path_vec(
    contract_id: &[u8],
    group_contract_position: GroupContractPosition,
    action_id: &[u8],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::GroupActions as u8],
        contract_id.to_vec(),
        group_contract_position.to_be_bytes().to_vec(),
        GROUP_ACTIONS_KEY.to_vec(),
        action_id.to_vec(),
    ]
}

/// Group path
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_action_signers_path<'a>(
    contract_id: &'a [u8],
    group_contract_position_bytes: &'a [u8],
    action_id: &'a [u8],
) -> [&'a [u8]; 6] {
    [
        Into::<&[u8; 1]>::into(RootTree::GroupActions),
        contract_id,
        group_contract_position_bytes,
        GROUP_ACTIONS_KEY,
        action_id,
        ACTION_SIGNERS_KEY,
    ]
}

/// Group path vector
#[cfg(any(feature = "server", feature = "verify"))]
pub fn group_action_signers_path_vec(
    contract_id: &[u8],
    group_contract_position: GroupContractPosition,
    action_id: &[u8],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::GroupActions as u8],
        contract_id.to_vec(),
        group_contract_position.to_be_bytes().to_vec(),
        GROUP_ACTIONS_KEY.to_vec(),
        action_id.to_vec(),
        ACTION_SIGNERS_KEY.to_vec(),
    ]
}
