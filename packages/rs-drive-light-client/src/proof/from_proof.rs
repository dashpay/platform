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
                ResponseMetadata { height:189,
                        core_chain_locked_height:1617,
                        time_ms:1688035046883, // TODO: should be 2023-06-27T13:14:34.372422898Z but this requires nanos
                        protocol_version: 1,
                        chain_id:"dashmate_local_5".to_string() // TODO: chain id must be read from tenderdash genesis.json
                }
            ),
            result: Some(platform::get_identity_response::Result::Proof(
                Proof{
                    grovedb_proof: b64.decode("Ab4CAexDylk+WuD86iRYb6bx230lXLoFtSqLCeSw/nOfXJbzBAEgACQCASCfZaUKZ1lrdoYjPs0O9YKoVr4p94txqspbRYfy8tthmABZxibcZD2C32GaVZgNvQPxYBU/KRFKrLVkOU8XFjqhyRAB4mgui2Q7VyjYiyGOhQEHJkboQALRbgk27TBWwAEwq1MRAq2LjwtLqRl3c1vXyNdJbjkiqqNzSo7D6lGlVpFqDdXnEAHDNjUTblAumsUkSxWiCnV+B1nOCpCCPNN/iT9qSVVtJgQBYAAlBAEgn2WlCmdZa3aGIz7NDvWCqFa+KfeLcarKW0WH8vLbYZgAAL+cHKphVe39PALl2CUHnzcEgIHnmPGQ365s/gLamwDhEAHNvOJim418TBEmZxdVX05Gz061JolAhxPmeH9hMx4KjRERAY0CASXW3HRMrb0+AAqVCkdJD+RsZae6sGW1r/vCDyFsv+5KAmi22C3rnUSnLDn+c9CSt04QOkpxKL1/YZ5GUiy6PzzsEAGdUhPjzqZgpGOWjAJP/znIzTn7qKPgk1a6LyqBq1MKtgUgbrmteWWfYvdMrBlDy3wFecWT6loLtbebBT32T7twCL8z4hMW3QlC0VXIs32IH74GchFZrO/qfURj+X+VvzGh9BARBSCfZaUKZ1lrdoYjPs0O9YKoVr4p94txqspbRYfy8tthmPpy14uS/Mz7XiFgFNWyiV1sK/ax7OyTRyP6Za2XPVYQEAEPxjC56LCfmjtCqSmPNZCiSrA5tHTV314El3etV8ASwBECjQIBMEug99o1aL1r1fTvjQWXnN5x05C0e5L6l7qn+ahazyECiTcgqXvEL2837Y7t1JcjkYGVIFZ7NkS80ZLVeDUZzS0QAUPVbm2zx+3HgQdPsmd+RaQ771V7S+7SeKoYqjyqLSb7BSBuua15ZZ9i90ysGUPLfAV5xZPqWgu1t5sFPfZPu3AIvyjYM5CSy5YOhZT1K8NRLwoOm9DWwQFT++RVRxeGkXrZEBEFIJ9lpQpnWWt2hiM+zQ71gqhWvin3i3GqyltFh/Ly22GYKNgzkJLLlg6FlPUrw1EvCg6b0NbBAVP75FVHF4aRetkQASjSJd0LlYIYG6WOaQQ7lr9v8gvVTprLZhKFGxh5uMwiEQ==").unwrap(),
                    quorum_hash: b64.decode("Wpwae/E+1U3EEcalbAVFohB//qOaDd+xw8ptDamXoi0=").unwrap(),
                    signature: b64.decode("gdgljA8wRS/BQn1IzI2fz4rBgAErLxsLdN3/0kBuYcf4wk9FpCSqS+3TBXriSs1cFChyFgChivdEhWbHUM0liwV6kktGGTWLvySDwdwxDrei4xwEzoxuvOA2tuikUoHj").unwrap(),
                    round:0,
                    quorum_type: 106,
                    block_id_hash:  b64.decode("iI1GM0cAhHtJBu+uv1EkhCCqLd9ZIiQWiLa8dUHhuxg=").unwrap(),
                }
            ))
        };

        let mut provider = MockQuorumInfoProvider::new();
        provider
            .expect_get_quorum_public_key()
            .returning(|_quorum_type,_quorum_hash| {
                Ok(hex::decode("b69aaf2a341960b0c6f0f94ce24170be898f0a64cf51dfdca51464bac1af66fa69df2f533294e3fb1bcb9b72edd97ef9").unwrap())
            })
            .once();

        let identity = Identity::maybe_from_proof(&request, &response, Box::new(provider)).unwrap();
        assert!(identity.is_none())
    }
}
