//! Giving up a DPNS name a wallet identity holds, from protocol version 15: a delete, which
//! frees the name for anybody to register again, or a tombstone, which removes it and keeps
//! anybody from registering it again.
//!
//! Which of the two the network accepts for a name is the protocol's rule (a contested name
//! can only be tombstoned) and is read from the SDK, as is whether the network serves either
//! yet ([`dash_sdk::Sdk::dpns_name_give_up_options`]). A host offers what
//! [`IdentityWallet::dpns_name_give_up_choices`] reports and decides nothing of its own.
//!
//! The wallet forgets a name only once the network confirmed it gone: the identity's label
//! and the name's marketplace row are removed after the delete or the tombstone is proved,
//! never before, the label last. A give-up whose result was not confirmed, or whose
//! bookkeeping the backend refused, leaves the label; the next marketplace sync pass then
//! finds the name gone and removes the label and the row.

use std::collections::{BTreeMap, BTreeSet};

use dash_sdk::platform::documents::transitions::DocumentDeleteResult;
use dash_sdk::platform::dpns_usernames::{convert_to_homograph_safe_chars, normalize_dpns_label};
use dpp::address_funds::AddressWitness;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::DataContract;
use dpp::prelude::Identifier;
use dpp::ProtocolError;

use crate::changeset::{DpnsNameSaleStatus, DpnsNameStateChangeSet};
use crate::error::{promote_identity_insufficient_balance_or, PlatformWalletError};

use super::super::contact_requests::GatedSigner;
use super::super::document::allowed_signing_security_levels;
use super::super::usable_authentication_key_for_types;
use super::{now_ms, DpnsDomainState, IdentityWallet, DPNS_DOCUMENT_TYPE};

/// DPNS document type of a name given up for good, from DPNS v3.
const DPNS_TOMBSTONE_DOCUMENT_TYPE: &str = "tombstone";

/// The way the identity holding a DPNS name gives it up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpnsNameGiveUpMethod {
    /// The name's `domain` document is deleted: the name is free again, and anybody may
    /// register it.
    Delete,
    /// A `tombstone` document takes the name's place: the name is gone and nobody can
    /// register it again. Nobody can undo it, the identity that created it included.
    Tombstone,
}

/// Which ways the network accepts for giving up one name a wallet identity holds
/// ([`IdentityWallet::dpns_name_give_up_choices`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpnsNameGiveUpChoices {
    /// The name's `domain` document id.
    pub document_id: Identifier,
    /// Display label, as the `domain` document carries it (e.g. "Alice").
    pub label: String,
    /// Whether [`DpnsNameGiveUpMethod::Delete`] is accepted. `false` for a contested name.
    pub can_delete: bool,
    /// Whether [`DpnsNameGiveUpMethod::Tombstone`] is accepted.
    pub can_tombstone: bool,
}

impl DpnsNameGiveUpChoices {
    /// Whether the network accepts `method` for this name.
    pub fn allows(&self, method: DpnsNameGiveUpMethod) -> bool {
        match method {
            DpnsNameGiveUpMethod::Delete => self.can_delete,
            DpnsNameGiveUpMethod::Tombstone => self.can_tombstone,
        }
    }
}

/// A name given up, as the network confirmed it ([`IdentityWallet::give_up_dpns_name`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpnsNameGivenUp {
    /// The `domain` document checked before signing. A delete removed exactly this one. A
    /// tombstone consumed the owner's domain of the name at execution, which is this one
    /// unless the name was deleted and registered again in the moments between.
    pub document_id: Identifier,
    /// Display label of the name given up.
    pub label: String,
    /// How it was given up.
    pub method: DpnsNameGiveUpMethod,
    /// The id of the tombstone the network stored, for [`DpnsNameGiveUpMethod::Tombstone`].
    pub tombstone_id: Option<Identifier>,
}

