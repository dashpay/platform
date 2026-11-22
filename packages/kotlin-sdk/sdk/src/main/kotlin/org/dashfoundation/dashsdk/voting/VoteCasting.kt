package org.dashfoundation.dashsdk.voting

import org.dashfoundation.dashsdk.wallet.op

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.TransactionsNative

/**
 * A contested-resource vote choice — Kotlin mirror of Swift's
 * `ContestedResourceVoteChoice`. The [discriminant] matches the FFI's
 * `vote_choice` byte (0 TowardsIdentity, 1 Abstain, 2 Lock).
 */
sealed class VoteChoice(val discriminant: Int) {
    /** Vote for a specific contender (base58 identity id). */
    data class TowardsIdentity(val contenderIdentityId: String) : VoteChoice(0)

    /** Abstain from the contest. */
    data object Abstain : VoteChoice(1)

    /** Vote that nobody should win the contested resource. */
    data object Lock : VoteChoice(2)
}

/**
 * Masternode contested-resource vote bridge — port of the vote-casting slice
 * of `SDK.castContestedResourceVote` (driven by Swift `ContestDetailView`).
 * Thin wrapper over [TransactionsNative]; no orchestration lives here (see
 * `packages/kotlin-sdk/CLAUDE.md`). The SDK handle is supplied by the caller
 * (`Sdk.handle`); the ECDSA_HASH160 voting key and signer are derived
 * Rust-side from [votingPrivateKey], so the key bytes are not stored.
 */
class VoteCasting internal constructor() {

    /**
     * Cast a masternode contested-resource vote and wait for the response —
     * mirrors Swift `SDK.castContestedResourceVote`'s parameter order.
     *
     * Builds a `ResourceVote` over the vote poll `(dataContractId,
     * documentTypeName, indexName, indexValues)` with [choice], signs it with
     * the masternode voting key derived from [votingPrivateKey], and
     * broadcasts it for [proTxHash].
     *
     * A vote from a non-masternode wallet is expected to be rejected by
     * Platform with an authorization-style error — the correct deterministic
     * outcome (surfaced as a thrown exception).
     *
     * @param indexValues Index values (DPNS: `["dash", label]`), read
     *   Rust-side in this order: a value starting with a backslash (`\`) is
     *   text with that one backslash removed; else a `0x`-prefixed value is
     *   hex bytes; else the value is text. Encode text with
     *   [ContestedIndexValue.text] (a DPNS label normalized to `0xf0rd` must
     *   be sent as `\0xf0rd`) and bytes with [ContestedIndexValue.bytes].
     * @param proTxHash 32-byte masternode pro_tx_hash in **wire order** — the
     *   orientation `Txid` stores, which is what a parsed ProRegTx yields and
     *   what a wallet holds internally. NOT the byte order of the hex Core
     *   displays, which is its reverse.
     *
     *   Not interchangeable: Platform identifies masternodes by the opposite
     *   orientation (`ProTxHash` is declared `#[hash_newtype(forward)]`,
     *   `Txid` is not), so the Rust side reverses these bytes before deriving
     *   the voter identity. Passing display order asks Platform for an
     *   identity that has never existed, and the vote is rejected as having no
     *   voter identity.
     * @param votingPrivateKey 32-byte masternode voting private key.
     * @param networkOrd `Network.ffiValue` (0 Mainnet, 1 Testnet, 2 Devnet,
     *   3 Regtest).
     */
    suspend fun castVote(
        sdk: org.dashfoundation.dashsdk.Sdk,
        dataContractId: String,
        documentTypeName: String,
        indexName: String,
        indexValues: List<String>,
        choice: VoteChoice,
        proTxHash: ByteArray,
        votingPrivateKey: ByteArray,
        networkOrd: Int,
    ) = sdk.queryGate.op {
        require(proTxHash.size == 32) { "proTxHash must be 32 bytes, got ${proTxHash.size}" }
        require(votingPrivateKey.size == 32) {
            "votingPrivateKey must be 32 bytes, got ${votingPrivateKey.size}"
        }
        val contender = (choice as? VoteChoice.TowardsIdentity)?.contenderIdentityId
        mapNativeErrors {
            TransactionsNative.castContestedResourceVote(
                sdk.handle,
                dataContractId,
                documentTypeName,
                indexName,
                encodeIndexValuesJson(indexValues),
                choice.discriminant,
                contender,
                proTxHash,
                votingPrivateKey,
                networkOrd,
            )
        }
    }

    /**
     * Cast a masternode contested-resource vote signed by a host signer that
     * holds the voting key, and wait for the response: the vote of the
     * `votingPrivateKey` overload, for a key the app stores
     * ([org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore]) rather
     * than one pasted for the call. Port of Swift
     * `SDK.castContestedResourceVote(..., votingAddress:, signer:)`
     * (`PlatformQueryExtensions.swift`).
     *
     * Rust builds the ECDSA_HASH160 voting key from [votingAddress] and asks
     * the signer to sign with it, so the signer receives the 20 address bytes
     * with key type ECDSA_HASH160. A signer that holds no key for that
     * address fails the call with the signer's error.
     *
     * @param indexValues as for the `votingPrivateKey` overload.
     * @param proTxHash 32-byte masternode pro_tx_hash in **wire order**, as
     *   for the `votingPrivateKey` overload.
     * @param votingAddress 20-byte voting address: the hash160 of the
     *   compressed voting public key
     *   ([org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore.votingAddressOf]).
     * @param signerHandle the native handle of the signer that holds the
     *   voting key, `PlatformWalletManager.signerHandle`. The caller keeps the
     *   owning signer (its manager) open until this returns.
     */
    suspend fun castVote(
        sdk: org.dashfoundation.dashsdk.Sdk,
        dataContractId: String,
        documentTypeName: String,
        indexName: String,
        indexValues: List<String>,
        choice: VoteChoice,
        proTxHash: ByteArray,
        votingAddress: ByteArray,
        signerHandle: Long,
    ) = sdk.queryGate.op {
        require(proTxHash.size == 32) { "proTxHash must be 32 bytes, got ${proTxHash.size}" }
        require(votingAddress.size == 20) {
            "votingAddress must be 20 bytes, got ${votingAddress.size}"
        }
        val contender = (choice as? VoteChoice.TowardsIdentity)?.contenderIdentityId
        mapNativeErrors {
            TransactionsNative.castContestedResourceVoteWithSigner(
                sdk.handle,
                dataContractId,
                documentTypeName,
                indexName,
                encodeIndexValuesJson(indexValues),
                choice.discriminant,
                contender,
                proTxHash,
                votingAddress,
                signerHandle,
            )
        }
    }
}
