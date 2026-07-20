package org.dashfoundation.dashsdk.identity

import java.nio.ByteBuffer

/**
 * One identity-registration-key preview row — port of the iOS
 * `IdentityKeyPreviewFFI` surface. Carries the derivation path, the
 * compressed public key, and the raw private-key scalar the Rust side
 * derived for a given identity slot.
 *
 * The private key is the one piece of key material the Kotlin layer is
 * allowed to touch (the Keychain/Keystore exception in
 * `packages/kotlin-sdk/CLAUDE.md`): callers persist it via
 * `WalletStorage.storePrivateKey` keyed by [publicKeyHex] and then drop
 * their reference. Do not log or retain [privateKey].
 */
data class IdentityKeyPreview(
    val identityIndex: Int,
    val derivationPath: String,
    val publicKey: ByteArray,
    val privateKey: ByteArray,
) {
    /** Lower-case hex of [publicKey] — the `WalletStorage` key-material id. */
    val publicKeyHex: String get() = publicKey.joinToString("") { "%02x".format(it) }

    // ByteArray fields need content-based equality for test assertions.
    override fun equals(other: Any?): Boolean =
        other is IdentityKeyPreview &&
            identityIndex == other.identityIndex &&
            derivationPath == other.derivationPath &&
            publicKey.contentEquals(other.publicKey) &&
            privateKey.contentEquals(other.privateKey)

    override fun hashCode(): Int {
        var result = identityIndex
        result = 31 * result + derivationPath.hashCode()
        result = 31 * result + publicKey.contentHashCode()
        result = 31 * result + privateKey.contentHashCode()
        return result
    }

    companion object {
        /**
         * Decode the flat BLOB produced by
         * `IdentityNative.previewRegistrationKeys`. Layout (big-endian):
         * `u32 rowCount` then per row `u32 identityIndex, u16 pathLen,
         * pathUtf8, u8[33] pubkey, u8[32] privkey`.
         */
        fun decodeAll(blob: ByteArray): List<IdentityKeyPreview> {
            val buf = ByteBuffer.wrap(blob) // big-endian by default
            val rows = ArrayList<IdentityKeyPreview>()
            try {
                val count = buf.int
                require(count >= 0) { "preview row count must be non-negative, got $count" }
                repeat(count) {
                    val identityIndex = buf.int
                    val pathLen = buf.short.toInt() and 0xFFFF
                    val pathBytes = ByteArray(pathLen)
                    buf.get(pathBytes)
                    val pubkey = ByteArray(33)
                    buf.get(pubkey)
                    val privkey = ByteArray(32)
                    var rowOwnsPrivateKey = false
                    try {
                        buf.get(privkey)
                        rows.add(
                            IdentityKeyPreview(
                                identityIndex = identityIndex,
                                derivationPath = String(pathBytes, Charsets.UTF_8),
                                publicKey = pubkey,
                                privateKey = privkey,
                            ),
                        )
                        rowOwnsPrivateKey = true
                    } finally {
                        // If construction/list insertion fails after the scalar
                        // copy, it never reaches the outer rows cleanup.
                        if (!rowOwnsPrivateKey) privkey.fill(0)
                    }
                }
                require(!buf.hasRemaining()) {
                    "preview blob has ${buf.remaining()} trailing byte(s)"
                }
                return rows
            } catch (e: Throwable) {
                // Some rows may already have been copied out of the interleaved
                // blob when a later row fails. They will never reach the caller,
                // so scrub those discarded scalar arrays here.
                rows.forEach { it.privateKey.fill(0) }
                throw e
            } finally {
                // The source blob interleaves every row's raw private scalar.
                // Wipe it on success and on every malformed/truncated failure;
                // otherwise an exception before the old tail-only fill left all
                // native-returned scalars resident in a JVM byte array.
                blob.fill(0)
            }
        }
    }
}
