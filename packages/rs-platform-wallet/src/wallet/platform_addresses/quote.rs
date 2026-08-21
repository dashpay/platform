//! State-aware funding fee quote — a thin wrapper over the SDK's
//! `getAddressFundingFeeQuote` call using the wallet's own SDK handle.

use super::wallet::PlatformAddressWallet;
use crate::error::PlatformWalletError;
use dash_sdk::platform::address_funding_fee_quote::{
    quote_address_funding_fee, AddressFundingFeeQuote, AddressFundingFeeQuoteQuery,
};
use dpp::address_funds::PlatformAddress;
use dpp::prelude::UserFeeIncrease;

impl PlatformAddressWallet {
    /// Fetches a state-aware fee quote for funding `recipient` with a fresh
    /// asset lock (0 address inputs, 1 remainder output).
    ///
    /// `prepared_outpoint` carries the exact outpoint when the wallet has
    /// already built and signed the lock transaction; `None` lets the node
    /// use a deterministic placeholder with the same expected search depth.
    ///
    /// The quote is planning data from a single node (no proof): sizing the
    /// lock stays governed by `minimum_required_lock_credits` plus the
    /// application's own margin policy. There is no offline fallback — a
    /// network failure surfaces as an error.
    pub async fn quote_funding_fee(
        &self,
        recipient: PlatformAddress,
        prepared_outpoint: Option<[u8; 36]>,
        user_fee_increase: UserFeeIncrease,
    ) -> Result<AddressFundingFeeQuote, PlatformWalletError> {
        Ok(quote_address_funding_fee(
            &self.sdk,
            AddressFundingFeeQuoteQuery {
                recipient,
                asset_lock_outpoint: prepared_outpoint,
                user_fee_increase,
                signable_bytes_len_hint: None,
            },
        )
        .await?)
    }
}
