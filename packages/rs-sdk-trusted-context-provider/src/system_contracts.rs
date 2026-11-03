//! System data contracts and activation heights a context provider serves
//! without asking any source.

use dash_context_provider::ContextProviderError;
use dpp::dashcore::Network;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
#[cfg(any(
    feature = "dpns-contract",
    feature = "dashpay-contract",
    feature = "withdrawals-contract",
    feature = "wallet-utils-contract",
    feature = "token-history-contract",
    feature = "keywords-contract",
    feature = "document-history-contract",
    feature = "app-connect-contract",
    feature = "moderation-charters-contract",
    feature = "decision-proposals-contract",
    feature = "token-market-contract",
    feature = "all-system-contracts"
))]
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
#[cfg(any(feature = "app-connect-contract", feature = "all-system-contracts"))]
use dpp::version::feature_initial_protocol_versions::APP_CONNECT_CONTRACT_INITIAL_PROTOCOL_VERSION;
#[cfg(any(
    feature = "decision-proposals-contract",
    feature = "all-system-contracts"
))]
use dpp::version::feature_initial_protocol_versions::DECISION_PROPOSALS_CONTRACT_INITIAL_PROTOCOL_VERSION;
#[cfg(any(
    feature = "moderation-charters-contract",
    feature = "all-system-contracts"
))]
use dpp::version::feature_initial_protocol_versions::MODERATION_CHARTERS_CONTRACT_INITIAL_PROTOCOL_VERSION;
use dpp::version::feature_initial_protocol_versions::TOKEN_MARKETPLACE_INITIAL_PROTOCOL_VERSION;
use dpp::version::PlatformVersion;
use std::sync::Arc;

/// Whether `id` names a system data contract that does not exist yet at
/// `platform_version`: the token market contract before protocol version 15. A
/// provider serves no contract under such an id, neither one preloaded into its
/// known contracts nor one its build loads, whichever contract features it has, so
/// each provider asks this before its known contracts.
pub(crate) fn is_inactive_system_contract(
    id: &Identifier,
    platform_version: &PlatformVersion,
) -> bool {
    *id == data_contracts::SystemDataContract::TokenMarket.id()
        && platform_version.protocol_version < TOKEN_MARKETPLACE_INITIAL_PROTOCOL_VERSION
}

