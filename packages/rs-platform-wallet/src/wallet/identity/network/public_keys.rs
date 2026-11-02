//! Reading an identity's public keys from Platform, outside any wallet.

use std::collections::BTreeMap;

use dash_sdk::platform::FetchMany;
use dash_sdk::Sdk;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::Identifier;

/// Every public key of `identity_id` as Platform proves it, disabled keys
/// included, or `None` when Platform has no such identity.
///
/// Runs the proved keys-only query (all keys, no limit), which is lighter
/// than fetching the whole identity and proves the same key set. It reads
/// nothing from and writes nothing to any wallet or managed identity.
///
/// An identity always holds at least one key, so a proved answer with no
/// keys means the identity does not exist. Entries the proof marks absent
/// are dropped; an all-keys request has none.
pub async fn fetch_identity_public_keys(
    sdk: &Sdk,
    identity_id: Identifier,
) -> Result<Option<BTreeMap<KeyID, IdentityPublicKey>>, dash_sdk::Error> {
    let keys: BTreeMap<KeyID, IdentityPublicKey> = IdentityPublicKey::fetch_many(sdk, identity_id)
        .await?
        .into_iter()
        .filter_map(|(key_id, key)| key.map(|key| (key_id, key)))
        .collect();
    Ok((!keys.is_empty()).then_some(keys))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::query_types::IdentityPublicKeys;
    use dash_sdk::SdkBuilder;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;

    fn key(id: KeyID, disabled_at: Option<u64>) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            read_only: false,
            data: BinaryData::new(vec![id as u8; 33]),
            disabled_at,
            contract_bounds: None,
        })
    }

    async fn sdk_answering(id: Identifier, keys: Option<IdentityPublicKeys>) -> Sdk {
        let mut sdk = SdkBuilder::new_mock().build().expect("mock sdk");
        sdk.mock()
            .expect_fetch_many::<KeyID, IdentityPublicKey, _, IdentityPublicKeys>(id, keys)
            .await
            .expect("identity keys expectation");
        sdk
    }

    #[tokio::test]
    async fn should_return_every_key_of_an_identity_platform_has() {
        let id = Identifier::from([4u8; 32]);
        let answer: IdentityPublicKeys = [
            (0, Some(key(0, None))),
            (1, Some(key(1, Some(1_700_000_000_000)))),
        ]
        .into_iter()
        .collect();
        let sdk = sdk_answering(id, Some(answer)).await;

        let keys = fetch_identity_public_keys(&sdk, id)
            .await
            .expect("fetch")
            .expect("identity found");

        assert_eq!(keys.len(), 2);
        assert_eq!(keys[&0], key(0, None));
        assert_eq!(keys[&1], key(1, Some(1_700_000_000_000)));
    }

    #[tokio::test]
    async fn should_report_an_identity_platform_does_not_have_as_none() {
        let id = Identifier::from([5u8; 32]);
        let sdk = sdk_answering(id, None).await;

        let keys = fetch_identity_public_keys(&sdk, id).await.expect("fetch");

        assert_eq!(keys, None);
    }

    #[tokio::test]
    async fn should_return_the_sdk_error_when_the_query_fails() {
        // No expectation: the mock transport fails the request.
        let sdk = SdkBuilder::new_mock().build().expect("mock sdk");

        let result = fetch_identity_public_keys(&sdk, Identifier::from([6u8; 32])).await;

        assert!(result.is_err());
    }
}
