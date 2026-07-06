package org.dashfoundation.dashsdk.ffi

/**
 * The receiving side of the platform-wallet-ffi `PersistenceCallbacks`
 * vtable, reached from Rust via JNI trampolines in
 * `rs-unified-sdk-jni/src/persistence.rs`.
 *
 * ## Why an abstract class, not an interface
 *
 * The Rust trampolines resolve each method with a single `GetMethodID`
 * against this concrete class. A `class` hierarchy keeps that lookup
 * stable and lets `PersistenceNative.createCallbacks` cache the method
 * ids off one `FindClass`. Every method has a base implementation here so
 * a subclass only overrides the slots it cares about; the defaults are
 * no-ops that return the success sentinel.
 *
 * ## The 32 vtable slots
 *
 * Each method maps 1:1 onto a `PersistenceCallbacks` slot (names kept
 * verbatim modulo camelCase). The exact JNI descriptor the Rust side
 * uses to resolve the method is documented next to each declaration —
 * these MUST stay byte-for-byte in sync with `persistence.rs`.
 *
 * ## Marshalling conventions
 *
 * - `walletId` and every other id is a 32-byte (`ByteArray`) copy made
 *   inside the trampoline before the Rust-owned pointer expires.
 * - Array-of-struct persist payloads are delivered **one bridge call per
 *   entry** with flat primitive + `ByteArray` args (spec option (a)),
 *   except the deeply nested wallet changeset which is decomposed into
 *   per-account / per-utxo / per-transaction calls bracketed by
 *   [onWalletChangesetAccountBegin] / [onWalletChangesetAccountEnd].
 * - Persist slots return `Int` (0 = ok, non-zero flips the round's
 *   success flag so [onChangesetEnd] delivers the rollback).
 * - Load slots return flattened representations (`Array<...>` / typed
 *   holder objects) that the trampoline re-packs into Rust-owned FFI
 *   structs; Kotlin never allocates native memory.
 *
 * ## Threading
 *
 * Callbacks arrive **synchronously** on Rust Tokio worker threads and
 * must complete before returning. Subclasses that touch Room do so with
 * `runBlocking` on a dedicated dispatcher (see
 * `PlatformWalletPersistenceHandler`).
 *
 * Known coverage deviation: the deferred contact-crypto queue
 * (`PlatformWalletChangeSet.pending_contact_crypto_added/_cleared`) has
 * no vtable slot, so it is NOT durable on this host — a restart before a
 * signer-backed drain relies on the recurring sweep to re-enqueue.
 */
abstract class NativePersistenceBridge {

    // ── Transactional bracketing ──────────────────────────────────────

    /** `on_changeset_begin_fn` — descriptor `([B)I`. */
    open fun onChangesetBegin(walletId: ByteArray): Int = 0

    /** `on_changeset_end_fn` — descriptor `([BZ)I`. */
    open fun onChangesetEnd(walletId: ByteArray, success: Boolean): Int = 0

    /** `on_store_fn` — descriptor `([B)I`. */
    open fun onStore(walletId: ByteArray): Int = 0

    /** `on_flush_fn` — descriptor `([B)I`. */
    open fun onFlush(walletId: ByteArray): Int = 0

    // ── Platform address balances ─────────────────────────────────────

    /**
     * `on_persist_address_balances_fn`, one call per `AddressBalanceEntryFFI`.
     * Descriptor `([BB[BJIIIJ)I`.
     *
     * @param addressType 0 = P2PKH, 1 = P2SH
     * @param addressHash 20-byte platform-address hash
     */
    open fun onPersistAddressBalance(
        walletId: ByteArray,
        addressType: Byte,
        addressHash: ByteArray,
        balance: Long,
        nonce: Int,
        accountIndex: Int,
        addressIndex: Int,
        asOfHeight: Long,
    ): Int = 0

    // ── Sync state ────────────────────────────────────────────────────

    /** `on_persist_sync_state_fn` — descriptor `([BJJJ)I`. */
    open fun onPersistSyncState(
        walletId: ByteArray,
        syncHeight: Long,
        syncTimestamp: Long,
        lastKnownRecentBlock: Long,
    ): Int = 0

    // ── Wallet metadata ───────────────────────────────────────────────

    /**
     * `on_persist_wallet_metadata_fn` — descriptor `([BI[BI)I`.
     *
     * @param network FFINetwork ordinal (0 Mainnet, 1 Testnet, 2 Devnet, 3 Regtest)
     * @param walletGroupId 32-byte network-independent group id
     */
    open fun onPersistWalletMetadata(
        walletId: ByteArray,
        network: Int,
        walletGroupId: ByteArray,
        birthHeight: Int,
    ): Int = 0

