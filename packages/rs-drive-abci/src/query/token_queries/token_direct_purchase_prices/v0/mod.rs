use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
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
use drive::util::grove_operations::GroveDBToUse;

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
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
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
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use crate::query::tests::setup_platform_with_token_state;
    use dapi_grpc::platform::v0::get_token_direct_purchase_prices_response::get_token_direct_purchase_prices_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_empty_token_ids() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenDirectPurchasePricesRequestV0 {
            token_ids: vec![],
            prove: false,
        };

        let result = platform
            .query_token_direct_purchase_prices_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("at least one token id")
        ));
    }

    #[test]
    fn test_invalid_token_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenDirectPurchasePricesRequestV0 {
            token_ids: vec![vec![0; 8]],
            prove: false,
        };

        let result = platform
            .query_token_direct_purchase_prices_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_ids")
        ));
    }

    #[test]
    fn test_query_with_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenDirectPurchasePricesRequestV0 {
            token_ids: vec![vec![0; 32]],
            prove: false,
        };

        let result = platform
            .query_token_direct_purchase_prices_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(
                get_token_direct_purchase_prices_response_v0::Result::TokenDirectPurchasePrices(
                    prices,
                ),
            ) => {
                assert_eq!(prices.token_direct_purchase_price.len(), 1);
                // Price should be None for nonexistent token
                assert!(prices.token_direct_purchase_price[0].price.is_none());
            }
            _ => panic!("expected TokenDirectPurchasePrices result"),
        }
    }

    #[test]
    fn test_query_with_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenDirectPurchasePricesRequestV0 {
            token_ids: vec![vec![0; 32]],
            prove: true,
        };

        let result = platform
            .query_token_direct_purchase_prices_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenDirectPurchasePricesResponseV0 {
                result: Some(get_token_direct_purchase_prices_response_v0::Result::Proof(
                    _
                )),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_with_fixed_price() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        // Token 1 has a fixed price of 25
        let request = GetTokenDirectPurchasePricesRequestV0 {
            token_ids: vec![token_ids[1].to_vec()],
            prove: false,
        };

        let result = platform
            .query_token_direct_purchase_prices_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(
                get_token_direct_purchase_prices_response_v0::Result::TokenDirectPurchasePrices(
                    prices,
                ),
            ) => {
                assert_eq!(prices.token_direct_purchase_price.len(), 1);
                match &prices.token_direct_purchase_price[0].price {
                    Some(Price::FixedPrice(price)) => assert_eq!(*price, 25),
                    other => panic!("expected FixedPrice, got {:?}", other),
                }
            }
            _ => panic!("expected TokenDirectPurchasePrices result"),
        }
    }

    #[test]
    fn test_query_with_variable_price() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        // Token 2 has a variable pricing schedule
        let request = GetTokenDirectPurchasePricesRequestV0 {
            token_ids: vec![token_ids[2].to_vec()],
            prove: false,
        };

        let result = platform
            .query_token_direct_purchase_prices_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(
                get_token_direct_purchase_prices_response_v0::Result::TokenDirectPurchasePrices(
                    prices,
                ),
            ) => {
                assert_eq!(prices.token_direct_purchase_price.len(), 1);
                match &prices.token_direct_purchase_price[0].price {
                    Some(Price::VariablePrice(schedule)) => {
                        assert!(!schedule.price_for_quantity.is_empty());
                    }
                    other => panic!("expected VariablePrice, got {:?}", other),
                }
            }
            _ => panic!("expected TokenDirectPurchasePrices result"),
        }
    }

    #[test]
    fn test_query_no_price_set() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        // Token 0 has no direct purchase price set
        let request = GetTokenDirectPurchasePricesRequestV0 {
            token_ids: vec![token_ids[0].to_vec()],
            prove: false,
        };

        let result = platform
            .query_token_direct_purchase_prices_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(
                get_token_direct_purchase_prices_response_v0::Result::TokenDirectPurchasePrices(
                    prices,
                ),
            ) => {
                assert_eq!(prices.token_direct_purchase_price.len(), 1);
                assert!(prices.token_direct_purchase_price[0].price.is_none());
            }
            _ => panic!("expected TokenDirectPurchasePrices result"),
        }
    }

    #[test]
    fn test_query_multiple_tokens() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenDirectPurchasePricesRequestV0 {
            token_ids: vec![
                token_ids[0].to_vec(),
                token_ids[1].to_vec(),
                token_ids[2].to_vec(),
            ],
            prove: false,
        };

        let result = platform
            .query_token_direct_purchase_prices_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(
                get_token_direct_purchase_prices_response_v0::Result::TokenDirectPurchasePrices(
                    prices,
                ),
            ) => {
                assert_eq!(prices.token_direct_purchase_price.len(), 3);
            }
            _ => panic!("expected TokenDirectPurchasePrices result"),
        }
    }

    #[test]
    fn test_query_with_data_proof() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenDirectPurchasePricesRequestV0 {
            token_ids: vec![token_ids[0].to_vec()],
            prove: true,
        };

        let result = platform
            .query_token_direct_purchase_prices_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenDirectPurchasePricesResponseV0 {
                result: Some(get_token_direct_purchase_prices_response_v0::Result::Proof(
                    _
                )),
                metadata: Some(_),
            })
        ));
    }
}
