use std::collections::BTreeMap;

use crate::{Error, Sdk};

use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Fetch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::validate_private_document_entry_size;
use dpp::data_contract::DataContract;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, PartialIdentity};
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::StateTransition;

use super::broadcast::BroadcastStateTransition;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;

/// Refuses `data_contract` when one of its private document types takes entries too small for
/// its largest payload in the standard private document envelope, or has a property with no
/// largest value. Platform registers such a type, since it never reads an entry, but no full
/// entry fits it and its entry size can never change, so the SDK refuses it before signing.
///
/// For an update, `stored_contract` is the contract as Platform holds it: a type it already
/// holds with the same properties is not checked, since the update does not change what its
/// entries carry, so an update is never refused for a type registered before it.
pub fn ensure_private_document_entries_fit(
    data_contract: &DataContract,
    stored_contract: Option<&DataContract>,
) -> Result<(), Error> {
    for (name, document_type) in data_contract.document_types() {
        let unchanged = stored_contract
            .and_then(|stored| stored.document_types().get(name))
            .is_some_and(|stored_type| stored_type.properties() == document_type.properties());
        if !unchanged {
            validate_private_document_entry_size(document_type.as_ref())?;
        }
    }
    Ok(())
}

/// [`ensure_private_document_entries_fit`] for an update of `data_contract`, against the contract
/// as Platform holds it. It fetches that only when `data_contract` declares a private document
/// type, the one case with anything to check.
pub async fn ensure_private_document_entries_fit_on_update(
    sdk: &Sdk,
    data_contract: &DataContract,
) -> Result<(), Error> {
    let declares_private_types = data_contract
        .document_types()
        .values()
        .any(|document_type| document_type.private_document().is_some());
    if !declares_private_types {
        return Ok(());
    }
    let stored_contract = DataContract::fetch(sdk, data_contract.id()).await?;
    ensure_private_document_entries_fit(data_contract, stored_contract.as_ref())
}

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
        ensure_private_document_entries_fit(self, None)?;

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
        )
        .await?;
        ensure_valid_state_transition_structure(&transition, sdk.version())?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::platform_value::{platform_value, Identifier};
    use dpp::version::PlatformVersion;
    use dpp::ProtocolError;

    /// A contract with the private type `request` of `entry_size`-byte entries, whose largest
    /// payload is 8 + 69 = 77 bytes, so it needs 77 + 299 = 376; with `label`, an optional string
    /// of at most 20 bytes, 22 more.
    fn contract(entry_size: u32, label: bool) -> DataContract {
        let id = Identifier::from([7; 32]).to_string(Encoding::Base58);
        let owner_id = Identifier::from([8; 32]).to_string(Encoding::Base58);
        let mut properties = platform_value!({
            "reference": { "type": "integer", "position": 0 },
            "key": {
                "type": "array", "byteArray": true,
                "minItems": 69, "maxItems": 69, "position": 1
            }
        });
        if label {
            properties
                .set_value(
                    "label",
                    platform_value!({ "type": "string", "maxBytes": 20, "position": 2 }),
                )
                .expect("a map");
        }
        DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": id,
                "ownerId": owner_id,
                "version": 1,
                "documentSchemas": {
                    "request": {
                        "type": "object",
                        "private": { "entrySize": entry_size },
                        "documentsMutable": false,
                        "canBeDeleted": false,
                        "properties": properties,
                        "required": ["reference", "key"],
                        "additionalProperties": false
                    }
                }
            }),
            true,
            PlatformVersion::latest(),
        )
        .expect("the contract registers")
    }

    #[test]
    fn should_refuse_a_contract_whose_private_entries_cannot_hold_their_payload() {
        ensure_private_document_entries_fit(&contract(376, false), None)
            .expect("376 bytes hold the largest payload");
        match ensure_private_document_entries_fit(&contract(375, false), None) {
            Err(Error::Protocol(ProtocolError::PrivateDocumentEntrySizeTooSmall {
                document_type_name,
                entry_size,
                minimum_entry_size,
            })) => {
                assert_eq!(
                    (document_type_name.as_str(), entry_size, minimum_entry_size),
                    ("request", 375, 376)
                );
            }
            other => panic!("expected the entry size refusal, got {other:?}"),
        }
    }

    /// An update checks the private types it adds or whose properties it changes, and leaves a
    /// type Platform already holds unchanged, even one too small, since the update does not
    /// change what its entries carry.
    #[test]
    fn should_check_only_the_private_types_an_update_adds_or_changes() {
        let stored = contract(375, false);
        ensure_private_document_entries_fit(&contract(375, false), Some(&stored))
            .expect("an unchanged type is not checked");
        assert!(matches!(
            ensure_private_document_entries_fit(&contract(375, true), Some(&stored),),
            Err(Error::Protocol(
                ProtocolError::PrivateDocumentEntrySizeTooSmall {
                    minimum_entry_size: 398,
                    ..
                }
            ))
        ));
        // A type the stored contract does not hold is new, and checked.
        let mut without_the_type = contract(4096, false);
        without_the_type.document_types_mut().clear();
        assert!(matches!(
            ensure_private_document_entries_fit(&contract(375, false), Some(&without_the_type),),
            Err(Error::Protocol(
                ProtocolError::PrivateDocumentEntrySizeTooSmall { .. }
            ))
        ));
    }

    /// A mock SDK at the latest protocol version, the first whose contracts declare private
    /// types.
    fn latest_mock_sdk() -> Sdk {
        crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("a mock SDK")
    }

    /// A contract holding only the public type `note`.
    fn public_contract() -> DataContract {
        let id = Identifier::from([7; 32]).to_string(Encoding::Base58);
        let owner_id = Identifier::from([8; 32]).to_string(Encoding::Base58);
        DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": id,
                "ownerId": owner_id,
                "version": 1,
                "documentSchemas": {
                    "note": {
                        "type": "object",
                        "properties": {
                            "text": { "type": "string", "maxLength": 63, "position": 0 }
                        },
                        "additionalProperties": false
                    }
                }
            }),
            true,
            PlatformVersion::latest(),
        )
        .expect("the contract registers")
    }

    /// An update declaring a private type is checked against the contract Platform holds,
    /// fetched: an unchanged undersized type passes, a changed one is refused, every type is new
    /// when Platform holds no such contract, and a fetch that fails stops the update. An update
    /// declaring no private type fetches nothing.
    #[tokio::test]
    async fn should_check_an_update_against_the_contract_platform_holds() {
        let stored = contract(375, false);
        let mut sdk = latest_mock_sdk();
        sdk.mock()
            .expect_fetch(stored.id(), Some(stored.clone()))
            .await
            .expect("an expectation");
        ensure_private_document_entries_fit_on_update(&sdk, &contract(375, false))
            .await
            .expect("an unchanged type is not checked");
        assert!(matches!(
            ensure_private_document_entries_fit_on_update(&sdk, &contract(375, true)).await,
            Err(Error::Protocol(
                ProtocolError::PrivateDocumentEntrySizeTooSmall {
                    minimum_entry_size: 398,
                    ..
                }
            ))
        ));

        let mut absent = latest_mock_sdk();
        absent
            .mock()
            .expect_fetch(stored.id(), None::<DataContract>)
            .await
            .expect("an expectation");
        assert!(matches!(
            ensure_private_document_entries_fit_on_update(&absent, &contract(375, false)).await,
            Err(Error::Protocol(
                ProtocolError::PrivateDocumentEntrySizeTooSmall { .. }
            ))
        ));

        // No expectation: the mock fails the fetch, and the update stops there.
        let failing = latest_mock_sdk();
        let error = ensure_private_document_entries_fit_on_update(&failing, &contract(376, false))
            .await
            .expect_err("the fetch fails");
        assert!(!matches!(error, Error::Protocol(_)), "{error:?}");
        ensure_private_document_entries_fit_on_update(&failing, &public_contract())
            .await
            .expect("a contract with no private type is not fetched");
    }
}