    // ── Account registrations ─────────────────────────────────────────

    /**
     * `on_persist_account_registrations_fn`, one call per `AccountSpecFFI`.
     * Descriptor `([BBBIII[B[B[B)I`.
     *
     * @param typeTag AccountTypeTagFFI raw value
     * @param standardTag StandardAccountTypeTagFFI raw value
     * @param userIdentityId 32-byte id (Dashpay variants), else 32 zero bytes
     * @param friendIdentityId 32-byte id (Dashpay variants), else 32 zero bytes
     * @param accountXpubBytes bincode `ExtendedPubKey`, or empty
     */
    open fun onPersistAccountRegistration(
        walletId: ByteArray,
        typeTag: Byte,
        standardTag: Byte,
        index: Int,
        registrationIndex: Int,
        keyClass: Int,
        userIdentityId: ByteArray,
        friendIdentityId: ByteArray,
        accountXpubBytes: ByteArray,
    ): Int = 0

    // ── Account address pools ─────────────────────────────────────────

    /**
     * `on_persist_account_address_pools_fn`, one call per
     * `CoreAddressEntryFFI` inside each `AccountAddressPoolFFI`. The owning
     * account spec fields are flattened onto every row so the handler can
     * resolve the parent account. Descriptor
     * `([BBBIII[B[BB[BZBIZJLjava/lang/String;Ljava/lang/String;)I`.
     *
     * @param poolTypeTag AddressPoolTypeTagFFI raw value
     * @param publicKey 33-byte compressed pubkey (valid iff hasPublicKey)
     */
    @Suppress("LongParameterList")
    open fun onPersistAccountAddressPoolEntry(
        walletId: ByteArray,
        accountTypeTag: Byte,
        accountStandardTag: Byte,
        accountIndex: Int,
        accountRegistrationIndex: Int,
        accountKeyClass: Int,
        accountUserIdentityId: ByteArray,
        accountFriendIdentityId: ByteArray,
        poolTypeTag: Byte,
        publicKey: ByteArray,
        hasPublicKey: Boolean,
        addressPoolTypeTag: Byte,
        addressIndex: Int,
        isUsed: Boolean,
        balance: Long,
        addressBase58: String,
        derivationPath: String,
    ): Int = 0

    // ── Wallet (core) changeset ───────────────────────────────────────

    /**
     * `on_persist_wallet_changeset_fn` chain / balance / chainlock header.
     * Fired once before the per-account decomposition. Descriptor
     * `([BZIZJJJJ[B)I`.
     */
    open fun onWalletChangesetHeader(
        walletId: ByteArray,
        hasSyncedHeight: Boolean,
        syncedHeight: Int,
        hasBalance: Boolean,
        confirmedDelta: Long,
        unconfirmedDelta: Long,
        immatureDelta: Long,
        lockedDelta: Long,
        lastAppliedChainLockBytes: ByteArray,
    ): Int = 0

    /**
     * One `AccountChangeSetFFI` — the account row itself. UTXO adds/spends
     * and transactions follow via the calls below, then
     * [onWalletChangesetAccountEnd]. Descriptor
     * `([BIBBII[B[BIZIZ)I`.
     */
    open fun onWalletChangesetAccountBegin(
        walletId: ByteArray,
        accountIndex: Int,
        typeTag: Byte,
        standardTag: Byte,
        registrationIndex: Int,
        keyClass: Int,
        userIdentityId: ByteArray,
        friendIdentityId: ByteArray,
        externalHighestUsed: Int,
        hasExternalHighestUsed: Boolean,
        internalHighestUsed: Int,
        hasInternalHighestUsed: Boolean,
    ): Int = 0

    /** One `UtxoEntryFFI` added on the current account. Descriptor `([B[BIJLjava/lang/String;[BIZZZZ)I`. */
    @Suppress("LongParameterList")
    open fun onWalletChangesetUtxoAdded(
        walletId: ByteArray,
        txid: ByteArray,
        vout: Int,
        amount: Long,
        address: String,
        scriptPubKey: ByteArray,
        height: Int,
        isCoinbase: Boolean,
        isConfirmed: Boolean,
        isInstantLocked: Boolean,
        isLocked: Boolean,
    ): Int = 0

