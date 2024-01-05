// TODO: Rename this test package from `fetch` to `sdk`,  'integration` or sth similar
#[cfg(not(feature = "mocks"))]
compile_error!("tests require `mocks` feature to be enabled");

#[cfg(not(any(feature = "network-testing", feature = "offline-testing")))]
compile_error!("network-testing or offline-testing must be enabled for tests");

mod broadcast;
mod common;
mod config;
mod data_contract;
mod document;
mod epoch;
mod identity;
mod mock_fetch;
mod mock_fetch_many;
mod protocol_version_vote_count;
mod protocol_version_votes;
mod wallet;
