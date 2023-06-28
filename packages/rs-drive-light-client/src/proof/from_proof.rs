use std::fmt::Debug;

use crate::Error;
use dapi_grpc::platform::v0::{self as platform};
use dpp::prelude::{Identifier, Identity};
pub use drive::drive::verify::RootHash;
use drive::drive::Drive;

use super::verify::verify_tenderdash_proof;

// #[cfg(feature = "mockall")]

/// Create an object based on proof received from DAPI
pub trait FromProof<Req, Resp> {
    /// Parse and verify the received proof and retrieve the requested object, if any.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `provider`: A callback implementing [QuorumInfoProvider] that provides quorum details required to verify the proof.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(object))` when the requested object was found in the proof.
    /// * `Ok(None)` when the requested object was not found in the proof; this can be interpreted as proof of non-existence.
    /// * `Err(Error)` when either the provided data is invalid or proof validation failed.
    fn maybe_from_proof(
        request: &Req,
        response: &Resp,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error>
    where
        Self: Sized;

    /// Retrieve the requested object from the proof.
    ///
    /// Runs full verification of the proof and retrieves enclosed objects.
    ///
    /// This method uses [`maybe_from_proof()`] internally and throws an error if the requested object does not exist in the proof.
    ///
    /// # Arguments
    ///
    /// * `request`: The request sent to the server.
    /// * `response`: The response received from the server.
    /// * `provider`: A callback implementing [QuorumInfoProvider] that provides quorum details required to verify the proof.
    ///
    /// # Returns
    ///
    /// * `Ok(object)` when the requested object was found in the proof.
    /// * `Err(Error::DocumentMissingInProof)` when the requested object was not found in the proof.
    /// * `Err(Error)` when either the provided data is invalid or proof validation failed.
    fn from_proof(
        request: &Req,
        response: &Resp,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::maybe_from_proof(request, response, provider)?.ok_or(Error::DocumentMissingInProof)
    }
}

/// `QuorumInfoProvider` trait provides an interface to fetch quorum related information, required to verify the proof.
///
/// Developers should implement this trait to provide required quorum details to [FromProof] implementations.
///
/// It defines a single method `get_quorum_public_key` which retrieves the public key of a given quorum.
#[uniffi::export(callback_interface)]
#[cfg_attr(feature = "mock", mockall::automock)]
pub trait QuorumInfoProvider: Send + Sync {
    /// Fetches the public key for a specified quorum.
    ///
    /// # Arguments
    ///
    /// * `quorum_type`: The type of the quorum.
    /// * `quorum_hash`: The hash of the quorum. This is used to determine which quorum's public key to fetch.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)`: On success, returns a byte vector representing the public key of the quorum.
    /// * `Err(Error)`: On failure, returns an error indicating why the operation failed.
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: Vec<u8>,
    ) -> Result<Vec<u8>, Error>;
}

#[cfg_attr(feature = "mock", mockall::automock)]
impl FromProof<platform::GetIdentityRequest, platform::GetIdentityResponse> for Identity {
    fn maybe_from_proof(
        request: &platform::GetIdentityRequest,
        response: &platform::GetIdentityResponse,
        provider: Box<dyn QuorumInfoProvider>,
    ) -> Result<Option<Self>, Error> {
        // Parse response to read proof and metadata
        let proof = match response.result.as_ref().ok_or(Error::EmptyResponse)? {
            platform::get_identity_response::Result::Proof(p) => p,
            platform::get_identity_response::Result::Identity(_) => {
                return Err(Error::EmptyResponseProof)
            }
        };

        let mtd = response
            .metadata
            .as_ref()
            .ok_or(Error::EmptyResponseMetadata)?;

        // Load some info from request
        let id = Identifier::from_bytes(&request.id).map_err(|e| Error::ProtocolError {
            error: e.to_string(),
        })?;

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_identity) = Drive::verify_full_identity_by_identity_id(
            &proof.grovedb_proof,
            false,
            id.into_buffer(),
        )
        .map_err(|e| Error::DriveError {
            error: e.to_string(),
        })?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok(maybe_identity)
    }
}

#[cfg(all(test, feature = "mock"))]
mod test {
    use base64::Engine;
    use dapi_grpc::platform::v0::{
        self as platform, GetIdentityRequest, GetIdentityResponse, Proof, ResponseMetadata,
    };
    use dpp::prelude::Identity;
    use tracing::Level;

    use super::{FromProof, MockQuorumInfoProvider};