    /** One `SpentOutPointFFI` on the current account. Descriptor `([B[BI[B)I`. */
    open fun onWalletChangesetUtxoSpent(
        walletId: ByteArray,
        txid: ByteArray,
        vout: Int,
        spendingTxid: ByteArray,
    ): Int = 0

    /** One `TransactionRecordFFI` on the current account. Descriptor `([B[B[BII[BIILjava/lang/String;IJJZLjava/lang/String;J)I`. */
    @Suppress("LongParameterList")
    open fun onWalletChangesetTransaction(
        walletId: ByteArray,
        txid: ByteArray,
        txData: ByteArray,
        context: Int,
        blockHeight: Int,
        blockHash: ByteArray,
        blockTimestamp: Int,
        direction: Int,
        transactionType: String,
        transactionTypeKind: Int,
        netAmount: Long,
        fee: Long,
        hasFee: Boolean,
        label: String,
        firstSeen: Long,
    ): Int = 0

    /** Close the current account bucket. Descriptor `([BI)I`. */
    open fun onWalletChangesetAccountEnd(walletId: ByteArray, accountIndex: Int): Int = 0

    // ── Identities ────────────────────────────────────────────────────

    /**
     * One `IdentityEntryFFI` upsert. DPNS labels + acquired-at timestamps
     * ride as parallel arrays. Descriptor
     * `([B[BJJZIBZ[B[Ljava/lang/String;[JZLjava/lang/String;Ljava/lang/String;Ljava/lang/String;[BZ[BZLjava/lang/String;)I`.
     */
    @Suppress("LongParameterList")
    open fun onPersistIdentityUpsert(
        walletId: ByteArray,
        identityId: ByteArray,
        balance: Long,
        revision: Long,
        identityIndexIsSome: Boolean,
        identityIndex: Int,
        status: Byte,
        walletIdIsSome: Boolean,
        identityWalletId: ByteArray,
        dpnsNames: Array<String>,
        dpnsNamesAcquiredAt: LongArray,
        dashpayProfilePresent: Boolean,
        dashpayDisplayName: String?,
        dashpayBio: String?,
        dashpayAvatarUrl: String?,
        dashpayAvatarHash: ByteArray,
        dashpayAvatarHashPresent: Boolean,
        dashpayAvatarFingerprint: ByteArray,
        dashpayAvatarFingerprintPresent: Boolean,
        dashpayPublicMessage: String?,
    ): Int = 0

    /** One identity-id removal. Descriptor `([B[B)I`. */
    open fun onPersistIdentityRemoval(walletId: ByteArray, identityId: ByteArray): Int = 0

    // ── Identity keys ─────────────────────────────────────────────────

    /** One `IdentityKeyEntryFFI` upsert. Descriptor `([B[BIBBBZZJ[B[BZ[BZIIB[BLjava/lang/String;)I`. */
    @Suppress("LongParameterList")
    open fun onPersistIdentityKeyUpsert(
        walletId: ByteArray,
        identityId: ByteArray,
        keyId: Int,
        purpose: Byte,
        securityLevel: Byte,
        keyType: Byte,
        readOnly: Boolean,
        disabledAtIsSome: Boolean,
        disabledAt: Long,
        publicKeyData: ByteArray,
        publicKeyHash: ByteArray,
        walletIdIsSome: Boolean,
        keyWalletId: ByteArray,
        derivationIndicesIsSome: Boolean,
        identityIndex: Int,
        keyIndex: Int,
        contractBoundsKind: Byte,
        contractBoundsId: ByteArray,
        contractBoundsDocumentType: String?,
    ): Int = 0

    /** One `(identityId, keyId)` removal. Descriptor `([B[BI)I`. */
    open fun onPersistIdentityKeyRemoval(walletId: ByteArray, identityId: ByteArray, keyId: Int): Int = 0

    // ── Token balances ────────────────────────────────────────────────

    /** One `TokenBalanceUpsertFFI`. Descriptor `([B[B[BJ)I`. */
    open fun onPersistTokenBalanceUpsert(
        walletId: ByteArray,
        identityId: ByteArray,
        tokenId: ByteArray,
        balance: Long,
    ): Int = 0

    /** One `TokenBalanceRemovalFFI`. Descriptor `([B[B[B)I`. */
    open fun onPersistTokenBalanceRemoval(
        walletId: ByteArray,
        identityId: ByteArray,
        tokenId: ByteArray,
    ): Int = 0

    // ── Contacts ──────────────────────────────────────────────────────

