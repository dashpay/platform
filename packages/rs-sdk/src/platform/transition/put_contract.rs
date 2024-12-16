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

use super::broadcast::BroadcastStateTransition;
use super::waitable::Waitable;

#[async_trait::async_trait]
/// A trait for putting a contract to platform
pub trait PutContract<S: Signer>: Waitable {
    /// Puts a document on platform
    /// setting settings to `None` sets default connection behavior
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
impl<S: Signer> PutContract<S> for DataContract {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        identity_public_key: IdentityPublicKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let new_identity_nonce = sdk
            .get_identity_nonce(self.owner_id(), true, settings)
            .await?;

        let key_id = identity_public_key.id();

        let partial_identity = PartialIdentity {
            id: self.owner_id(),
            loaded_public_keys: BTreeMap::from([(key_id, identity_public_key)]),
            balance: None,
            revision: None,
            not_found_public_keys: Default::default(),
        };
        let transition = DataContractCreateTransition::new_from_data_contract(
            self.clone(),
            new_identity_nonce,
            &partial_identity,
            key_id,
            signer,
            sdk.version(),
            None,
        )?;

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
        let state_transition = self
            .put_to_platform(sdk, identity_public_key, signer, settings)
            .await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}
