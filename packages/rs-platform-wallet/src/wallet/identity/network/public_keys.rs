//! Reading an identity's public keys from Platform, outside any wallet.

use std::collections::BTreeMap;

use dash_sdk::platform::FetchMany;
use dash_sdk::Sdk;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use dpp::prelude::Identifier;

/// An identity's public keys as Platform proved them, and the Platform
/// block time the proof is signed at.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchedIdentityPublicKeys {
    /// Every key of the identity, disabled ones included, or `None` when
    /// Platform has no such identity.
    pub keys: Option<BTreeMap<KeyID, IdentityPublicKey>>,
    /// The block time, in milliseconds, of the response's metadata: the
    /// time of the Platform state the keys were proved against.
    ///
    /// It is part of what the proof verifies: the SDK hashes it into the
    /// Tenderdash `StateId` (with the height, core chain-locked height,
    /// protocol version and the proved root hash) whose precommit the
    /// validator quorum signed, so a node cannot report another time without
    /// failing verification.
    ///
    /// It is a lower bound on Platform's current time, not the current time:
    /// the SDK accepts a proof whose time is up to its time tolerance behind
    /// the local clock, and any proof on a builder without one, so a caller
    /// that needs a fresh answer must build the SDK with a time tolerance.
    /// A key with `expires_at <= block_time_ms` can no longer sign; any other
    /// key may still have expired since. See
    /// [`Self::is_expired_at_proof_time`].
    pub block_time_ms: TimestampMillis,
}

impl FetchedIdentityPublicKeys {
    /// Whether `key` has expired at [`Self::block_time_ms`], by dpp's own
    /// rule ([`IdentityPublicKeyGettersV1::is_expired_at`], the expiry
    /// instant included). `true` means the key can no longer sign; `false`
    /// does not mean it still can, since Platform's time is at least the
    /// proof's.
    pub fn is_expired_at_proof_time(&self, key: &IdentityPublicKey) -> bool {
        key.is_expired_at(self.block_time_ms)
    }
}

/// Every public key of `identity_id` as Platform proves it, disabled keys
/// included, with the block time of the proof.
///
/// Runs the proved keys-only query (all keys, no limit), which is lighter
/// than fetching the whole identity and proves the same key set. It reads
/// nothing from and writes nothing to any wallet or managed identity.
///
/// An identity always holds at least one key, so a proved answer with no
/// keys means the identity does not exist: `keys` is `None`, and the block
/// time is still the proof's. Entries the proof marks absent are dropped;
/// an all-keys request has none.
pub async fn fetch_identity_public_keys(
    sdk: &Sdk,
    identity_id: Identifier,
) -> Result<FetchedIdentityPublicKeys, dash_sdk::Error> {
    let (keys, metadata) =
        IdentityPublicKey::fetch_many_with_metadata(sdk, identity_id, None).await?;
    let keys: BTreeMap<KeyID, IdentityPublicKey> = keys
        .into_iter()
        .filter_map(|(key_id, key)| key.map(|key| (key_id, key)))
        .collect();
    Ok(FetchedIdentityPublicKeys {
        keys: (!keys.is_empty()).then_some(keys),
        block_time_ms: metadata.time_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::query_types::IdentityPublicKeys;
    use dash_sdk::SdkBuilder;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::identity_public_key::v1::IdentityPublicKeyV1;
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

    const BLOCK_TIME_MS: u64 = 1_750_000_000_000;

    async fn sdk_answering(id: Identifier, keys: Option<IdentityPublicKeys>) -> Sdk {
        let mut sdk = SdkBuilder::new_mock().build().expect("mock sdk");
        sdk.mock()
            .set_response_time_ms(BLOCK_TIME_MS)
            .expect_fetch_many::<KeyID, IdentityPublicKey, _, IdentityPublicKeys>(id, keys)
            .await
            .expect("identity keys expectation");
        sdk
    }

    #[tokio::test]
    async fn should_return_every_key_of_an_identity_platform_has_with_the_block_time() {
        let id = Identifier::from([4u8; 32]);
        let answer: IdentityPublicKeys = [
            (0, Some(key(0, None))),
            (1, Some(key(1, Some(1_700_000_000_000)))),
        ]
        .into_iter()
        .collect();
        let sdk = sdk_answering(id, Some(answer)).await;

        let fetched = fetch_identity_public_keys(&sdk, id).await.expect("fetch");

        assert_eq!(fetched.block_time_ms, BLOCK_TIME_MS);
        let keys = fetched.keys.expect("identity found");
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[&0], key(0, None));
        assert_eq!(keys[&1], key(1, Some(1_700_000_000_000)));
    }

    #[tokio::test]
    async fn should_report_an_identity_platform_does_not_have_as_none_with_the_block_time() {
        let id = Identifier::from([5u8; 32]);
        let sdk = sdk_answering(id, None).await;

        let fetched = fetch_identity_public_keys(&sdk, id).await.expect("fetch");

        assert_eq!(fetched.keys, None);
        assert_eq!(fetched.block_time_ms, BLOCK_TIME_MS);
    }

    #[test]
    fn should_judge_expiry_at_the_proof_time_by_dpp_rule() {
        let fetched = FetchedIdentityPublicKeys {
            keys: None,
            block_time_ms: BLOCK_TIME_MS,
        };
        let IdentityPublicKey::V0(v0) = key(0, None) else {
            unreachable!()
        };
        let expiring_at = |expires_at| {
            IdentityPublicKey::V1(IdentityPublicKeyV1::from_v0_with_limits(
                v0.clone(),
                None,
                expires_at,
            ))
        };

        assert!(fetched.is_expired_at_proof_time(&expiring_at(Some(BLOCK_TIME_MS - 1))));
        assert!(fetched.is_expired_at_proof_time(&expiring_at(Some(BLOCK_TIME_MS))));
        assert!(!fetched.is_expired_at_proof_time(&expiring_at(Some(BLOCK_TIME_MS + 1))));
        assert!(!fetched.is_expired_at_proof_time(&expiring_at(None)));
        assert!(!fetched.is_expired_at_proof_time(&key(1, None)));
    }

    #[tokio::test]
    async fn should_return_the_sdk_error_when_the_query_fails() {
        // No expectation: the mock transport fails the request.
        let sdk = SdkBuilder::new_mock().build().expect("mock sdk");

        let result = fetch_identity_public_keys(&sdk, Identifier::from([6u8; 32])).await;

        assert!(result.is_err());
    }
}