/// The system data contract with this id, if its cargo feature is enabled and
/// it exists at `platform_version`; `Ok(None)` otherwise.
#[cfg_attr(
    not(any(
        feature = "dpns-contract",
        feature = "dashpay-contract",
        feature = "withdrawals-contract",
        feature = "wallet-utils-contract",
        feature = "token-history-contract",
        feature = "keywords-contract",
        feature = "document-history-contract",
        feature = "app-connect-contract",
        feature = "moderation-charters-contract",
        feature = "decision-proposals-contract",
        feature = "token-market-contract",
        feature = "all-system-contracts"
    )),
    allow(unused_variables)
)]
pub(crate) fn system_data_contract(
    id: &Identifier,
    platform_version: &PlatformVersion,
) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
    // Check if this is a system data contract and the corresponding feature is enabled
    #[cfg(any(
        feature = "dpns-contract",
        feature = "dashpay-contract",
        feature = "withdrawals-contract",
        feature = "wallet-utils-contract",
        feature = "token-history-contract",
        feature = "keywords-contract",
        feature = "document-history-contract",
        feature = "app-connect-contract",
        feature = "moderation-charters-contract",
        feature = "decision-proposals-contract",
        feature = "token-market-contract",
        feature = "all-system-contracts"
    ))]
    {
        // Check each system contract if its feature is enabled
        #[cfg(any(feature = "dpns-contract", feature = "all-system-contracts"))]
        if *id == SystemDataContract::DPNS.id() {
            return load_system_data_contract(SystemDataContract::DPNS, platform_version)
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!("Failed to load DPNS contract: {}", e))
                });
        }

        #[cfg(any(feature = "dashpay-contract", feature = "all-system-contracts"))]
        if *id == SystemDataContract::Dashpay.id() {
            return load_system_data_contract(SystemDataContract::Dashpay, platform_version)
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!("Failed to load Dashpay contract: {}", e))
                });
        }

        #[cfg(any(feature = "withdrawals-contract", feature = "all-system-contracts"))]
        if *id == SystemDataContract::Withdrawals.id() {
            return load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load Withdrawals contract: {}",
                        e
                    ))
                });
        }

        #[cfg(any(feature = "wallet-utils-contract", feature = "all-system-contracts"))]
        if *id == SystemDataContract::WalletUtils.id() {
            return load_system_data_contract(SystemDataContract::WalletUtils, platform_version)
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load WalletUtils contract: {}",
                        e
                    ))
                });
        }

        #[cfg(any(feature = "token-history-contract", feature = "all-system-contracts"))]
        if *id == SystemDataContract::TokenHistory.id() {
            return load_system_data_contract(SystemDataContract::TokenHistory, platform_version)
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load TokenHistory contract: {}",
                        e
                    ))
                });
        }

        #[cfg(any(feature = "keywords-contract", feature = "all-system-contracts"))]
        if *id == SystemDataContract::KeywordSearch.id() {
            return load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load KeywordSearch contract: {}",
                        e
                    ))
                });
        }

        #[cfg(any(
            feature = "document-history-contract",
            feature = "all-system-contracts"
        ))]
        if *id == SystemDataContract::DocumentHistory.id() {
            return load_system_data_contract(
                SystemDataContract::DocumentHistory,
                platform_version,
            )
            .map(|contract| Some(Arc::new(contract)))
            .map_err(|e| {
                ContextProviderError::Generic(format!(
                    "Failed to load DocumentHistory contract: {}",
                    e
                ))
            });
        }

        #[cfg(any(feature = "app-connect-contract", feature = "all-system-contracts"))]
        // Below protocol version 14 the app-connect contract is
        // absent, so the lookup falls through to the fallback provider (or `None`).
        if *id == SystemDataContract::AppConnect.id()
            && platform_version.protocol_version >= APP_CONNECT_CONTRACT_INITIAL_PROTOCOL_VERSION
        {
            return load_system_data_contract(SystemDataContract::AppConnect, platform_version)
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load AppConnect contract: {}",
                        e
                    ))
                });
        }

        #[cfg(any(
            feature = "moderation-charters-contract",
            feature = "all-system-contracts"
        ))]
        // Below protocol version 14 the moderation charters contract is absent too.
        if *id == SystemDataContract::ModerationCharters.id()
            && platform_version.protocol_version
                >= MODERATION_CHARTERS_CONTRACT_INITIAL_PROTOCOL_VERSION
        {
            return load_system_data_contract(
                SystemDataContract::ModerationCharters,
                platform_version,
            )
            .map(|contract| Some(Arc::new(contract)))
            .map_err(|e| {
                ContextProviderError::Generic(format!(
                    "Failed to load ModerationCharters contract: {}",
                    e
                ))
            });
        }

        #[cfg(any(
            feature = "decision-proposals-contract",
            feature = "all-system-contracts"
        ))]
        // Below protocol version 15 the decision proposals contract is absent.
        if *id == SystemDataContract::DecisionProposals.id()
            && platform_version.protocol_version
                >= DECISION_PROPOSALS_CONTRACT_INITIAL_PROTOCOL_VERSION
        {
            return load_system_data_contract(
                SystemDataContract::DecisionProposals,
                platform_version,
            )
            .map(|contract| Some(Arc::new(contract)))
            .map_err(|e| {
                ContextProviderError::Generic(format!(
                    "Failed to load DecisionProposals contract: {}",
                    e
                ))
            });
        }

        #[cfg(any(feature = "token-market-contract", feature = "all-system-contracts"))]
        // Below protocol version 15 the token market contract is absent
        // (`is_inactive_system_contract`).
        if *id == SystemDataContract::TokenMarket.id()
            && !is_inactive_system_contract(id, platform_version)
        {
            return load_system_data_contract(SystemDataContract::TokenMarket, platform_version)
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load TokenMarket contract: {}",
                        e
                    ))
                });
        }
    }

    Ok(None)
}

/// The Core height Platform activated at (the L1 locked height) on `network`.
pub(crate) fn platform_activation_height(network: Network) -> CoreBlockHeight {
    match network {
        Network::Mainnet => 2132092,             // Mainnet L1 locked height
        Network::Testnet => 1090319,             // Testnet L1 locked height
        Network::Devnet | Network::Regtest => 1, // Devnet/Regtest activation height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shared loader serves the token market contract only from protocol version 15,
    /// whatever its caller checked: before, none, without an error.
    #[cfg(any(feature = "token-market-contract", feature = "all-system-contracts"))]
    #[test]
    fn should_load_no_token_market_contract_before_protocol_15() {
        let id = data_contracts::SystemDataContract::TokenMarket.id();
        assert!(
            system_data_contract(&id, PlatformVersion::get(14).expect("protocol version 14"))
                .expect("a pre-activation load does not error")
                .is_none()
        );
        assert!(
            system_data_contract(&id, PlatformVersion::get(15).expect("protocol version 15"))
                .expect("the load succeeds at protocol version 15")
                .is_some()
        );
    }

    #[test]
    fn should_mark_only_the_token_market_inactive_before_protocol_15() {
        let pv14 = PlatformVersion::get(14).expect("protocol version 14");
        let pv15 = PlatformVersion::get(15).expect("protocol version 15");
        let token_market = data_contracts::SystemDataContract::TokenMarket.id();
        assert!(is_inactive_system_contract(&token_market, pv14));
        assert!(!is_inactive_system_contract(&token_market, pv15));
        assert!(!is_inactive_system_contract(
            &data_contracts::SystemDataContract::DPNS.id(),
            pv14
        ));
    }
}
