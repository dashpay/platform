mod fetch;
mod fetch_json;
mod fetch_many;
mod fetch_with_serialization;
mod history;
mod info;

// Re-export all public functions for convenient access
pub use fetch::dash_sdk_data_contract_fetch;
pub use fetch_json::dash_sdk_data_contract_fetch_json;
pub use fetch_many::dash_sdk_data_contracts_fetch_many;
pub use fetch_with_serialization::{
    dash_sdk_data_contract_fetch_result_free, dash_sdk_data_contract_fetch_with_serialization,
    DashSDKDataContractFetchResult,
};
pub use history::dash_sdk_data_contract_fetch_history;
