use crate::drive::RootTree;
use dpp::data_contract::GroupContractPosition;

// We should have
//           GROUP_ACTIVE_ACTIONS_TREE
//            /                    \
//   GROUP_INFO             GROUP_CLOSED_ACTIONS_TREE

/// The key used to identify the group information in storage.
pub const GROUP_INFO_KEY: &[u8; 1] = b"I";
/// The key used to identify the group actions in storage.
pub const GROUP_ACTIVE_ACTIONS_KEY: &[u8; 1] = b"M";
/// The key used to identify the group actions in storage.
pub const GROUP_CLOSED_ACTIONS_KEY: &[u8; 1] = b"X";
/// The key used to identify the action information in storage.
pub const ACTION_INFO_KEY: &[u8; 1] = b"I";
/// The key used to identify the action signers in storage.
pub const ACTION_SIGNERS_KEY: &[u8; 1] = b"S";

/// Group root path
pub fn group_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::GroupActions)]
}

/// Group root path vector
pub fn group_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::GroupActions as u8]]
}

/// Group path
pub fn group_contract_path(contract_id: &[u8]) -> [&[u8]; 2] {
    [Into::<&[u8; 1]>::into(RootTree::GroupActions), contract_id]
}

/// Group path vector
pub fn group_contract_path_vec(contract_id: &[u8]) -> Vec<Vec<u8>> {
    vec![vec![RootTree::GroupActions as u8], contract_id.to_vec()]
}

/// Group path
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
pub fn group_active_action_root_path<'a>(
    contract_id: &'a [u8],
    group_contract_position_bytes: &'a [u8],
) -> [&'a [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::GroupActions),
        contract_id,
        group_contract_position_bytes,
        GROUP_ACTIVE_ACTIONS_KEY,
    ]
}

/// Group action path vector
pub fn group_active_action_root_path_vec(
    contract_id: &[u8],
    group_contract_position: GroupContractPosition,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::GroupActions as u8],
        contract_id.to_vec(),
        group_contract_position.to_be_bytes().to_vec(),
        GROUP_ACTIVE_ACTIONS_KEY.to_vec(),
    ]
}

/// Group path
pub fn group_active_action_path<'a>(
    contract_id: &'a [u8],
    group_contract_position_bytes: &'a [u8],
    action_id: &'a [u8],
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::GroupActions),
        contract_id,
        group_contract_position_bytes,
        GROUP_ACTIVE_ACTIONS_KEY,
        action_id,
    ]
}

/// Group path vector
pub fn group_active_action_path_vec(
    contract_id: &[u8],
    group_contract_position: GroupContractPosition,
    action_id: &[u8],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::GroupActions as u8],
        contract_id.to_vec(),
        group_contract_position.to_be_bytes().to_vec(),
        GROUP_ACTIVE_ACTIONS_KEY.to_vec(),
        action_id.to_vec(),
    ]
}

/// Group path
pub fn group_action_signers_path<'a>(
    contract_id: &'a [u8],
    group_contract_position_bytes: &'a [u8],
    action_id: &'a [u8],
) -> [&'a [u8]; 6] {
    [
        Into::<&[u8; 1]>::into(RootTree::GroupActions),
        contract_id,
        group_contract_position_bytes,
        GROUP_ACTIVE_ACTIONS_KEY,
        action_id,
        ACTION_SIGNERS_KEY,
    ]
}

/// Group path vector
pub fn group_action_signers_path_vec(
    contract_id: &[u8],
    group_contract_position: GroupContractPosition,
    action_id: &[u8],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::GroupActions as u8],
        contract_id.to_vec(),
        group_contract_position.to_be_bytes().to_vec(),
        GROUP_ACTIVE_ACTIONS_KEY.to_vec(),
        action_id.to_vec(),
        ACTION_SIGNERS_KEY.to_vec(),
    ]
}

/// Group action path
pub fn group_closed_action_root_path<'a>(
    contract_id: &'a [u8],
    group_contract_position_bytes: &'a [u8],
) -> [&'a [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::GroupActions),
        contract_id,
        group_contract_position_bytes,
        GROUP_CLOSED_ACTIONS_KEY,
    ]
}

/// Group action path vector
pub fn group_closed_action_root_path_vec(
    contract_id: &[u8],
    group_contract_position: GroupContractPosition,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::GroupActions as u8],
        contract_id.to_vec(),
        group_contract_position.to_be_bytes().to_vec(),
        GROUP_CLOSED_ACTIONS_KEY.to_vec(),
    ]
}