    /**
     * One `ContactRequestFFI` upsert. Descriptor
     * `([B[B[BZIII[B[B[BIJZLjava/lang/String;Ljava/lang/String;ZLjava/lang/String;[I)I`.
     *
     * The tail block ([paymentChannelBroken] / [alias] / [note] /
     * [isHidden] / [contactAccountLabel] / [acceptedAccounts]) is
     * established-row relationship metadata (contactInfo + DIP-15
     * accepted accounts) — null / false / empty on pending rows.
     * [acceptedAccounts] is never null (empty when absent).
     */
    @Suppress("LongParameterList")
    open fun onPersistContactUpsert(
        walletId: ByteArray,
        ownerId: ByteArray,
        contactId: ByteArray,
        isOutgoing: Boolean,
        senderKeyIndex: Int,
        recipientKeyIndex: Int,
        accountReference: Int,
        encryptedPublicKey: ByteArray,
        encryptedAccountLabel: ByteArray?,
        autoAcceptProof: ByteArray?,
        coreHeightCreatedAt: Int,
        createdAt: Long,
        paymentChannelBroken: Boolean,
        alias: String?,
        note: String?,
        isHidden: Boolean,
        contactAccountLabel: String?,
        acceptedAccounts: IntArray,
    ): Int = 0

    /** One sent-side `ContactRequestRemovalFFI`. Descriptor `([B[B[B)I`. */
    open fun onPersistContactRemovalSent(
        walletId: ByteArray,
        ownerId: ByteArray,
        contactId: ByteArray,
    ): Int = 0

    /** One incoming-side `ContactRequestRemovalFFI`. Descriptor `([B[B[B)I`. */
    open fun onPersistContactRemovalIncoming(
        walletId: ByteArray,
        ownerId: ByteArray,
        contactId: ByteArray,
    ): Int = 0

    /**
     * One `ContactIgnoredSenderFFI` per-sender ignore delta. Descriptor
     * `([B[B[BZ)I`.
     *
     * [isIgnored] `true` ⇒ persist the ignored-sender row (an ignore —
     * also drop every incoming request row from that sender, rotations
     * included); `false` ⇒ delete it (an un-ignore). Ignore is a
     * local-only per-sender mute keyed `(ownerId, senderId)`.
     */
    open fun onPersistContactIgnored(
        walletId: ByteArray,
        ownerId: ByteArray,
        senderId: ByteArray,
        isIgnored: Boolean,
    ): Int = 0

    /**
     * One `ContactProfileRowFFI` delta riding an identity upsert
     * (`IdentityEntryFFI.contact_profiles`). Descriptor
     * `([B[B[BZLjava/lang/String;Ljava/lang/String;Ljava/lang/String;[BZ[BZLjava/lang/String;J)I`.
     *
     * [isPresent] `true` ⇒ upsert the cached contact-profile row for
     * `(ownerId, contactId)`; `false` ⇒ tombstone — the contact removed
     * their on-chain profile and any persisted row must be DELETED (an
     * upsert-only pipeline would show the stale name/avatar forever).
     * Gate [avatarHash] / [avatarFingerprint] on their paired `_present`
     * flags — all-zero is a valid hash value.
     */
    @Suppress("LongParameterList")
    open fun onPersistContactProfileDelta(
        walletId: ByteArray,
        ownerId: ByteArray,
        contactId: ByteArray,
        isPresent: Boolean,
        displayName: String?,
        bio: String?,
        avatarUrl: String?,
        avatarHash: ByteArray,
        avatarHashPresent: Boolean,
        avatarFingerprint: ByteArray,
        avatarFingerprintPresent: Boolean,
        publicMessage: String?,
        checkedAtMs: Long,
    ): Int = 0

    // ── Asset locks ───────────────────────────────────────────────────

    /** One `AssetLockEntryFFI` upsert. Descriptor `([B[B[BIBIJB[B)I`. */
    @Suppress("LongParameterList")
    open fun onPersistAssetLockUpsert(
        walletId: ByteArray,
        outPoint: ByteArray,
        transactionBytes: ByteArray,
        accountIndex: Int,
        fundingType: Byte,
        identityIndex: Int,
        amountDuffs: Long,
        status: Byte,
        proofBytes: ByteArray?,
    ): Int = 0

    /** One 36-byte outpoint removal. Descriptor `([B[B)I`. */
    open fun onPersistAssetLockRemoval(walletId: ByteArray, outPoint: ByteArray): Int = 0

    // ── Shielded persist ──────────────────────────────────────────────