/// A name the wallet checked before giving it up: what the network accepts for it, and the
/// `domain` document it read, raw and as marketplace state.
pub(super) struct DpnsGiveUpTarget {
    pub(super) choices: DpnsNameGiveUpChoices,
    /// The `domain` document read when the choices were: a delete is built for this very
    /// document, never for one read again by the name.
    pub(super) domain: Document,
    pub(super) state: DpnsDomainState,
}

/// Serializes DPNS marketplace mutations and sync reconciliation for one wallet, across its
/// cloned handles, and remembers the give-ups that were sent but not confirmed.
///
/// A give-up whose result was not confirmed may still execute. Until a read shows the name
/// gone (or held by another identity), no other give-up of it is signed: a tombstone sent
/// after a pending delete could execute first and retire a name the user only meant to
/// free, and even a second delete of the same document is a second paid transition. The
/// record lives for the session (keeping it across a restart is #6072).
///
/// Each hold carries the attempt that took it. A read clears a hold only if it is the one
/// the read saw when it started, so a read that began before a later give-up was signed
/// can not clear that give-up's hold.
#[derive(Debug, Default)]
pub(crate) struct DpnsOperationGate {
    gate: tokio::sync::Mutex<()>,
    /// The attempt holding each `(owner identity, normalized label)` given up but not
    /// confirmed.
    unresolved_give_ups: std::sync::Mutex<BTreeMap<(Identifier, String), u64>>,
    /// The last attempt number handed out; attempts start at 1.
    attempts: std::sync::atomic::AtomicU64,
}

impl DpnsOperationGate {
    /// Hold the gate for one DPNS mutation or sync pass.
    pub(crate) async fn lock(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.gate.lock().await
    }

    fn unresolved(&self) -> std::sync::MutexGuard<'_, BTreeMap<(Identifier, String), u64>> {
        self.unresolved_give_ups
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Hold `key` for a new attempt, which it returns.
    pub(super) fn hold(&self, key: (Identifier, String)) -> u64 {
        let attempt = self
            .attempts
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        self.unresolved().insert(key, attempt);
        attempt
    }

    /// The attempt holding `key`, if any.
    pub(super) fn holder(&self, key: &(Identifier, String)) -> Option<u64> {
        self.unresolved().get(key).copied()
    }

    /// Release `key` if `attempt` still holds it: by the attempt's own outcome, or by a
    /// read that saw `attempt` holding it when it started.
    pub(super) fn release(&self, key: &(Identifier, String), attempt: Option<u64>) {
        let mut unresolved = self.unresolved();
        if attempt.is_some() && unresolved.get(key).copied() == attempt {
            unresolved.remove(key);
        }
    }

    /// Whether nothing is held.
    pub(super) fn holds_nothing(&self) -> bool {
        self.unresolved().is_empty()
    }
}

/// The host's signer for a give-up (already gated on the wallet's generation), calling
/// `hold` once a signature returns: from then the signed transition can leave the wallet, so
/// the give-up must be held as unresolved even if the call is dropped before its result.
pub(super) struct HoldWhenSigned<'a, S: ?Sized> {
    pub(super) signer: &'a S,
    pub(super) hold: &'a (dyn Fn() + Send + Sync),
}

impl<S: ?Sized> std::fmt::Debug for HoldWhenSigned<'_, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HoldWhenSigned")
    }
}

#[async_trait::async_trait]
impl<S> Signer<IdentityPublicKey> for HoldWhenSigned<'_, S>
where
    S: Signer<IdentityPublicKey> + ?Sized + Send + Sync,
{
    async fn sign(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let signature = self.signer.sign(key, data).await?;
        (self.hold)();
        Ok(signature)
    }

    async fn sign_create_witness(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let witness = self.signer.sign_create_witness(key, data).await?;
        (self.hold)();
        Ok(witness)
    }

    fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
        self.signer.can_sign_with(key)
    }
}