    /// Given some test vectors dumped from a devnet, prove non-existence of identity with some hardcoded identifier
    #[test]
    fn identity_not_found() {
        tracing_subscriber::fmt::fmt()
            .pretty()
            .with_ansi(true)
            .with_max_level(Level::TRACE)
            .try_init()
            .ok();
        let b64 = base64::engine::general_purpose::STANDARD;

        let request = GetIdentityRequest {
            id: b64
                .decode("lCJoCnN5TKJdBflau+DETzZZBo/gjyYs9FI7BwIb9pY=")
                .unwrap(),
            prove: true,
        };

        let response =  GetIdentityResponse{
            metadata: Some(
                ResponseMetadata { height:23,
                        core_chain_locked_height:1553,
                        time_ms:1687871674372, // TODO: should be 2023-06-27T13:14:34.372422898Z but this requires nanos
                        protocol_version: 1,
                        chain_id:"dashmate_local_44".to_string() 
                }
            ),
            result: Some(platform::get_identity_response::Result::Proof(
                Proof{
                    grovedb_proof: b64.decode("Ab4CAdTMXXKfNBehMLGIDo1S/+VaobU2N+QpXBdr6owVPleeBAEgACQCASCfZaUKZ1lrdoYjPs0O9YKoVr4p94txqspbRYfy8tthmACACDCtOGi/yyGncjKmZqC9fqfxV9nCtsO4y2gn4l+9fxABYiG0IZHw7TDXzDUAXWi6z/hgr8lQzAN0j5wuMdJ6aqIRAq2LjwtLqRl3c1vXyNdJbjkiqqNzSo7D6lGlVpFqDdXnEAHDNjUTblAumsUkSxWiCnV+B1nOCpCCPNN/iT9qSVVtJgQBYAAlBAEgn2WlCmdZa3aGIz7NDvWCqFa+KfeLcarKW0WH8vLbYZgAAHOVB8mU4K79Rdese3jm2g58QKT2XPka+IX6tpGkQpcaEAFSECjhepg5Q7de0bwiqqL865Ld4kZl6n9ukB/BRckx7hERAa8CAUTBdOXhdoytGGHcrzmhQWCjPI1uGYsDFVn32FxViWYbAkdxoPOsy9DT6kvKbcEpP0FM3WLiGQK0vdhXK5wEEF/vEAEwUJLq51xHHt60BH2s6/VNgxByXCdvp8Y41qjUp6uU/QLwuqamIdLJqKS4xhWUvDFZLhZJdMVZt139Y8BWREOqLxAFIIHZyUBcdCWDoV4ys55RTjRRM/wkHP4yvL9cjAJcXEsCV3a5TQtfQOOBFCxdVzWbjsMF2loXqN9E9sOVsaxhps8REQUgn2WlCmdZa3aGIz7NDvWCqFa+KfeLcarKW0WH8vLbYZha1yeCyXQq8C1LzJb//PEJxztgvzhW251WU1y3eCO11hABaWJNMvhAlZ068ODAL/QQTIvB8Dva9NR3dKNkw3WRWxgRAq8CAc2Yxt2Dqgacp9xO/kQALsasiPkhvLegWwERnbyMFFyHAok3IKl7xC9vN+2O7dSXI5GBlSBWezZEvNGS1Xg1Gc0tEAHlmBOqdoRiNVSluckGRW3/5D8Z4qS0Jk5/6Rt1/KzIFgLqvap1+tURIGj7/7p+LwwMzPdvgXNDMd0nPuhv32x8SxAFIIHZyUBcdCWDoV4ys55RTjRRM/wkHP4yvL9cjAJcXEsCKNgzkJLLlg6FlPUrw1EvCg6b0NbBAVP75FVHF4aRetkREQUgn2WlCmdZa3aGIz7NDvWCqFa+KfeLcarKW0WH8vLbYZgo2DOQksuWDoWU9SvDUS8KDpvQ1sEBU/vkVUcXhpF62RAB8iWtldjew7fM6/uTR+w6kIM+AqWleXuHVZTy5BZaB5ER").unwrap(),
                    quorum_hash: b64.decode("DHiurOlg/svYjTZD6o9S89f293yvDEqvo/TMu0mLHh4=").unwrap(),
                    signature: b64.decode( "kcEDkS5mYSRecLYpOUm8Vb7CJLSHKQRBoVGb52VGlqjjygm+LS4Ddh8AMhcejoMfCfrxp/OfJZCsBAkzKbO8W/vcthvATMFADFDG4D2yQIIqSzoizTL2LZiTMuKD1T8E").unwrap(),
                    round:0,
                    quorum_type: 106,
                    block_id_hash:  b64.decode( "TVwes9dgmiBOwcWR48wwNOpuXXn1NRU/pV93ZfY5wVk=").unwrap(),
                }
            ))
        };

        let mut provider = MockQuorumInfoProvider::new();
        provider
            .expect_get_quorum_public_key()
            .returning(|_quorum_type,_quorum_hash| {
                Ok(hex::decode("83fe724d9658a1b3f10a2db285f6132ca5c8795c4bf36e139a4b873d29b101a666efdbe06f81a4ed19a363ef39569df9").unwrap())
            })
            .once();

        let identity = Identity::maybe_from_proof(&request, &response, Box::new(provider)).unwrap();
        assert!(identity.is_none())
    }
}
