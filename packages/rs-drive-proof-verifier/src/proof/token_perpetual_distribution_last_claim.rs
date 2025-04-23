use dapi_grpc::platform::v0::{
    get_token_perpetual_distribution_last_claim_request::Version as RequestVersion,
    get_token_perpetual_distribution_last_claim_response::{
        get_token_perpetual_distribution_last_claim_response_v0, Version as ResponseVersion,
    },
    GetTokenPerpetualDistributionLastClaimResponse, Proof, ResponseMetadata,
};
use dpp::{
    dashcore::Network,
    data_contract::associated_token::{
        token_configuration::accessors::v0::TokenConfigurationV0Getters,
        token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters,
        token_perpetual_distribution::{
            methods::v0::TokenPerpetualDistributionV0Accessors,
            reward_distribution_moment::RewardDistributionMoment,
        },
    },
    prelude::Identifier,
    version::PlatformVersion,
};
use drive::drive::Drive;
use get_token_perpetual_distribution_last_claim_response_v0::Result as RespResult;

use crate::{verify::verify_tenderdash_proof, ContextProvider, Error};

use super::FromProof;
use dapi_grpc::platform::v0::GetTokenPerpetualDistributionLastClaimRequest;

impl FromProof<GetTokenPerpetualDistributionLastClaimRequest> for RewardDistributionMoment {
    type Request = GetTokenPerpetualDistributionLastClaimRequest;
    type Response = GetTokenPerpetualDistributionLastClaimResponse;

    /// Parse & verify the last‑claim proof returned by Platform.
    ///
    /// Case 1: Proof in the response & ContextProvider returns a valid token distribution type via `get_token_configuration`,
    /// we then try to verify the proof and return a `RewardDistributionMoment` with the metadata and proof.
    ///
    /// Case 2: Proof in the response & ContextProvider returns None via `get_token_configuration`,
    /// we return None with the metadata and proof.
    ///
    /// Case 3: No proof in the response, we return the response's LastClaim (not supported for now).
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
        let request = request.into();
        let response = response.into();

        let RequestVersion::V0(req_v0) = request.version.ok_or(Error::EmptyVersion)?;

        // Parse required fields from request
        let token_id: [u8; 32] =
            req_v0
                .token_id
                .as_slice()
                .try_into()
                .map_err(|_| Error::RequestError {
                    error: "token_id must be 32 bytes".into(),
                })?;

        let identity_id: [u8; 32] =
            req_v0
                .identity_id
                .as_slice()
                .try_into()
                .map_err(|_| Error::RequestError {
                    error: "identity_id must be 32 bytes".into(),
                })?;

        // Extract and validate response version
        let ResponseVersion::V0(resp_v0) = response.version.ok_or(Error::EmptyVersion)?;

        let metadata = resp_v0
            .metadata
            .clone()
            .ok_or(Error::EmptyResponseMetadata)?;

        // Extract the result
        let result = resp_v0.result.clone().ok_or(Error::NoProofInResult)?;

        match result {
            RespResult::Proof(proof_msg) => {
                // --- Case 1 or 2: proof returned => attempt to verify ---
                let maybe_distribution_type = {
                    let token_id_identifier = Identifier::from_vec(req_v0.token_id.clone())
                        .map_err(|_| Error::RequestError {
                            error: "token_id must be 32 bytes".into(),
                        })?;

                    let maybe_token_config =
                        provider.get_token_configuration(&token_id_identifier)?;
                    let maybe_dist_type = maybe_token_config
                        .as_ref()
                        .and_then(|cfg| cfg.distribution_rules().perpetual_distribution())
                        .map(|perp| perp.distribution_type().clone());

                    maybe_dist_type
                };

                match maybe_distribution_type {
                    Some(distribution_type) => {
                        // --- Case 1: we can verify ---
                        let (root_hash, moment_opt) =
                            Drive::verify_token_perpetual_distribution_last_paid_time(
                                &proof_msg.grovedb_proof,
                                token_id,
                                identity_id,
                                &distribution_type,
                                false,
                                platform_version,
                            )?;

                        verify_tenderdash_proof(&proof_msg, &metadata, &root_hash, provider)?;

                        // May be None if identity has not yet claimed
                        Ok((moment_opt, metadata, proof_msg))
                    }
                    None => {
                        // --- Case 2: no distribution type => return None ---
                        Ok((None, metadata, proof_msg))
                    }
                }
            }

            RespResult::LastClaim(_) => {
                // --- Case 3: LastClaim returned => Not supported for now ---
                Err(Error::RequestError {
                    error: "Non-proof LastClaim response is not supported".into(),
                })
            }
        }
    }
}
