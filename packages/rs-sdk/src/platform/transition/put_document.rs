use super::broadcast::BroadcastStateTransition;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{Rng, SeedableRng};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters, INITIAL_REVISION};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::token_payment_info::TokenPaymentInfo;

#[async_trait::async_trait]
/// A trait for putting a document to platform
pub trait PutDocument<S: Signer<IdentityPublicKey>>: Waitable {
    /// Puts a document on platform
    /// setting settings to `None` sets default connection behavior
    #[allow(clippy::too_many_arguments)]
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Puts a document on platform and waits for the confirmation proof
    #[allow(clippy::too_many_arguments)]
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error>;
}

#[async_trait::async_trait]
impl<S: Signer<IdentityPublicKey>> PutDocument<S> for Document {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let new_identity_contract_nonce = sdk
            .get_identity_contract_nonce(
                self.owner_id(),
                document_type.data_contract_id(),
                true,
                settings,
            )
            .await?;

        let settings = settings.unwrap_or_default();
        let transition =
            if self.revision().is_some() && self.revision().unwrap() != INITIAL_REVISION {
                BatchTransition::new_document_replacement_transition_from_document(
                    self.clone(),
                    document_type.as_ref(),
                    &identity_public_key,
                    new_identity_contract_nonce,
                    settings.user_fee_increase.unwrap_or_default(),
                    token_payment_info,
                    signer,
                    sdk.version(),
                    settings.state_transition_creation_options,
                )
                .await?
            } else {
                let (document, document_state_transition_entropy) =
                    match document_state_transition_entropy {
                        Some(entropy) => {
                            // A caller-supplied entropy must derive the document's own id.
                            // Platform consensus recomputes generate_document_id_v0 from the
                            // transition entropy and rejects the create with
                            // InvalidDocumentTransitionIdError on mismatch, so guard here
                            // before broadcasting to fail locally (no wasted nonce/fee).
                            ensure_entropy_matches_document_id(
                                &document_type.data_contract_id(),
                                &self.owner_id(),
                                document_type.name(),
                                &entropy,
                                self.id(),
                            )?;
                            (self.clone(), entropy)
                        }
                        None => {
                            let mut rng = StdRng::from_entropy();
                            let mut document = self.clone();
                            let entropy = rng.gen::<[u8; 32]>();
                            document.set_id(Document::generate_document_id_v0(
                                &document_type.data_contract_id(),
                                &document.owner_id(),
                                document_type.name(),
                                entropy.as_slice(),
                            ));
                            (document, entropy)
                        }
                    };
                BatchTransition::new_document_creation_transition_from_document(
                    document,
                    document_type.as_ref(),
                    document_state_transition_entropy,
                    &identity_public_key,
                    new_identity_contract_nonce,
                    settings.user_fee_increase.unwrap_or_default(),
                    token_payment_info,
                    signer,
                    sdk.version(),
                    settings.state_transition_creation_options,
                )
                .await?
            };
        ensure_valid_state_transition_structure(&transition, sdk.version())?;

        // response is empty for a broadcast, result comes from the stream wait for state transition result
        transition.broadcast(sdk, Some(settings)).await?;
        Ok(transition)
    }

    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error> {
        let state_transition = self
            .put_to_platform(
                sdk,
                document_type,
                document_state_transition_entropy,
                identity_public_key,
                token_payment_info,
                signer,
                settings,
            )
            .await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}

/// Ensures a caller-supplied `entropy` derives the same document id already set
/// on a create document.
///
/// A document-create state transition carries both the document id and the
/// entropy, and Drive recomputes the id from the entropy during
/// `advanced_structure` validation, rejecting the transition with
/// `InvalidDocumentTransitionIdError` when they disagree. Because
/// [`PutDocument::put_to_platform`] trusts the caller's id verbatim in the
/// `Some(entropy)` arm, a two-phase caller whose id and entropy have drifted
/// would only discover the mismatch after paying (a bumped identity-contract
/// nonce). This check surfaces the mismatch locally before broadcasting.
fn ensure_entropy_matches_document_id(
    contract_id: &Identifier,
    owner_id: &Identifier,
    document_type_name: &str,
    entropy: &[u8; 32],
    document_id: Identifier,
) -> Result<(), Error> {
    let expected_id = Document::generate_document_id_v0(
        contract_id,
        owner_id,
        document_type_name,
        entropy.as_slice(),
    );
    if expected_id != document_id {
        return Err(Error::Generic(format!(
            "document id {document_id} does not match the id {expected_id} derived from the \
             supplied entropy; the entropy must be the one used to generate the document id"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract_id() -> Identifier {
        Identifier::from([1u8; 32])
    }

    fn owner_id() -> Identifier {
        Identifier::from([2u8; 32])
    }

    #[test]
    fn matching_entropy_and_id_pass() {
        let entropy = [7u8; 32];
        let id = Document::generate_document_id_v0(
            &contract_id(),
            &owner_id(),
            "contactRequest",
            entropy.as_slice(),
        );

        ensure_entropy_matches_document_id(
            &contract_id(),
            &owner_id(),
            "contactRequest",
            &entropy,
            id,
        )
        .expect("id derived from the supplied entropy must be accepted");
    }

    #[test]
    fn mismatched_entropy_and_id_error_before_broadcast() {
        // The id was derived from E1, but the caller passes E2 != E1 (mirroring
        // the very drift consensus rejects with InvalidDocumentTransitionIdError).
        let entropy_used = [1u8; 32];
        let id = Document::generate_document_id_v0(
            &contract_id(),
            &owner_id(),
            "contactRequest",
            entropy_used.as_slice(),
        );

        let different_entropy = [2u8; 32];
        let result = ensure_entropy_matches_document_id(
            &contract_id(),
            &owner_id(),
            "contactRequest",
            &different_entropy,
            id,
        );

        assert!(
            matches!(result, Err(Error::Generic(_))),
            "a document id derived from a different entropy must be rejected locally"
        );
    }
}
