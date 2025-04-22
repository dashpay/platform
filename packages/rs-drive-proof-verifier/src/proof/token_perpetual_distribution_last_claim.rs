use dapi_grpc::platform::{
    v0::{
        get_token_perpetual_distribution_last_claim_request::Version as RequestVersion,
        get_token_perpetual_distribution_last_claim_response::{
            get_token_perpetual_distribution_last_claim_response_v0::{
                self, last_claim_info::PaidAt,
            },
            Version as ResponseVersion,
        },
        GetTokenPerpetualDistributionLastClaimResponse, Proof, ResponseMetadata,
    },
    VersionedGrpcResponse,
};
use dpp::{
    dashcore::Network,
    data_contract::{
        accessors::v1::DataContractV1Getters,
        associated_token::{
            token_configuration::accessors::v0::TokenConfigurationV0Getters,
            token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters,
            token_perpetual_distribution::{
                methods::v0::TokenPerpetualDistributionV0Accessors,
                reward_distribution_moment::RewardDistributionMoment,
            },
        },
    },
    prelude::Identifier,
    version::PlatformVersion,
};
use drive::drive::Drive;
use get_token_perpetual_distribution_last_claim_response_v0::Result as RespResult;

use crate::{
    types::TokenPerpetualDistributionLastClaim, verify::verify_tenderdash_proof, ContextProvider,
    Error,
};

use super::FromProof;
use dapi_grpc::platform::v0::GetTokenPerpetualDistributionLastClaimRequest;

/// Parse & (optionally) verify the last‑claim proof returned by Platform.
impl FromProof<GetTokenPerpetualDistributionLastClaimRequest>
    for TokenPerpetualDistributionLastClaim
{
    type Request = GetTokenPerpetualDistributionLastClaimRequest;
    type Response = GetTokenPerpetualDistributionLastClaimResponse;

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

        let (resp_v0, mtd) = match response.version {
            Some(ResponseVersion::V0(ref v0)) => {
                let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;
                (v0, mtd.clone())
            }
            _ => return Err(Error::EmptyVersion),
        };

        let (token_id, identity_id, distribution_type) = match request
            .version
            .ok_or(Error::EmptyVersion)?
        {
            RequestVersion::V0(req_v0) => {
                let tok = <[u8; 32]>::try_from(req_v0.token_id.as_slice()).map_err(|_| {
                    Error::RequestError {
                        error: "token_id must be 32 bytes".into(),
                    }
                })?;
                let ide = <[u8; 32]>::try_from(req_v0.identity_id.as_slice()).map_err(|_| {
                    Error::RequestError {
                        error: "identity_id must be 32 bytes".into(),
                    }
                })?;

                // If the caller provided ContractTokenInfo we can look up the
                // concrete RewardDistributionType (needed to verify a Grove proof)
                let dist_type = if let Some(cti) = req_v0.contract_info {
                    let contract_id =
                        Identifier::from_vec(cti.contract_id).map_err(|e| Error::RequestError {
                            error: format!("invalid contract_id: {e}"),
                        })?;

                    let contract = provider
                        .get_data_contract(&contract_id, platform_version)
                        .map_err(|e| Error::RequestError {
                            error: format!("failed to fetch contract: {e}"),
                        })?
                        .ok_or_else(|| Error::RequestError {
                            error: "contract not found".into(),
                        })?;

                    let pos = cti.token_contract_position as u16;
                    let token = contract
                        .tokens()
                        .get(&pos)
                        .ok_or_else(|| Error::RequestError {
                            error: "token not found in contract".into(),
                        })?;

                    let rules = token.distribution_rules();
                    let Some(perp_rules) = rules.perpetual_distribution() else {
                        return Err(Error::RequestError {
                            error: "token has no perpetual distribution rules".into(),
                        });
                    };

                    Some(perp_rules.distribution_type().clone())
                } else {
                    None
                };

                (tok, ide, dist_type)
            }
        };

        let result = resp_v0.result.clone().ok_or(Error::NoProofInResult)?;

        // Server returned a proof
        if let RespResult::Proof(ref proof_msg) = result {
            // If caller didn’t pass ContractTokenInfo we cannot decode / verify the proof
            let Some(dist_type) = distribution_type else {
                return Err(Error::RequestError {
                    error: "cannot verify GroveDB proof without contract_info / distribution_type"
                        .into(),
                });
            };

            // Verify the GroveDB proof => reward‑moment
            let (root, moment_opt) = Drive::verify_token_perpetual_distribution_last_paid_time(
                &proof_msg.grovedb_proof,
                token_id,
                identity_id,
                &dist_type,
                false,
                platform_version,
            )?;

            // Verify the Tendermint header / quorum sig that wraps the Grove proof
            verify_tenderdash_proof(&proof_msg, &mtd, &root, provider)?;

            // Item is absent => the identity has never claimed => return None
            let Some(moment) = moment_opt else {
                return Ok((None, mtd, proof_msg.clone()));
            };

            // Convert Drive’s internal enum into the public SDK enum
            let claim = match moment {
                RewardDistributionMoment::BlockBasedMoment(h) => {
                    TokenPerpetualDistributionLastClaim::BlockHeight(h)
                }
                RewardDistributionMoment::TimeBasedMoment(ms) => {
                    TokenPerpetualDistributionLastClaim::TimestampMs(ms)
                }
                RewardDistributionMoment::EpochBasedMoment(e) => {
                    TokenPerpetualDistributionLastClaim::Epoch(e as u32)
                }
            };

            return Ok((Some(claim), mtd, proof_msg.clone()));
        }

        // Server returned a direct value
        if let RespResult::LastClaim(last) = result {
            // This path is hit when the request had `prove = false`
            let paid_at = last
                .paid_at
                .ok_or(Error::NoProofInResult /* better err name? */)?;

            let claim = match paid_at {
                PaidAt::TimestampMs(ms) => TokenPerpetualDistributionLastClaim::TimestampMs(ms),
                PaidAt::BlockHeight(h) => TokenPerpetualDistributionLastClaim::BlockHeight(h),
                PaidAt::Epoch(e) => TokenPerpetualDistributionLastClaim::Epoch(e),
                PaidAt::RawBytes(bytes) => TokenPerpetualDistributionLastClaim::Raw(bytes),
            };

            // There ISN’T any proof in this branch, return an empty one
            return Ok((Some(claim), mtd, Proof::default()));
        }

        unreachable!("`result` covers all variants; qed")
    }
}