/// Group path
pub fn group_closed_action_path<'a>(
    contract_id: &'a [u8],
    group_contract_position_bytes: &'a [u8],
    action_id: &'a [u8],
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::GroupActions),
        contract_id,
        group_contract_position_bytes,
        GROUP_CLOSED_ACTIONS_KEY,
        action_id,
    ]
}

/// Group path vector
pub fn group_closed_action_path_vec(
    contract_id: &[u8],
    group_contract_position: GroupContractPosition,
    action_id: &[u8],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::GroupActions as u8],
        contract_id.to_vec(),
        group_contract_position.to_be_bytes().to_vec(),
        GROUP_CLOSED_ACTIONS_KEY.to_vec(),
        action_id.to_vec(),
    ]
}

/// Group path
pub fn group_closed_action_signers_path<'a>(
    contract_id: &'a [u8],
    group_contract_position_bytes: &'a [u8],
    action_id: &'a [u8],
) -> [&'a [u8]; 6] {
    [
        Into::<&[u8; 1]>::into(RootTree::GroupActions),
        contract_id,
        group_contract_position_bytes,
        GROUP_CLOSED_ACTIONS_KEY,
        action_id,
        ACTION_SIGNERS_KEY,
    ]
}

/// Group path vector
pub fn group_closed_action_signers_path_vec(
    contract_id: &[u8],
    group_contract_position: GroupContractPosition,
    action_id: &[u8],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::GroupActions as u8],
        contract_id.to_vec(),
        group_contract_position.to_be_bytes().to_vec(),
        GROUP_CLOSED_ACTIONS_KEY.to_vec(),
        action_id.to_vec(),
        ACTION_SIGNERS_KEY.to_vec(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE_CONTRACT_ID: [u8; 32] = [0xAA; 32];
    const FAKE_ACTION_ID: [u8; 32] = [0xBB; 32];
    const GROUP_POS: GroupContractPosition = 5u16;

    fn root_byte() -> u8 {
        RootTree::GroupActions as u8
    }

    #[test]
    fn group_root_path_returns_single_element() {
        let path = group_root_path();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], &[root_byte()]);
    }

    #[test]
    fn group_root_path_vec_returns_single_element() {
        let path = group_root_path_vec();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], vec![root_byte()]);
    }

    #[test]
    fn group_contract_path_includes_contract_id() {
        let path = group_contract_path(&FAKE_CONTRACT_ID);
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], &[root_byte()]);
        assert_eq!(path[1], &FAKE_CONTRACT_ID);
    }

    #[test]
    fn group_contract_path_vec_includes_contract_id() {
        let path = group_contract_path_vec(&FAKE_CONTRACT_ID);
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], vec![root_byte()]);
        assert_eq!(path[1], FAKE_CONTRACT_ID.to_vec());
    }

    #[test]
    fn group_path_includes_position_bytes() {
        let pos_bytes = GROUP_POS.to_be_bytes();
        let path = group_path(&FAKE_CONTRACT_ID, &pos_bytes);
        assert_eq!(path.len(), 3);
        assert_eq!(path[2], &pos_bytes);
    }

    #[test]
    fn group_path_vec_includes_position_bytes() {
        let path = group_path_vec(&FAKE_CONTRACT_ID, GROUP_POS);
        assert_eq!(path.len(), 3);
        assert_eq!(path[2], GROUP_POS.to_be_bytes().to_vec());
    }

    #[test]
    fn group_active_action_root_path_has_active_key() {
        let pos_bytes = GROUP_POS.to_be_bytes();
        let path = group_active_action_root_path(&FAKE_CONTRACT_ID, &pos_bytes);
        assert_eq!(path.len(), 4);
        assert_eq!(path[3], GROUP_ACTIVE_ACTIONS_KEY);
    }

    #[test]
    fn group_active_action_root_path_vec_has_active_key() {
        let path = group_active_action_root_path_vec(&FAKE_CONTRACT_ID, GROUP_POS);
        assert_eq!(path.len(), 4);
        assert_eq!(path[3], GROUP_ACTIVE_ACTIONS_KEY.to_vec());
    }

    #[test]
    fn group_active_action_path_has_action_id() {
        let pos_bytes = GROUP_POS.to_be_bytes();
        let path = group_active_action_path(&FAKE_CONTRACT_ID, &pos_bytes, &FAKE_ACTION_ID);
        assert_eq!(path.len(), 5);
        assert_eq!(path[3], GROUP_ACTIVE_ACTIONS_KEY);
        assert_eq!(path[4], &FAKE_ACTION_ID);
    }

    #[test]
    fn group_active_action_path_vec_has_action_id() {
        let path = group_active_action_path_vec(&FAKE_CONTRACT_ID, GROUP_POS, &FAKE_ACTION_ID);
        assert_eq!(path.len(), 5);
        assert_eq!(path[4], FAKE_ACTION_ID.to_vec());
    }

    #[test]
    fn group_action_signers_path_has_signers_key() {
        let pos_bytes = GROUP_POS.to_be_bytes();
        let path = group_action_signers_path(&FAKE_CONTRACT_ID, &pos_bytes, &FAKE_ACTION_ID);
        assert_eq!(path.len(), 6);
        assert_eq!(path[5], ACTION_SIGNERS_KEY);
    }

    #[test]
    fn group_action_signers_path_vec_has_signers_key() {
        let path = group_action_signers_path_vec(&FAKE_CONTRACT_ID, GROUP_POS, &FAKE_ACTION_ID);
        assert_eq!(path.len(), 6);
        assert_eq!(path[5], ACTION_SIGNERS_KEY.to_vec());
    }

    #[test]
    fn group_closed_action_root_path_has_closed_key() {
        let pos_bytes = GROUP_POS.to_be_bytes();
        let path = group_closed_action_root_path(&FAKE_CONTRACT_ID, &pos_bytes);
        assert_eq!(path.len(), 4);
        assert_eq!(path[3], GROUP_CLOSED_ACTIONS_KEY);
    }

    #[test]
    fn group_closed_action_root_path_vec_has_closed_key() {
        let path = group_closed_action_root_path_vec(&FAKE_CONTRACT_ID, GROUP_POS);
        assert_eq!(path.len(), 4);
        assert_eq!(path[3], GROUP_CLOSED_ACTIONS_KEY.to_vec());
    }

    #[test]
    fn group_closed_action_path_has_action_id() {
        let pos_bytes = GROUP_POS.to_be_bytes();
        let path = group_closed_action_path(&FAKE_CONTRACT_ID, &pos_bytes, &FAKE_ACTION_ID);
        assert_eq!(path.len(), 5);
        assert_eq!(path[3], GROUP_CLOSED_ACTIONS_KEY);
        assert_eq!(path[4], &FAKE_ACTION_ID);
    }

    #[test]
    fn group_closed_action_path_vec_has_action_id() {
        let path = group_closed_action_path_vec(&FAKE_CONTRACT_ID, GROUP_POS, &FAKE_ACTION_ID);
        assert_eq!(path.len(), 5);
        assert_eq!(path[3], GROUP_CLOSED_ACTIONS_KEY.to_vec());
        assert_eq!(path[4], FAKE_ACTION_ID.to_vec());
    }

    #[test]
    fn group_closed_action_signers_path_has_signers_key() {
        let pos_bytes = GROUP_POS.to_be_bytes();
        let path = group_closed_action_signers_path(&FAKE_CONTRACT_ID, &pos_bytes, &FAKE_ACTION_ID);
        assert_eq!(path.len(), 6);
        assert_eq!(path[4], &FAKE_ACTION_ID);
        assert_eq!(path[5], ACTION_SIGNERS_KEY);
    }

    #[test]
    fn group_closed_action_signers_path_vec_has_signers_key() {
        let path =
            group_closed_action_signers_path_vec(&FAKE_CONTRACT_ID, GROUP_POS, &FAKE_ACTION_ID);
        assert_eq!(path.len(), 6);
        assert_eq!(path[4], FAKE_ACTION_ID.to_vec());
        assert_eq!(path[5], ACTION_SIGNERS_KEY.to_vec());
    }

    #[test]
    fn active_and_closed_paths_differ_only_by_key() {
        let pos_bytes = GROUP_POS.to_be_bytes();
        let active = group_active_action_root_path(&FAKE_CONTRACT_ID, &pos_bytes);
        let closed = group_closed_action_root_path(&FAKE_CONTRACT_ID, &pos_bytes);

        // First 3 components are the same
        assert_eq!(active[..3], closed[..3]);
        // 4th differs
        assert_ne!(active[3], closed[3]);
        assert_eq!(active[3], GROUP_ACTIVE_ACTIONS_KEY);
        assert_eq!(closed[3], GROUP_CLOSED_ACTIONS_KEY);
    }
}
