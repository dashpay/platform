use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_token_direct_purchase_prices_request::GetTokenDirectPurchasePricesRequestV0;
use dapi_grpc::platform::v0::get_token_direct_purchase_prices_response::{
    get_token_direct_purchase_prices_response_v0::{
        self, token_direct_purchase_price_entry::Price, PriceForQuantity, PricingSchedule,
        TokenDirectPurchasePriceEntry,
    },
    GetTokenDirectPurchasePricesResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_token_direct_purchase_prices_v0(
        &self,
        GetTokenDirectPurchasePricesRequestV0 { token_ids, prove }: GetTokenDirectPurchasePricesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTokenDirectPurchasePricesResponseV0>, Error> {
        if token_ids.is_empty() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(
                    "please provide at least one token id in token_ids".to_string(),
                ),
            ));
        }

        let token_ids: Vec<[u8; 32]> = check_validation_result_with_data!(token_ids
            .into_iter()
            .map(|x| {
                x.try_into().map_err(|_| {
                    QueryError::InvalidArgument(
                        "token_ids must be a list of valid identifiers (32 bytes long)".to_string(),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>());

        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_tokens_direct_purchase_price(&token_ids, None, platform_version,));

            GetTokenDirectPurchasePricesResponseV0 {
                result: Some(get_token_direct_purchase_prices_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let token_prices: Vec<TokenDirectPurchasePriceEntry> = self
                .drive
                .fetch_tokens_direct_purchase_price(&token_ids, None, platform_version)?
                .into_iter()
                .map(|(token_id, schedule)| match schedule {
                    Some(TokenPricingSchedule::SinglePrice(price)) => {
                        TokenDirectPurchasePriceEntry {
                            token_id: token_id.to_vec(),
                            price: Some(Price::FixedPrice(price)),
                        }
                    }
                    Some(TokenPricingSchedule::SetPrices(prices)) => {
                        let price_for_quantity = prices
                            .into_iter()
                            .map(|(quantity, price)| PriceForQuantity { price, quantity })
                            .collect();

                        TokenDirectPurchasePriceEntry {
                            token_id: token_id.to_vec(),
                            price: Some(Price::VariablePrice(PricingSchedule {
                                price_for_quantity,
                            })),
                        }
                    }
                    None => TokenDirectPurchasePriceEntry {
                        token_id: token_id.to_vec(),
                        price: None,
                    },
                })
                .collect();

            GetTokenDirectPurchasePricesResponseV0 {
                result: Some(
                    get_token_direct_purchase_prices_response_v0::Result::TokenDirectPurchasePrices(
                        get_token_direct_purchase_prices_response_v0::TokenDirectPurchasePrices {
                            token_direct_purchase_price: token_prices,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
