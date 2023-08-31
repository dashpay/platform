//! Platform DAPI requests.

// TODO: rs-sdk should define a proper user-facing API hiding transport and serialization
// details, however we don't have any data model yet, so for now it will re-export proto
// generated types. Later these re-exports could be swapped with actual rs-sdk's requests
// and while it will change the substance, the API structure will remain the same.

use dapi_grpc::platform::v0::{self as platform_proto};

pub use platform_proto::BroadcastStateTransitionRequest;
pub use platform_proto::GetConsensusParamsRequest;
pub use platform_proto::GetDataContractHistoryRequest;
pub use platform_proto::GetDataContractRequest;
pub use platform_proto::GetDocumentsRequest;
pub use platform_proto::GetIdentitiesByPublicKeyHashesRequest;
pub use platform_proto::GetIdentityRequest;
pub use platform_proto::WaitForStateTransitionResultRequest;

pub use rs_dapi_client::DapiClient as PlatformClient;

pub mod data_contract;
pub mod get_documents;
pub mod identity;
