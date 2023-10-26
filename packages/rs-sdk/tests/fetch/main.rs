#[cfg(not(feature = "mocks"))]
compile_error!("tests require `mocks` feature to be enabled");

mod common;
mod data_contract;
mod document;
mod identity;
mod mock_fetch;
mod mock_fetch_many;