    /** One `ShieldedNoteFFI`. Descriptor `([B[BIJ[B[BJBJ[B)I`. */
    @Suppress("LongParameterList")
    open fun onPersistShieldedNote(
        walletId: ByteArray,
        noteWalletId: ByteArray,
        accountIndex: Int,
        position: Long,
        cmx: ByteArray,
        nullifier: ByteArray,
        blockHeight: Long,
        isSpent: Byte,
        value: Long,
        noteData: ByteArray,
    ): Int = 0

    /** One `ShieldedNullifierSpentFFI`. Descriptor `([B[BI[B)I`. */
    open fun onPersistShieldedNullifierSpent(
        walletId: ByteArray,
        noteWalletId: ByteArray,
        accountIndex: Int,
        nullifier: ByteArray,
    ): Int = 0

    /** One `ShieldedOutgoingNoteFFI`. Descriptor `([B[BI[B[BJJ[B)I`. */
    @Suppress("LongParameterList")
    open fun onPersistShieldedOutgoingNote(
        walletId: ByteArray,
        noteWalletId: ByteArray,
        accountIndex: Int,
        cmx: ByteArray,
        recipient: ByteArray,
        value: Long,
        blockHeight: Long,
        memo: ByteArray,
    ): Int = 0

    /** One `ShieldedSyncedIndexFFI`. Descriptor `([B[BIJ)I`. */
    open fun onPersistShieldedSyncedIndex(
        walletId: ByteArray,
        noteWalletId: ByteArray,
        accountIndex: Int,
        lastSyncedIndex: Long,
    ): Int = 0

    /** One `ShieldedActivityFFI`. Descriptor `([B[BI[BBBBJJZJZJ[BZ[B[B[B[B)I`. */
    @Suppress("LongParameterList")
    open fun onPersistShieldedActivity(
        walletId: ByteArray,
        noteWalletId: ByteArray,
        accountIndex: Int,
        entryId: ByteArray,
        kindTag: Byte,
        direction: Byte,
        status: Byte,
        amount: Long,
        fee: Long,
        hasFee: Boolean,
        blockHeight: Long,
        hasBlockHeight: Boolean,
        createdAtMs: Long,
        identityId: ByteArray,
        hasIdentityId: Boolean,
        counterparty: ByteArray,
        memo: ByteArray,
        noteCmxs: ByteArray,
        spentNullifiers: ByteArray,
    ): Int = 0

    // ── Load callbacks ────────────────────────────────────────────────

    /**
     * `on_load_wallet_list_fn`. Returns the persisted wallet list as an
     * array of flat holders; the Rust trampoline re-packs each into a
     * `WalletRestoreEntryFFI` (plus nested arrays) in Rust-owned memory,
     * freed by the paired free callback. Descriptor
     * `()[Lorg/dashfoundation/dashsdk/ffi/WalletRestoreData;`.
     *
     * A minimal-but-correct restore populates `accounts` (with xpub
     * bytes), the platform / core sync watermarks, the wallet's
     * identities, and its cached platform-address balances; the
     * remaining nested arrays (utxos, tracked asset locks) are optional
     * for this milestone and default to empty.
     */
    open fun onLoadWalletList(): Array<WalletRestoreData> = emptyArray()

    /** `on_load_shielded_notes_fn`. Descriptor `()[Lorg/dashfoundation/dashsdk/ffi/ShieldedNoteData;`. */
    open fun onLoadShieldedNotes(): Array<ShieldedNoteData> = emptyArray()

    /** `on_load_shielded_outgoing_notes_fn`. Descriptor `()[Lorg/dashfoundation/dashsdk/ffi/ShieldedOutgoingNoteData;`. */
    open fun onLoadShieldedOutgoingNotes(): Array<ShieldedOutgoingNoteData> = emptyArray()

    /** `on_load_shielded_sync_states_fn`. Descriptor `()[Lorg/dashfoundation/dashsdk/ffi/ShieldedSyncStateData;`. */
    open fun onLoadShieldedSyncStates(): Array<ShieldedSyncStateData> = emptyArray()

    /** `on_load_shielded_activity_fn`. Descriptor `()[Lorg/dashfoundation/dashsdk/ffi/ShieldedActivityData;`. */
    open fun onLoadShieldedActivity(): Array<ShieldedActivityData> = emptyArray()

