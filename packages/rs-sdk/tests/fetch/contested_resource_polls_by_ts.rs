//! Test VotePollsByEndDateDriveQuery

use crate::fetch::{common::setup_logs, config::Config};
use chrono::{DateTime, TimeZone, Utc};
use dash_sdk::platform::FetchMany;
use dpp::voting::vote_polls::VotePoll;
use drive::query::VotePollsByEndDateDriveQuery;
use std::collections::BTreeMap;

/// Test that we can fetch vote polls
///
/// ## Preconditions
///
/// 1. At least one vote poll exists
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn vote_polls_by_ts_ok() {
    setup_logs();

    let cfg = Config::new();

    let sdk = cfg.setup_api("vote_polls_by_ts_ok").await;
    super::contested_resource::check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisities");

    let query = VotePollsByEndDateDriveQuery {
        limit: None,
        offset: None,
        order_ascending: true,
        start_time: None,
        end_time: None,
    };

    let rss = VotePoll::fetch_many(&sdk, query)
        .await
        .expect("fetch contested resources");
    tracing::info!("vote polls retrieved: {:?}", rss);
    assert!(!rss.0.is_empty());
}

/// Test that we can fetch vote polls ordered by timestamp, ascending and descending
///
/// ## Preconditions
///
/// 1. At least 2 vote polls exist
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
#[allow(non_snake_case)]
async fn vote_polls_by_ts_order_PLAN_661() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("vote_polls_by_ts_order").await;
    super::contested_resource::check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisities");

    let base_query = VotePollsByEndDateDriveQuery {
        limit: None,
        offset: None,
        order_ascending: true,
        start_time: None,
        end_time: None,
    };

    for order_ascending in [true, false] {
        let query = VotePollsByEndDateDriveQuery {
            order_ascending,
            ..base_query.clone()
        };

        let rss = VotePoll::fetch_many(&sdk, query)
            .await
            .expect("fetch contested resources");
        tracing::debug!(order_ascending, ?rss, "vote polls retrieved");
        assert!(!rss.0.is_empty());
        let enumerated = rss.0.iter().enumerate().collect::<BTreeMap<_, _>>();
        for (i, (ts, _)) in &enumerated {
            if *i > 0 {
                let (prev_ts, _) = &enumerated[&(i - 1)];
                if order_ascending {
                    assert!(
                        ts > prev_ts,
                        "ascending order: item {} ({}) must be > than item {} ({})",
                        ts,
                        i,
                        prev_ts,
                        i - 1
                    );
                } else {
                    assert!(
                        ts < prev_ts,
                        "descending order: item {} ({}) must be < than item {} ({})",
                        ts,
                        i,
                        prev_ts,
                        i - 1
                    );
                }
            }
        }
    }
}

/// Test that we can fetch vote polls with a limit
///
/// ## Preconditions
///
/// 1. At least 3 vote poll exists
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[cfg_attr(
    not(feature = "offline-testing"),
    ignore = "requires manual DPNS names setup for masternode voting tests; see fn check_mn_voting_prerequisities()"
)]
async fn vote_polls_by_ts_limit() {
    setup_logs();

    let cfg = Config::new();
    let sdk = cfg.setup_api("vote_polls_by_ts_limit").await;
    super::contested_resource::check_mn_voting_prerequisities(&cfg)
        .await
        .expect("prerequisities");

    // Given index with more than 2 contested resources; note LIMIT must be > 1
    const LIMIT: usize = 2;
    const LIMIT_ALL: usize = 100;

    let end_time: DateTime<Utc> = Utc.with_ymd_and_hms(2035, 12, 24, 13, 59, 30).unwrap();

    let query_all = VotePollsByEndDateDriveQuery {
        limit: Some(LIMIT_ALL as u16),
        offset: None,
        order_ascending: true,
        start_time: None,
        end_time: Some((end_time.timestamp_millis() as u64, true)), // 1 month in future
    };

    let all = VotePoll::fetch_many(&sdk, query_all.clone())
        .await
        .expect("fetch vote polls");
    // this counts timestamps, not vote polls themselves
    let count_all_timestamps = all.0.len();
    assert_ne!(count_all_timestamps, 0, "at least one vote poll expected");

    let all_values = all.0.into_iter().collect::<Vec<_>>();

    tracing::debug!(
        count = count_all_timestamps,
        all = ?all_values,
        "All results"
    );

    for inclusive in [true, false] {
        // When we query for 2 contested values at a time, we get all of them
        let mut checked_count: usize = 0;
        let mut start_time = None;

        loop {
            let query = VotePollsByEndDateDriveQuery {
                limit: Some(LIMIT as u16),
                start_time,
                ..query_all.clone()
            };

            let rss = VotePoll::fetch_many(&sdk, query)
                .await
                .expect("fetch vote polls");

            let Some(last) = rss.0.last() else {
                // no more vote polls
                break;
            };

            tracing::debug!(polls=?rss, inclusive, ?start_time, checked_count, "Vote pools");
            let length = rss.0.len();

            for (j, current) in rss.0.iter().enumerate() {
                let all_idx = if inclusive && (checked_count > 0) {
                    j + checked_count - 1
                } else {
                    j + checked_count
                };
                let expected = &all_values[all_idx];
                assert_eq!(
                    current.0, expected.0,
                    "inclusive {}: timestamp should match",
                    inclusive
                );
                assert_eq!(
                    &current.1, &expected.1,
                    "inclusive {}: vote polls should match",
                    inclusive
                );
            }

            tracing::debug!(polls=?rss, checked_count, ?start_time, "Vote polls");

            start_time = Some((last.0, inclusive));
            // when inclusive, we include the first item in checked_count only on first iteration
            checked_count += if inclusive && checked_count != 0 {
                length - 1
            } else {
                length
            };

            if (inclusive && length == 1) || (!inclusive && length == 0) {
                break;
            }
        }

        assert_eq!(
            checked_count, count_all_timestamps,
            "all vote polls should be checked when inclusive is {}",
            inclusive
        );
    }
}
