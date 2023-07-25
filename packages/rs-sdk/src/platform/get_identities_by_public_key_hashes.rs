//! `GetIdentitiesByPublicKeyHashes` request.

use dapi_grpc::platform::v0::{self as platform_proto};

/// Request Identities' bytes by public key hashes.
#[derive(Debug)]
pub struct GetIdentitiesByPublicKeyHashesRequest {
    /// Public key hashes
    pub public_key_hashes: Vec<Vec<u8>>,
}

impl From<GetIdentitiesByPublicKeyHashesRequest>
    for platform_proto::GetIdentitiesByPublicKeyHashesRequest
{
    fn from(dapi_request: GetIdentitiesByPublicKeyHashesRequest) -> Self {
        platform_proto::GetIdentitiesByPublicKeyHashesRequest {
            public_key_hashes: dapi_request.public_key_hashes,
            prove: true,
        }
    }
}
