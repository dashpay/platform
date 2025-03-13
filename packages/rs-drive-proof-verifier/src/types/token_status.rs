use crate::types::RetrievedObjects;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_token_statuses_request, GetTokenStatusesRequest, GetTokenStatusesResponse, Proof,
    ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::identifier::Identifier;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

/// Token statuses (i.e. is token paused or not)
/// Token ID to token status
pub type TokenStatuses = RetrievedObjects<Identifier, TokenStatus>;
