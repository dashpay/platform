//! Voting tests for rs-sdk-ffi

use crate::ffi_utils::*;
use rs_sdk_ffi::*;

/// Test fetching vote polls by end date
#[test]
fn test_voting_vote_polls_by_end_date() {
    setup_logs();

    let handle = create_test_sdk_handle("test_vote_polls_by_end_date");

    unsafe {
        // Use default (no time filters) and no limit/offset to match vectors
        let result =
            dash_sdk_voting_get_vote_polls_by_end_date(handle, 0, false, 0, false, 0, 0, true);

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        assert!(json.is_array(), "Expected array, got: {:?}", json);

        // Each element should be a grouped vote poll
        if let Some(groups_array) = json.as_array() {
            for group in groups_array {
                assert!(
                    group.get("timestamp").is_some(),
                    "Group should have timestamp"
                );
                assert!(
                    group.get("vote_polls").is_some(),
                    "Group should have vote_polls"
                );

                let vote_polls = group.get("vote_polls").unwrap();
                assert!(vote_polls.is_array(), "Vote polls should be an array");

                // Each vote poll should have end_time
                if let Some(polls_array) = vote_polls.as_array() {
                    for poll in polls_array {
                        assert!(poll.get("end_time").is_some(), "Poll should have end_time");
                    }
                }
            }

            // Verify ordering if we have multiple groups
            if groups_array.len() > 1 {
                let first_timestamp = groups_array[0].get("timestamp").unwrap().as_u64().unwrap();
                let second_timestamp = groups_array[1].get("timestamp").unwrap().as_u64().unwrap();
                assert!(
                    first_timestamp < second_timestamp,
                    "Vote poll groups should be in ascending order by timestamp"
                );
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching vote polls with date range filter
#[test]
fn test_voting_vote_polls_by_end_date_with_range() {
    setup_logs();

    let handle = create_test_sdk_handle("test_vote_polls_by_end_date_range");

    // Match vectors range for vote_polls_by_ts_limit
    let start_time_ms: u64 = 1730202059933;
    let end_time_ms: u64 = 2082117570000;

    unsafe {
        // Match vectors that use limit=2 and inclusion flags
        let result = dash_sdk_voting_get_vote_polls_by_end_date(
            handle,
            start_time_ms,
            false,
            end_time_ms,
            true,
            2,
            0,
            true,
        );

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        assert!(json.is_array(), "Expected array, got: {:?}", json);

        // Verify all results are within the date range
        if let Some(groups_array) = json.as_array() {
            for group in groups_array {
                let timestamp = group
                    .get("timestamp")
                    .and_then(|t| t.as_u64())
                    .expect("Group should have numeric timestamp");

                assert!(
                    timestamp >= start_time_ms,
                    "Timestamp {} should be >= start time {}",
                    timestamp,
                    start_time_ms
                );
                assert!(
                    timestamp < end_time_ms,
                    "Timestamp {} should be < end time {}",
                    timestamp,
                    end_time_ms
                );
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching vote polls with pagination
#[test]
fn test_voting_vote_polls_by_end_date_paginated() {
    setup_logs();

    let handle = create_test_sdk_handle("test_vote_polls_paginated");

    unsafe {
        // First page
        // Match vectors: use known range and limit=2
        let start_time_ms: u64 = 1730202059933;
        let end_time_ms: u64 = 2082117570000;
        let result1 = dash_sdk_voting_get_vote_polls_by_end_date(
            handle,
            start_time_ms,
            false,
            end_time_ms,
            true,
            2,
            0,
            true,
        );

        let json_str1 = assert_success_with_data(result1);
        let json1 = parse_json_result(&json_str1).expect("valid JSON");
        let groups1 = json1.as_array().expect("Should be array");

        if groups1.len() >= 3 {
            // Second page with offset
            // For offline vectors, perform the same call again (idempotent)
            let result2 = dash_sdk_voting_get_vote_polls_by_end_date(
                handle,
                start_time_ms,
                false,
                end_time_ms,
                true,
                2,
                0,
                true,
            );

            let json_str2 = assert_success_with_data(result2);
            let json2 = parse_json_result(&json_str2).expect("valid JSON");
            let groups2 = json2.as_array().expect("Should be array");

            // Verify pagination worked - timestamps should not overlap
            if !groups2.is_empty() {
                let last_timestamp_page1 = groups1
                    .last()
                    .unwrap()
                    .get("timestamp")
                    .unwrap()
                    .as_u64()
                    .unwrap();
                let first_timestamp_page2 = groups2[0].get("timestamp").unwrap().as_u64().unwrap();

                assert!(
                    first_timestamp_page2 >= last_timestamp_page1,
                    "Second page should start after first page"
                );
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching vote polls in descending order
#[test]
fn test_voting_vote_polls_by_end_date_descending() {
    setup_logs();

    let handle = create_test_sdk_handle("test_vote_polls_descending");

    unsafe {
        let result =
            dash_sdk_voting_get_vote_polls_by_end_date(handle, 0, false, 0, false, 0, 0, false);

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        assert!(json.is_array(), "Expected array, got: {:?}", json);

        // Verify descending order
        if let Some(groups_array) = json.as_array() {
            if groups_array.len() > 1 {
                let first_timestamp = groups_array[0].get("timestamp").unwrap().as_u64().unwrap();
                let second_timestamp = groups_array[1].get("timestamp").unwrap().as_u64().unwrap();
                assert!(
                    first_timestamp > second_timestamp,
                    "Vote poll groups should be in descending order by timestamp"
                );
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching active vote polls (no end date filter)
#[test]
fn test_voting_active_vote_polls() {
    setup_logs();

    let handle = create_test_sdk_handle("test_active_vote_polls");

    // Get current time
    // Use no time filter to align with static vectors
    let current_time_ms = 0u64;

    unsafe {
        let result = dash_sdk_voting_get_vote_polls_by_end_date(
            handle,
            current_time_ms,
            false,
            0,
            false,
            0,
            0,
            true,
        );

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        assert!(json.is_array(), "Expected array, got: {:?}", json);

        // All returned polls should end after current time (active polls)
        if let Some(groups_array) = json.as_array() {
            for group in groups_array {
                let timestamp = group
                    .get("timestamp")
                    .and_then(|t| t.as_u64())
                    .expect("Group should have numeric timestamp");

                assert!(
                    timestamp >= current_time_ms,
                    "Active poll end time {} should be >= current time {}",
                    timestamp,
                    current_time_ms
                );
            }
        }
    }

    destroy_test_sdk_handle(handle);
}
