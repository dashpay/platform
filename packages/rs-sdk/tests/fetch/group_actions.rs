use crate::fetch::common::setup_logs;
use crate::fetch::config::Config;
use crate::fetch::generated_data::*;
use assert_matches::assert_matches;
use dash_sdk::platform::group_actions::{
    GroupActionSignersQuery, GroupActionsQuery, GroupInfosQuery, GroupQuery,
};
use dash_sdk::platform::{Fetch, FetchMany};
use dpp::data_contract::group::v0::GroupV0;
use dpp::data_contract::group::{Group, GroupMemberPower};
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::v0::GroupActionV0;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::tokens::token_event::TokenEvent;
use std::collections::BTreeMap;

/// Fetches non-existing group
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_group_not_found() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_group_not_found").await;

    let query = GroupQuery {
        contract_id: DATA_CONTRACT_ID,
        group_contract_position: 99,
    };

    let group = Group::fetch(&sdk, query).await.expect("fetch group");

    assert_eq!(group, None);
}

/// Fetches existing group
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_group_fetch() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_group_fetch").await;

    let query = GroupQuery {
        contract_id: DATA_CONTRACT_ID,
        group_contract_position: 0,
    };

    let group = Group::fetch(&sdk, query).await.expect("fetch group");

    assert_matches!(
        group,
        Some(Group::V0(GroupV0 {
            members,
            required_power: 1
        })) if members == BTreeMap::from([(IDENTITY_ID_1, 1), (IDENTITY_ID_2, 1)])
    );
}

/// Fetches one group since first one exclusive
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_fetch_1_groups_since_0() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_fetch_1_groups_since_0").await;

    let query = GroupInfosQuery {
        contract_id: DATA_CONTRACT_ID,
        start_group_contract_position: Some((0, false)),
        limit: Some(1),
    };

    let groups = Group::fetch_many(&sdk, query).await.expect("fetch group");

    assert_eq!(groups.len(), 1);

    dbg!(&groups);

    assert_matches!(
        groups.get(&1),
        Some(Some(Group::V0(GroupV0 {
            members,
            required_power: 3
        }))) if members == &BTreeMap::from([(IDENTITY_ID_1, 1), (IDENTITY_ID_2, 1), (IDENTITY_ID_3, 1)])
    );
}

/// Fetches all groups since second one inclusive
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_fetch_all_groups_since_1_inclusive() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("test_fetch_all_groups_since_1_inclusive")
        .await;

    let query = GroupInfosQuery {
        contract_id: DATA_CONTRACT_ID,
        start_group_contract_position: Some((1, true)),
        limit: None,
    };

    let groups = Group::fetch_many(&sdk, query).await.expect("fetch group");

    assert_eq!(groups.len(), 2);

    assert_matches!(
        groups.get(&1),
        Some(Some(Group::V0(GroupV0 {
            members,
            required_power: 3
        }))) if members == &BTreeMap::from([(IDENTITY_ID_1, 1), (IDENTITY_ID_2, 1), (IDENTITY_ID_3, 1)])
    );

    assert_matches!(
        groups.get(&2),
        Some(Some(Group::V0(GroupV0 {
            members,
            required_power: 2
        }))) if members == &BTreeMap::from([(IDENTITY_ID_1, 1), (IDENTITY_ID_3, 1)])
    );
}

/// Fetches all group actions
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_fetch_all_group_actions() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_fetch_all_group_actions").await;

    let query = GroupActionsQuery {
        contract_id: DATA_CONTRACT_ID,
        group_contract_position: 2,
        status: GroupActionStatus::ActionActive,
        limit: None,
        start_at_action_id: None,
    };

    let group_actions = GroupAction::fetch_many(&sdk, query)
        .await
        .expect("fetch group");

    assert_eq!(group_actions.len(), 1);

    dbg!(&group_actions);

    assert_matches!(
        group_actions.get(&GROUP_ACTION_ID),
        Some(Some(GroupAction::V0(GroupActionV0 {
            contract_id: DATA_CONTRACT_ID, proposer_id: IDENTITY_ID_1, token_contract_position: 2, event: GroupActionEvent::TokenEvent(TokenEvent::Burn(10, IDENTITY_ID_1, Some(note))),
        }))) if note == "world on fire"
    );
}

/// Fetches one group action since specific one
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_fetch_one_group_action_since_existing_one_with_limit() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg
        .setup_api("test_fetch_one_group_action_since_existing_one_with_limit")
        .await;

    let query = GroupActionsQuery {
        contract_id: DATA_CONTRACT_ID,
        group_contract_position: 2,
        status: GroupActionStatus::ActionActive,
        limit: Some(1),
        start_at_action_id: Some((GROUP_ACTION_ID, true)),
    };

    let group_actions = GroupAction::fetch_many(&sdk, query)
        .await
        .expect("fetch group");

    assert_eq!(group_actions.len(), 1);

    assert_matches!(
        group_actions.get(&GROUP_ACTION_ID),
        Some(Some(GroupAction::V0(GroupActionV0 {
            contract_id: DATA_CONTRACT_ID, proposer_id: IDENTITY_ID_1, token_contract_position: 2, event: GroupActionEvent::TokenEvent(TokenEvent::Burn(10, IDENTITY_ID_1, Some(note))),
        }))) if note == "world on fire"
    );
}

/// Fetches group action signers
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_fetch_group_action_signers() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("test_fetch_group_action_signers").await;

    let query = GroupActionSignersQuery {
        contract_id: DATA_CONTRACT_ID,
        group_contract_position: 2,
        status: GroupActionStatus::ActionActive,
        action_id: GROUP_ACTION_ID,
    };

    let group_actions = GroupMemberPower::fetch_many(&sdk, query)
        .await
        .expect("fetch group");

    assert_eq!(group_actions.len(), 1);
    assert_eq!(group_actions.get(&IDENTITY_ID_1), Some(&Some(1)));
}
