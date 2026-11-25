//! Key references of any masternode, tracked or not: what the current list
//! knows plus the key hashes its Platform identities carry (owner identity:
//! OWNER and payout TRANSFER keys; operator identity: operator payout
//! TRANSFER key).

use std::future::Future;

use dash_sdk::platform::Fetch;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose};
use dpp::prelude::Identifier;

use super::list::CurrentMasternodeList;
use super::locator::{operator_identity_ids, MasternodeKeyReference};
use super::tracked::first_operator_identity;
use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;
use crate::manager::PlatformWalletManager;

/// Whether `key` is an enabled `ECDSA_HASH160` key for `purpose`. Drive
/// disables a masternode identity's replaced keys instead of removing them,
/// so a disabled key is stale.
fn is_enabled_hash160_key(key: &IdentityPublicKey, purpose: Purpose) -> bool {
    key.purpose() == purpose && key.key_type() == KeyType::ECDSA_HASH160 && !key.is_disabled()
}

/// The hash of `identity`'s enabled `ECDSA_HASH160` key for `purpose`
/// ([`is_enabled_hash160_key`]), the newest by key id if several are.
pub(super) fn enabled_key_hash(identity: &Identity, purpose: Purpose) -> Option<[u8; 20]> {
    identity
        .public_keys()
        .values()
        .filter(|key| is_enabled_hash160_key(key, purpose))
        .filter_map(|key| key.data().as_slice().try_into().ok())
        .next_back()
}