    /**
     * `on_get_core_tx_record_fn`. Returns the record for `txid` or `null`
     * if no row exists. Descriptor
     * `([B[B)Lorg/dashfoundation/dashsdk/ffi/CoreTxRecordData;`.
     */
    open fun onGetCoreTxRecord(walletId: ByteArray, txid: ByteArray): CoreTxRecordData? = null
}

// ── Flat data holders for the load callbacks ──────────────────────────
//
// These are plain value classes the Rust trampolines read field-by-field
// via GetFieldID; keeping them primitive keeps the JNI reflection cheap
// and the native re-pack allocation-free on the Kotlin side.

/**
 * Minimal wallet-restore row. Mirrors the subset of
 * `WalletRestoreEntryFFI` this milestone rehydrates: identity of the
 * wallet, its accounts, and the sync watermarks.
 *
 * @param accountSpecs each `AccountSpecData` becomes one `AccountSpecFFI`
 */
class WalletRestoreData(
    @JvmField val walletId: ByteArray,
    /** FFINetwork ordinal (0 Mainnet, 1 Testnet, 2 Devnet, 3 Regtest). */
    @JvmField val network: Int,
    @JvmField val accountSpecs: Array<AccountSpecData>,
    @JvmField val platformSyncHeight: Long,
    @JvmField val platformSyncTimestamp: Long,
    @JvmField val platformLastKnownRecentBlock: Long,
    @JvmField val birthHeight: Int,
    @JvmField val syncedHeight: Int,
    @JvmField val lastProcessedHeight: Int,
    @JvmField val lastSynced: Long,
    /**
     * Per-wallet identities to rehydrate into the wallet's
     * `IdentityManager` (the `wallet_identities[wallet_id]` bucket), each
     * carrying its public keys. Empty when the wallet has no persisted
     * identities. The Rust trampoline re-packs each into an
     * `IdentityRestoreEntryFFI` (plus a nested `IdentityKeyRestoreFFI`
     * array) so the restored `Identity.public_keys` map is populated on
     * cold start — without this every Platform write is rejected at DPP
     * validation for a keyless identity (mirror of the Swift
     * `buildIdentityRestoreBuffer`).
     */
    @JvmField val identities: Array<IdentityRestoreData>,
    /**
     * Cached platform-address balances for this wallet, re-seeding the
     * Rust provider's per-account balance map on cold start so the next
     * BLAST sync resumes from the persisted `(balance, as_of_height)` pin
     * instead of an empty found map. Empty when the wallet has no
     * persisted platform-address rows. The Rust trampoline re-packs each
     * into an `AddressBalanceEntryFFI`.
     *
     * Without this, a persisted credit whose height is at or below the
     * trusted sync watermark is never rehydrated: the tree-scan re-pins
     * the address at the checkpoint height, the incremental delta at the
     * same height is gated off by the ADDR-09 height pin
     * (`op_height <= as_of_height`), and the balance stays 0 across
     * relaunches (SH-06). Mirror of the Swift
     * `loadCachedBalances` → `AddressBalanceEntryFFI` buffer path.
     */
    @JvmField val platformAddressBalances: Array<PlatformAddressBalanceRestoreData>,
)

/**
 * One flat cached platform-address balance row — mirror of
 * `AddressBalanceEntryFFI` (the `#4019` layout with `as_of_height`).
 *
 * [addressType] is the DIP-0018 discriminant (0 = P2PKH, 1 = P2SH); the
 * Rust load path currently rehydrates P2PKH (0) only and skip-warns other
 * types. [addressHash] is the 20-byte platform-address hash. [asOfHeight]
 * is the platform block height [balance] is current as of — the ADDR-09
 * height pin (from the persisted `lastSeenHeight`); it MUST round-trip
 * faithfully (a reset to 0 would re-open the double-count gate the pin
 * closes). A persisted `0` means "unknown provenance" and self-heals by
 * yielding to the first pinned absolute on the next sync.
 */
class PlatformAddressBalanceRestoreData(
    @JvmField val addressType: Byte,
    @JvmField val addressHash: ByteArray,
    @JvmField val balance: Long,
    @JvmField val nonce: Int,
    @JvmField val accountIndex: Int,
    @JvmField val addressIndex: Int,
    @JvmField val asOfHeight: Long,
)

/** One flat account spec — mirror of `AccountSpecFFI`. */
class AccountSpecData(
    @JvmField val typeTag: Byte,
    @JvmField val standardTag: Byte,
    @JvmField val index: Int,
    @JvmField val registrationIndex: Int,
    @JvmField val keyClass: Int,
    @JvmField val userIdentityId: ByteArray,
    @JvmField val friendIdentityId: ByteArray,
    /** bincode `ExtendedPubKey`; empty when unavailable. */
    @JvmField val accountXpubBytes: ByteArray,
)

