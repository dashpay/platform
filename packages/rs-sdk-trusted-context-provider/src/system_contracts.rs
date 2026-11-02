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
use dpp::version::PlatformVersion;
use std::sync::Arc;

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