/// What `list` and the node's Platform identities know about the keys of
/// `pro_tx_hash` (wire order). `fetch` reads one identity by id. `Ok(None)`
/// when the node is neither on the list nor has an owner identity; a node
/// absent from the list keeps its list fields `None`.
pub(super) async fn key_reference_from_list_and_identities<F, Fut, E>(
    list: &CurrentMasternodeList,
    pro_tx_hash: &[u8; 32],
    mut fetch: F,
) -> Result<Option<MasternodeKeyReference>, E>
where
    F: FnMut(Identifier) -> Fut,
    Fut: Future<Output = Result<Option<Identity>, E>>,
{
    let listed = list.key_reference(pro_tx_hash);
    let mut display = *pro_tx_hash;
    display.reverse();
    let owner_identity = fetch(Identifier::from(display)).await?;
    if listed.is_none() && owner_identity.is_none() {
        return Ok(None);
    }

    let mut reference = listed.unwrap_or_default();
    if let Some(identity) = &owner_identity {
        reference.owner_key_hash = enabled_key_hash(identity, Purpose::OWNER);
        reference.payout_key_hash = enabled_key_hash(identity, Purpose::TRANSFER);
    }
    if let Some(operator_key) = reference.operator_public_key {
        let ids = operator_identity_ids(&display, &operator_key);
        if let Some(identity) = first_operator_identity(ids, &mut fetch).await? {
            reference.operator_payout_key_hash = enabled_key_hash(&identity, Purpose::TRANSFER);
        }
    }
    Ok(Some(reference))
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// The key references of any masternode, tracked or not: the current
    /// list's fields, owner and payout key hashes from its owner identity,
    /// the operator payout key hash from its operator identity. Nothing is
    /// persisted.
    ///
    /// Fails with [`PlatformWalletError::MasternodeListUnavailable`] while
    /// the masternode list does not follow the chain. `Ok(None)` when the
    /// node is neither on the list nor has an owner identity.
    pub async fn fetch_masternode_key_reference(
        &self,
        pro_tx_hash: &[u8; 32],
    ) -> Result<Option<MasternodeKeyReference>, PlatformWalletError> {
        let list = self.spv().current_masternode_list().await?;
        let reference = key_reference_from_list_and_identities(&list, pro_tx_hash, |id| {
            Identity::fetch(self.sdk(), id)
        })
        .await?;
        Ok(reference)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Masternode identities with chosen keys, for the tests here and the
    //! tracked-node refresh tests.
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;

    /// A key of `purpose` and `key_type` whose data is `hash`.
    pub(crate) fn key(
        id: KeyID,
        purpose: Purpose,
        key_type: KeyType,
        hash: [u8; 20],
        disabled: bool,
    ) -> IdentityPublicKey {
        IdentityPublicKeyV0 {
            id,
            key_type,
            purpose,
            security_level: SecurityLevel::CRITICAL,
            read_only: true,
            data: BinaryData::new(hash.to_vec()),
            disabled_at: disabled.then_some(1),
            contract_bounds: None,
        }
        .into()
    }

    pub(crate) fn identity_with(id: Identifier, keys: Vec<IdentityPublicKey>) -> Identity {
        IdentityV0 {
            id,
            public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
            balance: 0,
            revision: 0,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{identity_with, key};
    use super::*;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::masternode::list::test_support::masternode;
    use crate::masternode::list::MasternodeListSummary;
    use crate::wallet::persister::NoPlatformPersistence;
    use std::sync::Arc;

    const STALE: [u8; 20] = [0x01; 20];
    const CURRENT: [u8; 20] = [0x02; 20];

    /// Drive disables a replaced key instead of removing it, and the new key
    /// may sit at a lower or a higher key id than the one it replaced.
    #[test]
    fn should_pick_the_enabled_key_over_a_disabled_one_in_either_key_order() {
        for purpose in [Purpose::OWNER, Purpose::TRANSFER] {
            for (disabled_id, enabled_id) in [(0, 1), (1, 0)] {
                let identity = identity_with(
                    Identifier::from([9u8; 32]),
                    vec![
                        key(disabled_id, purpose, KeyType::ECDSA_HASH160, STALE, true),
                        key(enabled_id, purpose, KeyType::ECDSA_HASH160, CURRENT, false),
                    ],
                );
                assert_eq!(
                    enabled_key_hash(&identity, purpose),
                    Some(CURRENT),
                    "{purpose:?}, disabled key id {disabled_id}"
                );
            }
        }
    }

    /// Only an `ECDSA_HASH160` key carries a public key hash; another 20-byte
    /// key of the same purpose is not one.
    #[test]
    fn should_ignore_keys_that_are_not_ecdsa_hash160() {
        let identity = identity_with(
            Identifier::from([9u8; 32]),
            vec![
                key(
                    0,
                    Purpose::TRANSFER,
                    KeyType::BIP13_SCRIPT_HASH,
                    STALE,
                    false,
                ),
                key(
                    1,
                    Purpose::OWNER,
                    KeyType::EDDSA_25519_HASH160,
                    STALE,
                    false,
                ),
            ],
        );
        assert_eq!(enabled_key_hash(&identity, Purpose::TRANSFER), None);
        assert_eq!(enabled_key_hash(&identity, Purpose::OWNER), None);
    }

    fn display(pro_tx_hash: &[u8; 32]) -> [u8; 32] {
        let mut out = *pro_tx_hash;
        out.reverse();
        out
    }

    /// Answer identity fetches from `identities`, recording every id asked.
    async fn reference_with(
        list: &CurrentMasternodeList,
        pro_tx_hash: &[u8; 32],
        identities: Vec<Identity>,
    ) -> (
        Result<Option<MasternodeKeyReference>, String>,
        Vec<Identifier>,
    ) {
        let mut requested = Vec::new();
        let found = key_reference_from_list_and_identities(list, pro_tx_hash, |id| {
            requested.push(id);
            std::future::ready(Ok(identities.iter().find(|i| i.id() == id).cloned()))
        })
        .await;
        (found, requested)
    }

    fn list_of(summaries: Vec<MasternodeListSummary>) -> CurrentMasternodeList {
        CurrentMasternodeList {
            height: 1,
            summaries,
        }
    }

    /// A listed node: the list's keys, the owner identity's enabled OWNER and
    /// TRANSFER keys, and the operator identity's enabled TRANSFER key.
    #[tokio::test]
    async fn should_join_the_list_entry_with_both_identities_of_a_listed_node() {
        let pro_tx_hash = [1u8; 32];
        let owner = identity_with(
            Identifier::from(display(&pro_tx_hash)),
            vec![
                key(0, Purpose::TRANSFER, KeyType::ECDSA_HASH160, STALE, true),
                key(1, Purpose::OWNER, KeyType::ECDSA_HASH160, [0x11; 20], false),
                key(2, Purpose::TRANSFER, KeyType::ECDSA_HASH160, CURRENT, false),
            ],
        );
        let operator_id = operator_identity_ids(&display(&pro_tx_hash), &[1u8; 48])[0];
        let operator = identity_with(
            operator_id,
            vec![key(
                0,
                Purpose::TRANSFER,
                KeyType::ECDSA_HASH160,
                [0x33; 20],
                false,
            )],
        );

        let (found, _) = reference_with(
            &list_of(vec![masternode(1), masternode(2)]),
            &pro_tx_hash,
            vec![owner, operator],
        )
        .await;

        assert_eq!(
            found,
            Ok(Some(MasternodeKeyReference {
                owner_key_hash: Some([0x11; 20]),
                voting_key_id: Some([1u8; 20]),
                operator_public_key: Some([1u8; 48]),
                platform_node_id: None,
                payout_key_hash: Some(CURRENT),
                operator_payout_key_hash: Some([0x33; 20]),
            }))
        );
    }

    /// A retired node still has an owner identity (its credits stay
    /// claimable): its owner and payout keys come back, the list's stay
    /// unknown, and no operator identity is looked up without an operator
    /// key.
    #[tokio::test]
    async fn should_return_the_owner_identity_keys_of_a_node_absent_from_the_list() {
        let pro_tx_hash = [7u8; 32];
        let owner_id = Identifier::from(display(&pro_tx_hash));
        let owner = identity_with(
            owner_id,
            vec![
                key(0, Purpose::TRANSFER, KeyType::ECDSA_HASH160, CURRENT, false),
                key(1, Purpose::OWNER, KeyType::ECDSA_HASH160, [0x11; 20], false),
            ],
        );

        let (found, requested) =
            reference_with(&list_of(vec![masternode(1)]), &pro_tx_hash, vec![owner]).await;

        assert_eq!(
            found,
            Ok(Some(MasternodeKeyReference {
                owner_key_hash: Some([0x11; 20]),
                payout_key_hash: Some(CURRENT),
                ..Default::default()
            }))
        );
        assert_eq!(requested, vec![owner_id]);
    }

    /// Neither the list nor Platform knows the node.
    #[tokio::test]
    async fn should_return_nothing_for_a_node_unknown_to_the_list_and_platform() {
        let (found, _) = reference_with(&list_of(vec![masternode(1)]), &[7u8; 32], vec![]).await;
        assert_eq!(found, Ok(None));
    }

    /// A listed node without Platform identities keeps the list's keys.
    #[tokio::test]
    async fn should_return_the_list_keys_of_a_listed_node_without_identities() {
        let (found, _) = reference_with(&list_of(vec![masternode(1)]), &[1u8; 32], vec![]).await;
        assert_eq!(
            found,
            Ok(Some(MasternodeKeyReference::from_summary(&masternode(1))))
        );
    }

    /// An unreadable identity fails the read rather than returning a
    /// reference that silently lacks its keys.
    #[tokio::test]
    async fn should_fail_when_an_identity_fetch_fails() {
        let found = key_reference_from_list_and_identities(
            &list_of(vec![masternode(1)]),
            &[1u8; 32],
            |_| std::future::ready(Err::<Option<Identity>, _>("unreadable".to_string())),
        )
        .await;
        assert_eq!(found, Err("unreadable".to_string()));
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    /// The manager read refuses while the list does not follow the chain; a
    /// manager whose SPV never started is such a case.
    #[tokio::test]
    async fn should_refuse_a_key_reference_while_the_list_is_not_current() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = PlatformWalletManager::new(
            sdk,
            Arc::new(NoPlatformPersistence),
            Arc::new(NoopEventHandler) as Arc<dyn PlatformEventHandler>,
        );

        let read = manager.fetch_masternode_key_reference(&[1u8; 32]).await;

        assert!(
            matches!(read, Err(PlatformWalletError::MasternodeListUnavailable)),
            "expected MasternodeListUnavailable, got {read:?}"
        );
    }
}