/**
 * One flat identity-restore row — mirror of `IdentityRestoreEntryFFI`.
 *
 * DPNS names ride separately (empty for this pass; the Rust trampoline
 * passes null/0). `status` uses the `IdentityStatus` discriminant encoding
 * (0 Unknown, 1 PendingCreation, 2 Active, 3 FailedCreation, 4 NotFound).
 */
class IdentityRestoreData(
    @JvmField val identityId: ByteArray,
    @JvmField val balance: Long,
    @JvmField val revision: Long,
    @JvmField val identityIndex: Int,
    @JvmField val status: Byte,
    @JvmField val keys: Array<IdentityKeyRestoreData>,
    /**
     * DashPay contact rows (pending + established, with their contactInfo
     * metadata) to rehydrate the Rust contact state at load — without
     * this, contacts only re-derive from chain on the first sync sweep
     * and the owner-private metadata is wiped during the DIP-15
     * deferred-publish window. Mirror of the Swift
     * `buildIdentityRestoreBuffer` contact block.
     */
    @JvmField val contacts: Array<ContactRequestRestoreData>,
    /**
     * 32-byte ids of ignored senders (per-sender mute, local-only) to
     * rehydrate the Rust `ignored_senders` set at load — without this a
     * previously-ignored sender's still-on-platform immutable
     * `contactRequest` documents re-ingest on the next sweep and the
     * sender resurfaces after every relaunch.
     */
    @JvmField val ignoredSenders: Array<ByteArray>,
    /**
     * DashPay payment-history rows to rehydrate the Rust
     * `dashpay_payments` map at load — without this *Sent* entries (and
     * their user-entered memos) vanish on every relaunch; the reconcile
     * sweep can only re-derive *Received* entries from UTXOs.
     */
    @JvmField val payments: Array<PaymentRestoreData>,
    /**
     * Cached contact-profile rows (present profiles only — tombstones
     * delete the Room row at persist time) to rehydrate the Rust
     * `contact_profiles` map at load — without this the contacts UI
     * shows raw identity ids after relaunch until the next profile sweep
     * re-fetches every contact.
     */
    @JvmField val contactProfiles: Array<ContactProfileRestoreData>,
)

/**
 * One flat DashPay payment-history restore row — mirror of
 * `PaymentRestoreEntryFFI`. `directionRaw`: 0 Sent, 1 Received;
 * `statusRaw`: 0 Pending, 1 Confirmed, 2 Failed. [memo] null mirrors the
 * source `Option` being `None`.
 */
class PaymentRestoreData(
    @JvmField val txid: String,
    @JvmField val counterpartyId: ByteArray,
    @JvmField val amountDuffs: Long,
    @JvmField val directionRaw: Byte,
    @JvmField val statusRaw: Byte,
    @JvmField val memo: String?,
)

/**
 * One flat cached contact-profile restore row — mirror of
 * `ContactProfileRestoreEntryFFI`. [avatarHash] / [avatarFingerprint]
 * are null when absent (the trampoline derives the FFI `_present` flags
 * from nullability + length: 32 / 8 bytes respectively).
 */
class ContactProfileRestoreData(
    @JvmField val contactId: ByteArray,
    @JvmField val displayName: String?,
    @JvmField val bio: String?,
    @JvmField val avatarUrl: String?,
    @JvmField val avatarHash: ByteArray?,
    @JvmField val avatarFingerprint: ByteArray?,
    @JvmField val publicMessage: String?,
    @JvmField val checkedAtMs: Long,
)

/**
 * One flat DashPay contact-request restore row — mirror of the
 * `ContactRequestFFI` rows carried on `IdentityRestoreEntryFFI.contacts`
 * (and of [NativePersistenceBridge.onPersistContactUpsert]'s parameter
 * list, whose Room rows feed this back).
 *
 * [encryptedAccountLabel] / [autoAcceptProof] use null for absent (the
 * trampoline maps null/empty back to `(null, 0)`); the metadata strings
 * use null for unset; [acceptedAccounts] is empty when absent.
 */
