use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{types::TokenPreProgrammedDistributions, ContextProvider, Error};
use dapi_grpc::platform::v0::{
    get_token_pre_programmed_distributions_request, GetTokenPreProgrammedDistributionsRequest,
    GetTokenPreProgrammedDistributionsResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt;
use drive::drive::Drive;

use super::FromProof;

impl FromProof<GetTokenPreProgrammedDistributionsRequest> for TokenPreProgrammedDistributions {
    type Request = GetTokenPreProgrammedDistributionsRequest;
    type Response = GetTokenPreProgrammedDistributionsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let get_token_pre_programmed_distributions_request::Version::V0(req_v0) =
            request.version.ok_or(Error::EmptyVersion)?;

        let token_id: [u8; 32] =
            req_v0
                .token_id
                .as_slice()
                .try_into()
                .map_err(|_| Error::RequestError {
                    error: "token_id must be 32 bytes".into(),
                })?;

        let start_at = match req_v0.start_at_info {
            Some(start_at_info) => {
                let start_at_recipient = match start_at_info.start_recipient {
                    Some(recipient_bytes) => {
                        let recipient_id =
                            Identifier::from_bytes(&recipient_bytes).map_err(|_| {
                                Error::RequestError {
                                    error: "start_recipient must be 32 bytes".into(),
                                }
                            })?;
                        let included = start_at_info.start_recipient_included.unwrap_or(true);
                        Some((recipient_id, included))
                    }
                    None => None,
                };

                Some(QueryPreProgrammedDistributionStartAt {
                    start_at_time: start_at_info.start_time_ms,
                    start_at_recipient,
                })
            }
            None => None,
        };

        let limit = req_v0
            .limit
            .map(|l| {
                u16::try_from(l).map_err(|_| Error::RequestError {
                    error: "limit exceeds u16::MAX".into(),
                })
            })
            .transpose()?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();

        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, result): ([u8; 32], TokenPreProgrammedDistributions) =
            Drive::verify_token_pre_programmed_distributions(
                &proof.grovedb_proof,
                token_id,
                start_at,
                limit,
                false,
                platform_version,
            )
            .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        if result.0.is_empty() {
            Ok((None, metadata, proof))
        } else {
            Ok((Some(result), metadata, proof))
        }
    }
}
