use std::collections::BTreeMap;

use crate::{Error, Sdk};

use crate::platform::transition::put_settings::PutSettings;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::validate_private_document_entry_size;
use dpp::data_contract::DataContract;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, PartialIdentity};
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

use super::broadcast::BroadcastStateTransition;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;

/// Refuses `data_contract` when one of its private document types takes entries too small for
/// its largest payload in the standard private document envelope, or has a property with no
/// largest value. Platform registers such a type, since it never reads an entry, but no full
/// entry fits it and its entry size can never change, so the SDK refuses it before signing.
pub fn ensure_private_document_entries_fit(
    data_contract: &DataContract,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    for document_type in data_contract.document_types().values() {
        validate_private_document_entry_size(document_type.as_ref(), platform_version)?;
    }
    Ok(())
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
        ensure_private_document_entries_fit(self, sdk.version())?;

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
    use dpp::ProtocolError;

    /// A contract with the private type `request` of `entry_size`-byte entries, whose largest
    /// payload is 8 + 69 = 77 bytes, so it needs 77 + 299 = 376.
    fn contract(entry_size: u32) -> DataContract {
        let id = Identifier::from([7; 32]).to_string(Encoding::Base58);
        let owner_id = Identifier::from([8; 32]).to_string(Encoding::Base58);
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
                        "properties": {
                            "reference": { "type": "integer", "position": 0 },
                            "key": {
                                "type": "array", "byteArray": true,
                                "minItems": 69, "maxItems": 69, "position": 1
                            }
                        },
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
        let platform_version = PlatformVersion::latest();
        ensure_private_document_entries_fit(&contract(376), platform_version)
            .expect("376 bytes hold the largest payload");
        match ensure_private_document_entries_fit(&contract(375), platform_version) {
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
}
