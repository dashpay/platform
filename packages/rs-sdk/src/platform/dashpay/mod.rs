//! DashPay contact request helpers
//!
//! This module provides helper functions for creating and sending DashPay contact requests
//! according to DIP-15 specification.

mod contact_request;

pub use contact_request::{
    ContactRequestInput, ContactRequestResult, EcdhProvider, RecipientIdentity,
    SendContactRequestInput, SendContactRequestResult,
};

use crate::platform::Fetch;
use crate::{Error, Sdk};
use dash_context_provider::ContextProvider;
use dpp::prelude::Identifier;
use std::sync::Arc;

impl Sdk {
    /// Helper method to get the DashPay contract ID
    fn get_dashpay_contract_id(&self) -> Result<Identifier, Error> {
        // Get DashPay contract ID from system contract if available
        #[cfg(feature = "dashpay-contract")]
        let dashpay_contract_id = {
            use dpp::system_data_contracts::SystemDataContract;
            SystemDataContract::Dashpay.id()
        };

        #[cfg(not(feature = "dashpay-contract"))]
        let dashpay_contract_id = {
            const DASHPAY_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
            Identifier::from_string(
                DASHPAY_CONTRACT_ID,
                dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| Error::Generic(format!("Invalid DashPay contract ID: {}", e)))?
        };

        Ok(dashpay_contract_id)
    }

    /// Helper method to fetch the DashPay contract, checking context provider first
    async fn fetch_dashpay_contract(&self) -> Result<Arc<dpp::data_contract::DataContract>, Error> {
        let dashpay_contract_id = self.get_dashpay_contract_id()?;

        // First check if the contract is available in the context provider
        let context_provider = self
            .context_provider()
            .ok_or_else(|| Error::Generic("Context provider not set".to_string()))?;

        match context_provider.get_data_contract(&dashpay_contract_id, self.version())? {
            Some(contract) => Ok(contract),
            None => {
                // If not in context, fetch from platform
                let contract = crate::platform::DataContract::fetch(self, dashpay_contract_id)
                    .await?
                    .ok_or_else(|| Error::Generic("DashPay contract not found".to_string()))?;
                Ok(Arc::new(contract))
            }
        }
    }
}
