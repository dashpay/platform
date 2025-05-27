use std::collections::BTreeMap;

use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use crate::platform::Fetch;
use crate::Error;
use crate::Sdk;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::identity::Identity;
use dpp::state_transition::state_transitions::identity::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::StateTransition;
use dpp::state_transition::StateTransitionLike;
use dpp::voting::votes::Vote;
use dpp::ProtocolError;
use platform_value::Identifier;

/// Waitable trait provides a wait to wait for a response of a state transition after it has been broadcast and
/// receive altered objects.
///
/// This is simple conveniance trait wrapping the [`BroadcastStateTransition::wait_for_response`] method.
#[async_trait::async_trait]
pub trait Waitable: Sized {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>;
}
#[async_trait::async_trait]
impl Waitable for DataContract {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<DataContract, Error> {
        state_transition.wait_for_response(sdk, settings).await
    }
}

#[async_trait::async_trait]
impl Waitable for Document {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error> {
        let doc_id = if let StateTransition::Batch(transition) = &state_transition {
            let ids = transition.modified_data_ids();
            if ids.len() != 1 {
                return Err(Error::Protocol(
                    dpp::ProtocolError::InvalidStateTransitionType(format!(
                        "expected state transition with exactly one document, got {}",
                        ids.into_iter()
                            .map(|id| id
                                .to_string(dpp::platform_value::string_encoding::Encoding::Base58))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                ));
            }
            ids[0]
        } else {
            return Err(Error::Protocol(ProtocolError::InvalidStateTransitionType(
                format!(
                    "expected state transition to be a DocumentsBatchTransition, got {}",
                    state_transition.name()
                ),
            )));
        };
        println!("wait_for_response: {:?}", doc_id);
        let mut documents: BTreeMap<Identifier, Option<Document>> =
            state_transition.wait_for_response(sdk, settings).await?;

        let document: Document = documents
            .remove(&doc_id)
            .ok_or(Error::InvalidProvedResponse(
                "did not prove the sent document".to_string(),
            ))?
            .ok_or(Error::InvalidProvedResponse(
                "expected there to actually be a document".to_string(),
            ))?;

        Ok(document)
    }
}

#[async_trait::async_trait]
impl Waitable for Identity {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error> {
        let result: Result<Self, Error> = state_transition.wait_for_response(sdk, settings).await;

        match result {
            Ok(identity) => Ok(identity),
            // TODO: We need to refactor sdk Error to be able to retrieve gRPC error code and identify conflicts
            Err(Error::AlreadyExists(_)) => {
                let identity_id = if let StateTransition::IdentityCreate(st) = state_transition {
                    st.identity_id()
                } else {
                    return Err(Error::Generic(format!(
                        "expected identity create state transition, got {:?}",
                        state_transition.name()
                    )));
                };

                tracing::debug!(
                    ?identity_id,
                    "attempt to create identity that already exists"
                );
                let identity = Identity::fetch(sdk, identity_id).await?;
                identity.ok_or(Error::DapiClientError(
                    "identity was proved to not exist but was said to exist".to_string(),
                ))
            }
            Err(e) => Err(e),
        }
    }
}

#[async_trait::async_trait]
impl Waitable for Vote {
    async fn wait_for_response(
        sdk: &Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error> {
        state_transition.wait_for_response(sdk, settings).await
    }
}
