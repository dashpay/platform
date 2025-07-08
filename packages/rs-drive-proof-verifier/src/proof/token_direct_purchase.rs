use std::collections::BTreeSet;

use dapi_grpc::platform::{
    v0::{
        get_token_direct_purchase_prices_request::Version as RequestVersion,
        GetTokenDirectPurchasePricesRequest, GetTokenDirectPurchasePricesResponse, Proof,
        ResponseMetadata,
    },
    VersionedGrpcResponse,
};
use dpp::{
    dashcore::{secp256k1::hashes::hex::DisplayHex, Network},
    version::PlatformVersion,
};
use drive::drive::Drive;

use crate::{
    error::MapGroveDbError, types::TokenDirectPurchasePrices, verify::verify_tenderdash_proof,
    ContextProvider, Error,
};

use super::FromProof;

impl FromProof<GetTokenDirectPurchasePricesRequest> for TokenDirectPurchasePrices {
    type Request = GetTokenDirectPurchasePricesRequest;
    type Response = GetTokenDirectPurchasePricesResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;

        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let token_ids = match request.version.ok_or(Error::EmptyVersion)? {
            RequestVersion::V0(v0) => v0.token_ids,
        }
        .into_iter()
        .map(<[u8; 32]>::try_from)
        .collect::<Result<BTreeSet<_>, _>>() // BTreeSet to make it unique
        .map_err(|e| Error::RequestError {
            error: format!("token id {} has invalid length", e.to_lower_hex_string()),
        })?
        .into_iter()
        .collect::<Vec<_>>();

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, tokens): (_, Self) = Drive::verify_token_direct_selling_prices(
            &proof.grovedb_proof,
            &token_ids,
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        if tokens.is_empty() {
            return Ok((None, mtd.clone(), proof.clone()));
        }

        Ok((Some(tokens), mtd.clone(), proof.clone()))
    }
}
