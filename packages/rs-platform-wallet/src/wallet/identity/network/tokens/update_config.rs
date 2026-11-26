//! Token update-config — applies a single
//! `TokenConfigurationChangeItem` mutation to the token's on-contract
//! config. Group-gateable. The variant enum stays open-ended so this
//! entry point doesn't have to grow a new method per setting.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use dash_sdk::dapi_client::CanRetry;
use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
use dash_sdk::platform::tokens::transitions::ConfigUpdateResult;
use dash_sdk::platform::{ContextProvider, Fetch};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::{DataContract, DataContractWithSerialization, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::network::IdentityWallet;

/// How many times the contract is fetched after an applied configuration
/// update before giving up on a node that has not seen the update's block.
const CONFIG_UPDATE_REFRESH_ATTEMPTS: u32 = 3;

/// Wait between those fetches.
const CONFIG_UPDATE_REFRESH_DELAY: Duration = Duration::from_secs(2);

/// What a token configuration update did, with the contract it produced.
///
/// Platform applies the change to the contract and raises its version, but
/// the update's proof covers only the token history document (or the group
/// action), not the contract. So after an applied update the contract is
/// fetched again, refusing an answer from a block before the one that
/// confirmed the update.
#[derive(Debug, Clone)]
pub enum TokenConfigUpdateOutcome {
    /// A group action still waiting for signatures: the contract is unchanged.
    Pending,
    /// The update was applied; `contract` (with its platform serialization)
    /// is a proof-verified fetch answered at the update's confirmation height
    /// or later.
    Applied {
        contract: DataContract,
        serialization: Vec<u8>,
    },
    /// The update was applied, but no fetch returned the contract from the
    /// update's block or later. `minimum_version` is the lowest version that
    /// contains the update; `reason` says why.
    AppliedNotRefreshed {
        minimum_version: u32,
        reason: String,
    },
}

/// Whether the proven result of a configuration update means the change took
/// effect. A plain update always does; a group action does once its signing
/// power closes it, which is when Drive writes the history document.
fn config_update_applied(result: &ConfigUpdateResult) -> bool {
    match result {
        ConfigUpdateResult::Document(_) => true,
        ConfigUpdateResult::GroupActionWithDocument(_, document) => document.is_some(),
    }
}

/// Why a contract fetch after an applied update did not return the update.
#[derive(Debug)]
enum RefetchFailure {
    /// The node answered from a block before the update's, or returned no
    /// contract or an older version: it may not have the update's block yet,
    /// so another fetch can succeed.
    Lagging(String),
    /// A failure another fetch will not fix (a closed SDK, a malformed reply).
    Terminal(String),
}

/// Fetch through `fetch` until it answers from block `confirmed_height` or
/// later with a contract at `minimum_version` or later, trying `attempts`
/// times with `delay` between tries. Only a lagging answer or a retriable SDK
/// error is tried again.
///
/// The height is what makes the answer include the update: the update does
/// not bind the revision it applies to, so another edit of the same contract
/// in between can make a revision that meets `minimum_version` but predates
/// the update. The version check stays as a second guard.
async fn refetch_at_or_after<F, Fut>(
    confirmed_height: u64,
    minimum_version: u32,
    attempts: u32,
    delay: Duration,
    mut fetch: F,
) -> Result<DataContractWithSerialization, String>
where
    F: FnMut() -> Fut,
    Fut: Future<
        Output = Result<(Option<DataContractWithSerialization>, ResponseMetadata), dash_sdk::Error>,
    >,
{
    let mut last = String::from("no fetch was made");
    for attempt in 0..attempts {
        if attempt > 0 {
            tokio::time::sleep(delay).await;
        }
        let failure = match fetch().await {
            Ok((_, metadata)) if metadata.height < confirmed_height => {
                RefetchFailure::Lagging(format!(
                    "the node answered from block {}, before block {confirmed_height} that \
                     confirmed the update",
                    metadata.height
                ))
            }
            Ok((Some((contract, serialization)), _)) if contract.version() >= minimum_version => {
                return Ok((contract, serialization));
            }
            Ok((Some((contract, _)), _)) => RefetchFailure::Lagging(format!(
                "the node returned contract version {}, older than version {minimum_version}",
                contract.version()
            )),
            Ok((None, _)) => {
                RefetchFailure::Lagging("the node did not return the contract".to_string())
            }
            Err(e) if e.can_retry() => RefetchFailure::Lagging(e.to_string()),
            Err(e) => RefetchFailure::Terminal(e.to_string()),
        };
        match failure {
            RefetchFailure::Lagging(reason) => last = reason,
            RefetchFailure::Terminal(reason) => return Err(reason),
        }
    }
    Err(last)
}

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Update config with a caller-supplied signer + key.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_update_config_with_signer<S: Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        config_change: dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<ConfigUpdateResult, dash_sdk::Error> {
        self.token_update_config_with_metadata(
            data_contract,
            token_position,
            identity_id,
            config_change,
            signing_key,
            signer,
            public_note,
            group_info,
            options,
        )
        .await
        .map(|(result, _)| result)
    }

    /// [`token_update_config_with_signer`](Self::token_update_config_with_signer),
    /// also returning the metadata of the response that proved the result:
    /// its `height` is the block that confirmed the update.
    #[allow(clippy::too_many_arguments)]
    async fn token_update_config_with_metadata<S: Signer<IdentityPublicKey>>(
        &self,
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        identity_id: Identifier,
        config_change: dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem,
        signing_key: &IdentityPublicKey,
        signer: &S,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        options: Option<
            dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions,
        >,
    ) -> Result<(ConfigUpdateResult, ResponseMetadata), dash_sdk::Error> {
        use dash_sdk::platform::tokens::builders::config_update::TokenConfigUpdateTransitionBuilder;

        let mut builder = TokenConfigUpdateTransitionBuilder::new(
            data_contract,
            token_position,
            identity_id,
            config_change,
        );

        if let Some(note) = public_note {
            builder = builder.with_public_note(note);
        }
        if let Some(gi) = group_info {
            builder = builder.with_using_group_info(gi);
        }
        if let Some(opts) = options {
            builder = builder.with_state_transition_creation_options(opts);
        }

        self.sdk
            .token_update_contract_token_configuration_with_metadata(builder, signing_key, signer)
            .await
    }

    /// Update config with an external signer. Contract fetched
    /// internally; `config_change` is the full enum so this entry
    /// point stays open-ended for variants the FFI doesn't yet
    /// surface.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_update_config_with_external_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        config_change: dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<dash_sdk::platform::tokens::transitions::ConfigUpdateResult, PlatformWalletError>
    {
        let data_contract = self.token_fetch_data_contract(token_contract_id).await?;
        let signing_key = self.token_resolve_signing_key(&identity_id, signer).await?;

        self.token_update_config_with_signer(
            data_contract,
            token_position,
            identity_id,
            config_change,
            &signing_key,
            signer,
            public_note,
            group_info,
            None,
        )
        .await
        // Keeps the SDK error, so a consensus rejection reaches the FFI
        // boundary with its code instead of as rendered text.
        .map_err(|e| PlatformWalletError::token_operation_failed("config update", e))
    }

    /// Update config with an external signer, then report what the update
    /// did and, when it was applied, the contract it produced.
    ///
    /// After an applied update the contract is fetched again, refusing an
    /// answer from a block before the one that confirmed the update, and a
    /// version below the one fetched to build the update plus one
    /// ([`refetch_at_or_after`]). The refreshed contract is registered with
    /// the SDK's context provider, as every token write path does with the
    /// contract it fetches.
    #[allow(clippy::too_many_arguments)]
    pub async fn token_update_config_and_refresh_with_external_signer<
        S: Signer<IdentityPublicKey>,
    >(
        &self,
        identity_id: Identifier,
        token_contract_id: Identifier,
        token_position: TokenContractPosition,
        config_change: dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem,
        public_note: Option<String>,
        group_info: Option<dpp::group::GroupStateTransitionInfoStatus>,
        signer: &S,
    ) -> Result<TokenConfigUpdateOutcome, PlatformWalletError> {
        let data_contract = self.token_fetch_data_contract(token_contract_id).await?;
        let minimum_version = data_contract.version().saturating_add(1);
        let signing_key = self.token_resolve_signing_key(&identity_id, signer).await?;

        let (result, metadata) = self
            .token_update_config_with_metadata(
                data_contract,
                token_position,
                identity_id,
                config_change,
                &signing_key,
                signer,
                public_note,
                group_info,
                None,
            )
            .await
            .map_err(|e| PlatformWalletError::token_operation_failed("config update", e))?;

        if !config_update_applied(&result) {
            return Ok(TokenConfigUpdateOutcome::Pending);
        }

        let refreshed = refetch_at_or_after(
            metadata.height,
            minimum_version,
            CONFIG_UPDATE_REFRESH_ATTEMPTS,
            CONFIG_UPDATE_REFRESH_DELAY,
            || {
                DataContractWithSerialization::fetch_with_metadata(
                    &self.sdk,
                    token_contract_id,
                    None,
                )
            },
        )
        .await;
        Ok(match refreshed {
            Ok((contract, serialization)) => {
                if let Some(provider) = self.sdk.context_provider() {
                    provider.register_data_contract(Arc::new(contract.clone()));
                }
                TokenConfigUpdateOutcome::Applied {
                    contract,
                    serialization,
                }
            }
            Err(reason) => TokenConfigUpdateOutcome::AppliedNotRefreshed {
                minimum_version,
                reason,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::document::{Document, DocumentV0};
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn contract_at(version: u32) -> DataContractWithSerialization {
        let mut contract =
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
                .data_contract_owned();
        contract.set_version(version);
        (contract, vec![version as u8])
    }

    #[test]
    fn should_treat_a_plain_update_and_a_closed_group_action_as_applied() {
        let document = Document::V0(DocumentV0::default());
        assert!(config_update_applied(&ConfigUpdateResult::Document(
            document.clone()
        )));
        assert!(config_update_applied(
            &ConfigUpdateResult::GroupActionWithDocument(2, Some(document))
        ));
    }

    #[test]
    fn should_treat_an_open_group_action_as_pending() {
        assert!(!config_update_applied(
            &ConfigUpdateResult::GroupActionWithDocument(1, None)
        ));
    }

    fn at(height: u64) -> ResponseMetadata {
        ResponseMetadata {
            height,
            ..Default::default()
        }
    }

    #[tokio::test(start_paused = true)]
    async fn should_return_the_first_answer_from_the_confirming_block_or_later() {
        let calls = AtomicU32::new(0);
        let fetched = refetch_at_or_after(100, 3, 3, Duration::from_secs(2), || {
            let n = calls.fetch_add(1, Ordering::SeqCst);
            async move { Ok((Some(contract_at(3)), at(99 + u64::from(n)))) }
        })
        .await
        .expect("the second fetch answers from block 100");
        assert_eq!(fetched.0.version(), 3);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn should_refuse_a_newer_revision_proved_before_the_update() {
        // Another edit made revision 3 at block 99; the update made revision 4
        // at block 100. Revision 3 meets the version floor (fetched 2, plus
        // one) but predates the update, so only its block height gives it away.
        let calls = AtomicU32::new(0);
        let failure = refetch_at_or_after(100, 3, 3, Duration::from_secs(2), || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Ok((Some(contract_at(3)), at(99))) }
        })
        .await
        .unwrap_err();
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert!(failure.contains("before block 100"), "{failure}");
    }

    #[tokio::test(start_paused = true)]
    async fn should_refuse_an_older_version_even_from_a_later_block() {
        let failure = refetch_at_or_after(100, 3, 3, Duration::from_secs(2), || async {
            Ok((Some(contract_at(2)), at(101)))
        })
        .await
        .unwrap_err();
        assert!(failure.contains("older than version 3"), "{failure}");
    }

    #[tokio::test(start_paused = true)]
    async fn should_stop_at_once_on_an_error_another_fetch_cannot_fix() {
        let calls = AtomicU32::new(0);
        let failure = refetch_at_or_after(100, 3, 3, Duration::from_secs(2), || {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Err(dash_sdk::Error::Generic("the SDK is closed".to_string())) }
        })
        .await
        .unwrap_err();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(failure.contains("the SDK is closed"), "{failure}");
    }

    #[tokio::test(start_paused = true)]
    async fn should_try_again_after_a_retriable_error() {
        let calls = AtomicU32::new(0);
        let fetched = refetch_at_or_after(100, 3, 3, Duration::from_secs(2), || {
            let n = calls.fetch_add(1, Ordering::SeqCst);
            async move {
                if n == 0 {
                    Err(dash_sdk::Error::TimeoutReached(
                        Duration::from_secs(1),
                        "first try".to_string(),
                    ))
                } else {
                    Ok((Some(contract_at(3)), at(100)))
                }
            }
        })
        .await
        .expect("the retry succeeds");
        assert_eq!(fetched.0.version(), 3);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