/// Map a give-up's SDK error. A protocol version refusal stays the SDK's own error, so a
/// host is told the network does not serve the feature yet rather than that the operation
/// failed; a balance refusal, an unavailable signing key and an unconfirmed result stay
/// typed too. Anything else is reported as the failed give-up of `name`.
fn give_up_error(error: dash_sdk::Error, name: &str) -> PlatformWalletError {
    if matches!(error, dash_sdk::Error::ProtocolVersionNotActive { .. }) {
        return PlatformWalletError::Sdk(error);
    }
    promote_identity_insufficient_balance_or(error, |error| {
        // A definite consensus verdict stays the SDK's error, so the FFI reports its code
        // and kind (an unconfirmed result was kept typed above, whatever its cause says)
        if error.consensus_error().is_some() {
            PlatformWalletError::Sdk(error)
        } else {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to give up DPNS name {name:?}: {error}"
            ))
        }
    })
}

/// The refusal of a give-up while an earlier one of the same name is unconfirmed: the SDK's
/// unconfirmed outcome, so a host shows it as such (FFI code 20) and offers nothing more.
fn earlier_give_up_unconfirmed(name: &str) -> PlatformWalletError {
    PlatformWalletError::Sdk(dash_sdk::Error::StateTransitionResultUnconfirmed(Box::new(
        dash_sdk::Error::Generic(format!(
            "an earlier give-up of DPNS name {name:?} was sent and is not confirmed yet: it \
             may still execute, so no other give-up of the name is signed until a read shows \
             it gone"
        )),
    )))
}

impl IdentityWallet {
    /// Which ways the network accepts for `owner_identity_id` to give up `name` ("alice" or
    /// "alice.dash"): a delete, a tombstone, or either. Nothing is signed or broadcast.
    ///
    /// # Errors
    ///
    /// - [`PlatformWalletError::Sdk`] holding the SDK's protocol version refusal when a
    ///   verified answer shows the network below protocol version 15, where no name can be
    ///   given up, or when the network's version could not be learned.
    /// - [`PlatformWalletError::DpnsNameNotFound`] when the name is not registered.
    /// - [`PlatformWalletError::InvalidParameter`] when another identity holds the name.
    pub async fn dpns_name_give_up_choices(
        &self,
        owner_identity_id: &Identifier,
        name: &str,
    ) -> Result<DpnsNameGiveUpChoices, PlatformWalletError> {
        self.dpns_give_up_target(owner_identity_id, name)
            .await
            .map(|target| target.choices)
    }

    /// [`Self::dpns_name_give_up_choices`], keeping the `domain` document it read.
    ///
    /// It also settles an unconfirmed give-up of the name ([`DpnsOperationGate`]): a read
    /// showing the name not registered, or held by another identity, shows the earlier one
    /// executed (or no longer matters), and the record is dropped. A read showing the name
    /// still held by `owner_identity_id` proves nothing about it, so it is refused as
    /// unconfirmed again.
    async fn dpns_give_up_target(
        &self,
        owner_identity_id: &Identifier,
        name: &str,
    ) -> Result<DpnsGiveUpTarget, PlatformWalletError> {
        // The label the SDK queries, its `.dash` stripped whatever its case, and the give-up
        // holding it when this read starts: only that one may be settled by what it reads
        let key = (*owner_identity_id, normalize_dpns_label(name));
        let seen = self.dpns_operation_gate.holder(&key);
        let options = self
            .sdk
            .dpns_name_give_up_options(name)
            .await
            .map_err(|error| give_up_error(error, name))?;
        let Some(options) = options else {
            self.dpns_operation_gate.release(&key, seen);
            return Err(PlatformWalletError::DpnsNameNotFound {
                name: name.to_string(),
            });
        };
        let state = DpnsDomainState::from_document(&options.domain)?;
        let owner_id = options.domain.owner_id();
        if owner_id != *owner_identity_id {
            self.dpns_operation_gate.release(&key, seen);
            return Err(PlatformWalletError::InvalidParameter(format!(
                "DPNS name {name:?} is owned by {owner_id}, not by {owner_identity_id}"
            )));
        }
        if self.dpns_operation_gate.holder(&key).is_some() {
            return Err(earlier_give_up_unconfirmed(name));
        }
        Ok(DpnsGiveUpTarget {
            choices: DpnsNameGiveUpChoices {
                document_id: state.document_id,
                label: state.label.clone(),
                can_delete: options.can_delete,
                can_tombstone: options.can_tombstone,
            },
            domain: options.domain,
            state,
        })
    }

