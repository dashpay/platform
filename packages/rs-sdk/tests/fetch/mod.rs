// TODO: Rename this test package from `fetch` to `sdk`,  'integration` or sth similar
#[cfg(not(feature = "mocks"))]
compile_error!("tests require `mocks` feature to be enabled");

#[cfg(not(any(feature = "network-testing", feature = "offline-testing")))]
compile_error!("network-testing or offline-testing must be enabled for tests");

#[cfg(feature = "mocks")]
mod broadcast;
mod common;
mod config;
mod contested_resource;
mod contested_resource_identity_votes;
mod contested_resource_polls_by_ts;
mod contested_resource_vote_state;
mod contested_resource_voters;
mod data_contract;
mod document;
mod epoch;
mod evonode;
mod generated_data;
mod group_actions;
mod identity;
mod identity_contract_nonce;
mod mock_fetch;
mod mock_fetch_many;
mod platform_events;
mod prefunded_specialized_balance;
mod protocol_version_vote_count;
mod protocol_version_votes;
mod token_direct_prices;
mod tokens;
