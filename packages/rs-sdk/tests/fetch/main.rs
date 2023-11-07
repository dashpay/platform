#[cfg(not(feature = "mocks"))]
compile_error!("tests require `mocks` feature to be enabled");

#[cfg(all(feature = "network-testing", feature = "offline-testing"))]
compile_error!("network-testing and offline-testing are mutually exclusive");

#[cfg(not(any(feature = "network-testing", feature = "offline-testing")))]
compile_error!("network-testing or offline-testing must be enabled for tests");

mod common;
mod config;
mod data_contract;
mod document;
mod identity;
mod mock_fetch;
mod mock_fetch_many;
