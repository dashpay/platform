//! A state-aware fee quote for a 0-input / 1-output address funding from a
//! fresh asset lock, served by the node's `getAddressFundingFeeQuote` query.
//!
//! The node prices the exact production operations with tree depths measured
//! from its committed state, adds the validation-operation fees execution
//! records, and applies the requested user fee increase. The response is a
//! computed value, not state — it carries no proof, so treat the quote as
//! planning data: sizing a funding lock stays governed by
//! `minimum_required_lock_credits` plus the wallet's own margin policy.

use crate::platform::proto;
use crate::platform::query::Query;
use crate::platform::QuerySettings;
use crate::{error::Error, Sdk};
use dapi_grpc::platform::v0::GetAddressFundingFeeQuoteRequest;
use dpp::address_funds::PlatformAddress;
use dpp::prelude::UserFeeIncrease;
use dpp::version::PlatformVersion;
pub use drive_proof_verifier::types::AddressFundingFeeQuote;
use rs_dapi_client::RequestSettings;

use crate::platform::FetchUnproved;

/// Parameters of an address funding fee quote.
#[derive(Debug, Clone)]
pub struct AddressFundingFeeQuoteQuery {
    /// The funding recipient.
    pub recipient: PlatformAddress,
    /// The exact planned asset lock outpoint (txid bytes followed by the
    /// vout as four little-endian bytes), when the wallet has already built
    /// and signed the lock transaction. `None` lets the node derive a
    /// deterministic placeholder — for a fresh (absent) outpoint both have
    /// the same expected search depth.
    pub asset_lock_outpoint: Option<[u8; 36]>,
    /// The user fee increase the quote should include; the SDK's chain-lock
    /// retry loop can raise a funding up to 14 units above the base.
    pub user_fee_increase: UserFeeIncrease,
    /// Length of the future transition's signable bytes when known; `None`
    /// uses the node's default. Clamped server-side, so it cannot understate
    /// the fee.
    pub signable_bytes_len_hint: Option<u32>,
}

impl Query<GetAddressFundingFeeQuoteRequest> for AddressFundingFeeQuoteQuery {
    fn query(
        &self,
        _settings: &QuerySettings<'_>,
    ) -> Result<GetAddressFundingFeeQuoteRequest, Error> {
        Ok(GetAddressFundingFeeQuoteRequest {
            version: Some(proto::get_address_funding_fee_quote_request::Version::V0(
                proto::get_address_funding_fee_quote_request::GetAddressFundingFeeQuoteRequestV0 {
                    address: self.recipient.to_bytes(),
                    asset_lock_outpoint: self
                        .asset_lock_outpoint
                        .map(|outpoint| outpoint.to_vec())
                        .unwrap_or_default(),
                    user_fee_increase: self.user_fee_increase as u32,
                    signable_bytes_len_hint: self.signable_bytes_len_hint.unwrap_or_default(),
                },
            )),
        })
    }
}

/// Fetches a state-aware address funding fee quote from the network.
///
/// Fails with a protocol error when the node quoted with a protocol version
/// this client does not know — a quote priced under unknown rules must not be
/// displayed as if it were understood.
pub async fn quote_address_funding_fee(
    sdk: &Sdk,
    query: AddressFundingFeeQuoteQuery,
) -> Result<AddressFundingFeeQuote, Error> {
    let (quote, _metadata) = AddressFundingFeeQuote::fetch_unproved_with_settings(
        sdk,
        query,
        RequestSettings::default(),
    )
    .await?;
    let quote = quote.ok_or_else(|| {
        Error::Generic("address funding fee quote response carried no data".to_string())
    })?;

    // Fail fast on a version this client doesn't know.
    PlatformVersion::get(quote.protocol_version).map_err(dpp::ProtocolError::from)?;

    Ok(quote)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_settings(request_settings: &RequestSettings) -> QuerySettings<'_> {
        QuerySettings {
            request_settings,
            protocol_version: PlatformVersion::latest(),
            prove: false,
        }
    }

    #[test]
    fn test_query_maps_placeholder_and_exact_outpoint() {
        let recipient = PlatformAddress::P2pkh([7; 20]);
        let request_settings = RequestSettings::default();
        let settings = test_settings(&request_settings);

        let placeholder = AddressFundingFeeQuoteQuery {
            recipient,
            asset_lock_outpoint: None,
            user_fee_increase: 3,
            signable_bytes_len_hint: None,
        };
        let request = placeholder.query(&settings).expect("query");
        let Some(proto::get_address_funding_fee_quote_request::Version::V0(v0)) = request.version
        else {
            panic!("expected V0 request");
        };
        assert_eq!(v0.address, recipient.to_bytes());
        assert!(v0.asset_lock_outpoint.is_empty(), "placeholder sends empty");
        assert_eq!(v0.user_fee_increase, 3);
        assert_eq!(v0.signable_bytes_len_hint, 0, "server default");

        let exact = AddressFundingFeeQuoteQuery {
            recipient,
            asset_lock_outpoint: Some([0xAB; 36]),
            user_fee_increase: 0,
            signable_bytes_len_hint: Some(390),
        };
        let request = exact.query(&settings).expect("query");
        let Some(proto::get_address_funding_fee_quote_request::Version::V0(v0)) = request.version
        else {
            panic!("expected V0 request");
        };
        assert_eq!(v0.asset_lock_outpoint, vec![0xAB; 36]);
        assert_eq!(v0.signable_bytes_len_hint, 390);
    }
}