class ContactRequestRestoreData(
    @JvmField val ownerIdentityId: ByteArray,
    @JvmField val contactIdentityId: ByteArray,
    @JvmField val isOutgoing: Boolean,
    @JvmField val senderKeyIndex: Int,
    @JvmField val recipientKeyIndex: Int,
    @JvmField val accountReference: Int,
    @JvmField val encryptedPublicKey: ByteArray,
    @JvmField val encryptedAccountLabel: ByteArray?,
    @JvmField val autoAcceptProof: ByteArray?,
    @JvmField val coreHeightCreatedAt: Int,
    @JvmField val createdAtMillis: Long,
    @JvmField val paymentChannelBroken: Boolean,
    @JvmField val alias: String?,
    @JvmField val note: String?,
    @JvmField val isHidden: Boolean,
    @JvmField val contactAccountLabel: String?,
    @JvmField val acceptedAccounts: IntArray,
)

/**
 * One flat identity-public-key row — mirror of `IdentityKeyRestoreFFI`.
 *
 * `keyType` / `purpose` / `securityLevel` are DPP `repr(u8)` discriminants
 * (out-of-range = 255 sentinel → Rust drops the row rather than coercing to
 * MASTER/AUTHENTICATION, matching the Swift loader's `UInt8.max` fallback).
 * `contractBoundsKind`: 0 none, 1 SingleContract, 2 SingleContractDocumentType;
 * `contractBoundsId` is 32 bytes (or empty for kind 0);
 * `contractBoundsDocumentType` is non-null only for kind 2.
 */
class IdentityKeyRestoreData(
    @JvmField val keyId: Int,
    @JvmField val keyType: Byte,
    @JvmField val purpose: Byte,
    @JvmField val securityLevel: Byte,
    @JvmField val readOnly: Boolean,
    @JvmField val data: ByteArray,
    @JvmField val contractBoundsKind: Byte,
    @JvmField val contractBoundsId: ByteArray,
    @JvmField val contractBoundsDocumentType: String?,
)

/** Mirror of `ShieldedNoteRestoreFFI`. */
class ShieldedNoteData(
    @JvmField val walletId: ByteArray,
    @JvmField val accountIndex: Int,
    @JvmField val position: Long,
    @JvmField val cmx: ByteArray,
    @JvmField val nullifier: ByteArray,
    @JvmField val blockHeight: Long,
    @JvmField val isSpent: Byte,
    @JvmField val value: Long,
    @JvmField val noteData: ByteArray,
)

/** Mirror of `ShieldedOutgoingNoteRestoreFFI`. */
class ShieldedOutgoingNoteData(
    @JvmField val walletId: ByteArray,
    @JvmField val accountIndex: Int,
    @JvmField val cmx: ByteArray,
    @JvmField val recipient: ByteArray,
    @JvmField val value: Long,
    @JvmField val blockHeight: Long,
    @JvmField val memo: ByteArray,
)

/** Mirror of `ShieldedSubwalletSyncStateFFI`. */
class ShieldedSyncStateData(
    @JvmField val walletId: ByteArray,
    @JvmField val accountIndex: Int,
    @JvmField val lastSyncedIndex: Long,
)

/** Mirror of `ShieldedActivityRestoreFFI`. */
class ShieldedActivityData(
    @JvmField val walletId: ByteArray,
    @JvmField val accountIndex: Int,
    @JvmField val entryId: ByteArray,
    @JvmField val kindTag: Byte,
    @JvmField val direction: Byte,
    @JvmField val status: Byte,
    @JvmField val amount: Long,
    @JvmField val fee: Long,
    @JvmField val hasFee: Boolean,
    @JvmField val blockHeight: Long,
    @JvmField val hasBlockHeight: Boolean,
    @JvmField val createdAtMs: Long,
    @JvmField val identityId: ByteArray,
    @JvmField val hasIdentityId: Boolean,
    @JvmField val counterparty: ByteArray,
    @JvmField val memo: ByteArray,
    @JvmField val noteCmxs: ByteArray,
    @JvmField val spentNullifiers: ByteArray,
)

/**
 * Mirror of the `on_get_core_tx_record_fn` output triad. `contextKind`
 * uses the `TransactionContext` discriminants (0 Mempool, 1 InstantSend,
 * 2 InBlock, 3 InChainLockedBlock); block fields are meaningful only for
 * kinds 2 and 3.
 */
class CoreTxRecordData(
    @JvmField val contextKind: Byte,
    @JvmField val blockHeight: Int,
    @JvmField val blockHash: ByteArray,
    @JvmField val blockTimestamp: Int,
    /** Raw transaction bytes, or empty if the row exists without them. */
    @JvmField val txBytes: ByteArray,
)
