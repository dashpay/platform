use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;

use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use crate::platform::Fetch;
use crate::Error;
use crate::Sdk;
use dpp::document::Document;
use dpp::prelude::{DataContract, Identifier, Identity};
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::StateTransition;
use dpp::state_transition::StateTransitionLike;
use dpp::voting::votes::Vote;
use dpp::ProtocolError;

/// Waitable trait provides a way to wait for a response of a state transition after it has been broadcast and
/// receive altered objects.
///
/// This is a simple convenience trait wrapping the [`BroadcastStateTransition::wait_for_response`] method.
pub trait Waitable: Sized {
    fn wait_for_response<'a>(
        sdk: &'a Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>>;
}

impl Waitable for DataContract {
    fn wait_for_response<'a>(
        sdk: &'a Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<DataContract, Error>> + Send + 'a>> {
        Box::pin(async move { state_transition.wait_for_response(sdk, settings).await })
    }
}

impl Waitable for Document {
    fn wait_for_response<'a>(
        sdk: &'a Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let doc_id = if let StateTransition::Batch(transition) = &state_transition {
                let ids = transition.modified_data_ids();
                if ids.len() != 1 {
                    return Err(Error::Protocol(
                        dpp::ProtocolError::InvalidStateTransitionType(format!(
                            "expected state transition with exactly one document, got {}",
                            ids.into_iter()
                                .map(|id| id.to_string(
                                    dpp::platform_value::string_encoding::Encoding::Base58
                                ))
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
        })
    }
}

impl Waitable for Identity {
    fn wait_for_response<'a>(
        sdk: &'a Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move {
            let result: Result<Self, Error> =
                state_transition.wait_for_response(sdk, settings).await;

            match result {
                Ok(identity) => Ok(identity),
                // TODO: We need to refactor sdk Error to be able to retrieve gRPC error code and identify conflicts
                Err(Error::AlreadyExists(_)) => {
                    let identity_id = if let StateTransition::IdentityCreate(st) = state_transition
                    {
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
                    identity.ok_or(Error::Generic(
                        "identity was proved to not exist but was said to exist".to_string(),
                    ))
                }
                Err(e) => Err(e),
            }
        })
    }
}

impl Waitable for Vote {
    fn wait_for_response<'a>(
        sdk: &'a Sdk,
        state_transition: StateTransition,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + 'a>> {
        Box::pin(async move { state_transition.wait_for_response(sdk, settings).await })
    }
}
