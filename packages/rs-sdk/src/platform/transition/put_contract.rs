use std::collections::BTreeMap;

use crate::{Error, Sdk};

use crate::platform::transition::put_settings::PutSettings;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, PartialIdentity};
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::StateTransition;

use super::broadcast::{
    broadcast_with_observation, wait_after_broadcast, BroadcastPolicy, BroadcastStateTransition,
};
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;

#[async_trait::async_trait]
/// A trait for putting a contract to platform
pub trait PutContract<S: Signer<IdentityPublicKey>>: Waitable {
    /// Puts a contract on platform
    /// Setting settings to `None` sets default connection behavior
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        identity_public_key: IdentityPublicKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Puts a contract on platform and waits for the confirmation proof
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        identity_public_key: IdentityPublicKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<DataContract, Error>;
}

#[async_trait::async_trait]
impl<S: Signer<IdentityPublicKey>> PutContract<S> for DataContract {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        identity_public_key: IdentityPublicKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let transition =
            prepare_contract_state_transition(self, sdk, identity_public_key, signer, settings)
                .await?;
        transition.broadcast(sdk, settings).await?;
        // response is empty for a broadcast, result comes from the stream wait for state transition result

        Ok(transition)
    }

    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        identity_public_key: IdentityPublicKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<DataContract, Error> {
        let state_transition =
            prepare_contract_state_transition(self, sdk, identity_public_key, signer, settings)
                .await?;

        let broadcast =
            broadcast_with_observation(&state_transition, sdk, settings, BroadcastPolicy::Standard)
                .await;
        wait_after_broadcast(broadcast, async move {
            Self::wait_for_response(sdk, state_transition, settings).await
        })
        .await
    }
}

/// Construct and sign once; dispatch is chosen by the calling API.
async fn prepare_contract_state_transition<S: Signer<IdentityPublicKey>>(
    contract: &DataContract,
    sdk: &Sdk,
    identity_public_key: IdentityPublicKey,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    let new_identity_nonce = sdk
        .get_identity_nonce(contract.owner_id(), true, settings)
        .await?;

    let key_id = identity_public_key.id();

    let partial_identity = PartialIdentity {
        id: contract.owner_id(),
        loaded_public_keys: BTreeMap::from([(key_id, identity_public_key)]),
        balance: None,
        revision: None,
        not_found_public_keys: Default::default(),
    };
    let transition = DataContractCreateTransition::new_from_data_contract(
        contract.clone(),
        new_identity_nonce,
        &partial_identity,
        key_id,
        signer,
        sdk.version(),
        None,
    )
    .await?;
    ensure_valid_state_transition_structure(&transition, sdk.version())?;

    Ok(transition)
}

#[cfg(all(test, feature = "mocks"))]
mod submission_tests {
    use super::*;
    use crate::sdk::put_test_support::PutFixture;
    use dapi_grpc::tonic::Status;

    #[tokio::test]
    async fn should_wait_for_original_signed_transition_after_ambiguous_submission() {
        for (status, expected_waits) in [
            (Status::unavailable("acknowledgement lost"), 1),
            (Status::already_exists("tx already exists in cache"), 1),
            (
                Status::resource_exhausted("broadcasting state transition is timed out"),
                1,
            ),
            (Status::resource_exhausted("mempool is full"), 0),
        ] {
            let mut fixture = PutFixture::new().await;
            let prepared = prepare_contract_state_transition(
                &fixture.contract,
                &fixture.sdk,
                fixture.key.clone(),
                &fixture.signer,
                PutFixture::settings(),
            )
            .await
            .unwrap();
            let requests = fixture.expect_submission(&prepared, status).await;
            let result = fixture
                .contract
                .put_to_platform_and_wait_for_response(
                    &fixture.sdk,
                    fixture.key.clone(),
                    &fixture.signer,
                    PutFixture::settings(),
                )
                .await
                .map(|_| ());
            fixture
                .assert_calls(&requests, expected_waits, result)
                .await;
        }
    }

    #[tokio::test]
    async fn should_preserve_plain_broadcast_error_without_waiting() {
        let mut fixture = PutFixture::new().await;
        let prepared = prepare_contract_state_transition(
            &fixture.contract,
            &fixture.sdk,
            fixture.key.clone(),
            &fixture.signer,
            PutFixture::settings(),
        )
        .await
        .unwrap();
        let requests = fixture
            .expect_submission(&prepared, Status::unavailable("acknowledgement lost"))
            .await;
        let result = fixture
            .contract
            .put_to_platform(
                &fixture.sdk,
                fixture.key.clone(),
                &fixture.signer,
                PutFixture::settings(),
            )
            .await
            .map(|_| ());
        fixture.assert_calls(&requests, 0, result).await;
    }
}