    /// Give up `name` ("alice" or "alice.dash"), held by the wallet identity
    /// `owner_identity_id`, by `method`. `expected_document_id` is the `domain` document the
    /// caller showed its user, from [`Self::dpns_name_give_up_choices`].
    ///
    /// Before anything is signed the name is read again: the network must serve the feature
    /// (protocol version 15, and an SDK not pinned below it), the name must be registered,
    /// held by `owner_identity_id` and still be `expected_document_id` (a name deleted and
    /// registered again since the user looked is refused, so the user never gives up a
    /// registration they did not see), and the network must accept `method` for it (a
    /// contested name can only be tombstoned). The signing key is selected on the owner, as
    /// for the other DPNS writes.
    ///
    /// A delete is built for that very `domain` document, so the network refuses it once
    /// the document is gone. A tombstone takes the owner's domain of the name at execution,
    /// whichever it is: the contract finds it by owner and name. The reported `document_id`
    /// is therefore the one checked before signing.
    ///
    /// The host signs through a signer gated on this wallet's generation: once a signature
    /// returns the generation's lifecycle gate is held to the end of the call, so a wallet
    /// removed (or re-created under its id) while the host signed sends nothing
    /// ([`PlatformWalletError::WalletRemovedBeforeSend`]), and the bookkeeping after a
    /// confirmed give-up touches only the generation that sent it.
    ///
    /// The wallet's stored state changes only once the network confirmed the name gone
    /// ([`Self::forget_given_up_dpns_name`]), and the owner's balance is refreshed (a
    /// give-up refunds what is left of the name's storage fee). On any error before that
    /// nothing stored changes. A confirmed give-up is reported as such even when the backend
    /// refuses the bookkeeping: broadcasting again would only be refused.
    pub async fn give_up_dpns_name<S>(
        &self,
        owner_identity_id: &Identifier,
        name: &str,
        expected_document_id: &Identifier,
        method: DpnsNameGiveUpMethod,
        signer: &S,
    ) -> Result<DpnsNameGivenUp, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let _operation = self.dpns_operation_gate.lock().await;
        let target = self.dpns_give_up_target(owner_identity_id, name).await?;
        if target.choices.document_id != *expected_document_id {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "DPNS name {name:?} was registered again since it was checked (document {} \
                 instead of {expected_document_id}): check it again before giving it up",
                target.choices.document_id
            )));
        }
        if !target.choices.allows(method) {
            return Err(PlatformWalletError::InvalidParameter(format!(
                "the network does not accept {method:?} for DPNS name {name:?}: a contested \
                 name can only be tombstoned"
            )));
        }
        // Both ways are signed by a key selected against the contract the network stores
        // now, which the SDK fetches checked against the protocol version it proved (an
        // ordinary fetch can take an older node's contract, which declares no tombstone).
        // A tombstone consumes the owner's domain, so its key must also be allowed every type
        // the contract lets a tombstone create consume.
        let current_contract = self
            .sdk
            .fetch_current_dpns_contract()
            .await
            .map_err(|error| give_up_error(error, name))?;
        let document_types: Vec<&str> = match method {
            DpnsNameGiveUpMethod::Delete => vec![DPNS_DOCUMENT_TYPE],
            DpnsNameGiveUpMethod::Tombstone => {
                let tombstone_type = current_contract
                    .document_type_for_name(DPNS_TOMBSTONE_DOCUMENT_TYPE)
                    .map_err(|error| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "DPNS contract has no {DPNS_TOMBSTONE_DOCUMENT_TYPE:?} document \
                             type: {error}"
                        ))
                    })?;
                std::iter::once(DPNS_TOMBSTONE_DOCUMENT_TYPE)
                    .chain(tombstone_type.consumable_document_type_names())
                    .collect()
            }
        };
        let signing_key = self
            .select_give_up_signing_key(
                owner_identity_id,
                &current_contract,
                &document_types,
                signer,
            )
            .await?;

        // The give-up is held as unresolved from the moment the host's signature returns
        // and the wallet is still current, the last step before the transition can leave
        // the wallet ([`HoldWhenSigned`]): a call dropped before that (while the nonce is
        // read or the host decides) holds nothing, and one dropped while it waits for the
        // result keeps another give-up of the name from being signed. The SDK's own verdict
        // on delivery decides the rest: a result it reports unconfirmed keeps the hold, any
        // other failure releases it.
        let unresolved_key = (*owner_identity_id, target.state.normalized_label.clone());
        let gated = GatedSigner::new(signer, self);
        // This attempt's number once it holds the name; 0 until then
        let attempt = std::sync::atomic::AtomicU64::new(0);
        let own_attempt =
            || Some(attempt.load(std::sync::atomic::Ordering::Relaxed)).filter(|n| *n != 0);
        let hold = || {
            attempt.store(
                self.dpns_operation_gate.hold(unresolved_key.clone()),
                std::sync::atomic::Ordering::Relaxed,
            );
        };
        let held = HoldWhenSigned {
            signer: &gated,
            hold: &hold,
        };
        let sent_error = |error: dash_sdk::Error| {
            if !error.is_state_transition_result_unconfirmed() || gated.removed() {
                self.dpns_operation_gate
                    .release(&unresolved_key, own_attempt());
            }
            if gated.removed() {
                return PlatformWalletError::WalletRemovedBeforeSend(hex::encode(self.wallet_id));
            }
            give_up_error(error, name)
        };
        let tombstone_id = match method {
            DpnsNameGiveUpMethod::Delete => {
                let DocumentDeleteResult::Deleted(_) = self
                    .sdk
                    .delete_dpns_domain(target.domain.clone(), &signing_key, &held)
                    .await
                    .map_err(sent_error)?;
                None
            }
            DpnsNameGiveUpMethod::Tombstone => {
                let tombstone = self
                    .sdk
                    .tombstone_dpns_name(name, *owner_identity_id, signing_key, &held)
                    .await
                    .map_err(sent_error)?;
                Some(tombstone.id())
            }
        };
        self.dpns_operation_gate
            .release(&unresolved_key, own_attempt());
        // Still under the generation's lifecycle gate the signature took (`gated` lives to
        // the end of this call): the generation that sent the give-up is the current one.
        self.forget_given_up_dpns_name(owner_identity_id, &target)
            .await;
        self.refresh_identity_after_trade(owner_identity_id, "name give-up")
            .await;
        drop(gated);
        Ok(DpnsNameGivenUp {
            document_id: target.choices.document_id,
            label: target.choices.label,
            method,
            tombstone_id,
        })
    }

    /// Drop a name the network confirmed gone from this wallet generation's stored state, in
    /// two ordered stores, each applied in memory only once the backend took it:
    ///
    /// 1. the removal of the name's marketplace rows (the checked document's, and any other
    ///    `Owned` row this identity holds under the same normalized label, such as a
    ///    registration made again from another client);
    /// 2. then the identity's snapshot without the name's label.
    ///
    /// The label is the trigger the marketplace sync pass classifies a departed name by, so
    /// it is dropped last: if the rows can not be stored, both stay, and if the label can
    /// not, it stays. Either way the next pass sees the label, finds the name gone on
    /// Platform and finishes the removal through its departure path. Neither store relies on
    /// the backend committing a changeset atomically.
    pub(super) async fn forget_given_up_dpns_name(
        &self,
        owner_identity_id: &Identifier,
        target: &DpnsGiveUpTarget,
    ) {
        let mut wm = self.wallet_manager.write().await;
        let Ok(info) = self.current_wallet_info_mut(&mut wm) else {
            return;
        };
        let mut removed: BTreeSet<Identifier> = info
            .dpns_name_states
            .values()
            .filter(|entry| {
                entry.wallet_identity_id == *owner_identity_id
                    && entry.normalized_label == target.state.normalized_label
                    && matches!(entry.status, DpnsNameSaleStatus::Owned)
            })
            .map(|entry| entry.document_id)
            .collect();
        removed.insert(target.choices.document_id);
        let rows = DpnsNameStateChangeSet {
            removed: removed.clone(),
            ..Default::default()
        };
        if let Err(error) = self.persister.withdraw_durably(rows.into()) {
            tracing::error!(
                identity = %owner_identity_id,
                document = %target.choices.document_id,
                "failed to persist the row removal of a confirmed DPNS name give-up; keeping \
                 the label and a row of the name so the next marketplace sync removes both: \
                 {error:?}"
            );
            // The departure path removes the row whose id it finds in memory: keep one for
            // the checked document even on a wallet loaded cold, whose session map does not
            // hold it, so the removal carries the id this give-up already knew
            info.dpns_name_states
                .entry(target.choices.document_id)
                .or_insert_with(|| {
                    target
                        .state
                        .to_entry(*owner_identity_id, DpnsNameSaleStatus::Owned, now_ms())
                });
            return;
        }
        for document_id in &removed {
            info.dpns_name_states.remove(document_id);
        }

        let Some(managed) = info
            .identity_manager
            .wallet_identity_mut(&self.wallet_id, owner_identity_id)
        else {
            return;
        };
        let previous_names = managed.dpns_names.clone();
        managed.dpns_names.retain(|known| {
            convert_to_homograph_safe_chars(&known.label) != target.state.normalized_label
        });
        if let Err(error) = self
            .persister
            .store_durably(managed.snapshot_changeset().into())
        {
            tracing::error!(
                identity = %owner_identity_id,
                document = %target.choices.document_id,
                "failed to persist the label removal of a confirmed DPNS name give-up; \
                 keeping the label so the next marketplace sync removes the name: {error:?}"
            );
            managed.dpns_names = previous_names;
        }
    }

    /// The signing key of a give-up acting on `document_type_names` of `contract`: an
    /// AUTHENTICATION ECDSA key of `identity_id` at a security level every one of those
    /// types accepts, whose contract bounds cover all of them
    /// ([`usable_authentication_key_for_types`]). A key bound to one of the types alone is
    /// skipped for a later key that covers both, rather than signing a transition the
    /// network refuses.
    async fn select_give_up_signing_key(
        &self,
        identity_id: &Identifier,
        contract: &DataContract,
        document_type_names: &[&str],
        signer: &impl Signer<IdentityPublicKey>,
    ) -> Result<IdentityPublicKey, PlatformWalletError> {
        let mut security_levels: Option<Vec<SecurityLevel>> = None;
        for document_type_name in document_type_names {
            let required = contract
                .document_type_for_name(document_type_name)
                .map_err(|error| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "DPNS contract has no {document_type_name:?} document type: {error}"
                    ))
                })?
                .security_level_requirement();
            let allowed = allowed_signing_security_levels(required);
            security_levels = Some(match security_levels {
                None => allowed,
                Some(levels) => levels
                    .into_iter()
                    .filter(|level| allowed.contains(level))
                    .collect(),
            });
        }
        let security_levels = security_levels.unwrap_or_default();
        let identity = {
            let wm = self.wallet_manager.read().await;
            self.current_wallet_info(&wm)?
                .identity_manager
                .wallet_identity(&self.wallet_id, identity_id)
                .map(|managed| managed.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
        };
        usable_authentication_key_for_types(
            &identity,
            signer,
            contract.id(),
            document_type_names,
            &security_levels,
            &[KeyType::ECDSA_SECP256K1],
        )?
        .cloned()
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(format!(
                "No ECDSA authentication key of identity {identity_id} may sign a DPNS \
                 {} transition: none is at a security level every type accepts with \
                 contract bounds covering them all",
                document_type_names.join(" + ")
            ))
        })
    }
}
