package org.dashfoundation.dashsdk.identity

import java.io.ByteArrayOutputStream
import java.io.DataOutputStream

/**
 * Shared big-endian encoder for the identity add/register public-key BLOB.
 *
 * One wire format is decoded on the Rust side by
 * `rs-unified-sdk-jni::pubkey_rows::parse_pubkey_rows`, reused by BOTH:
 * - the identity-UPDATE add-key path (`TransactionsNative.updateIdentity`),
 *   driven by [IdentityUpdates], and
 * - every identity-CREATE registration path, driven by
 *   [IdentityRegistration] and `PlatformWalletManager.shieldedIdentityCreateFromPool`.
 *
 * Centralising the encoder here keeps the two paths byte-for-byte identical —
 * the registration wire format is now the *same* rich layout the update path
 * already used, so a key's DPP role (type / purpose / security level) and any
 * contract bounds ride the wire rather than being reconstructed positionally
 * on the Rust side.
 *
 * Layout (all integers big-endian), one row per key:
 * ```text
 * u32 rowCount
 * repeat rowCount times:
 *   u32 keyId
 *   u8  keyType          (DPP KeyType discriminant, 0 = ECDSA_SECP256K1)
 *   u8  purpose          (DPP Purpose discriminant, 0 = AUTHENTICATION)
 *   u8  securityLevel    (DPP SecurityLevel discriminant, 0 = MASTER)
 *   u8  readOnly         (0 / 1)
 *   u8  contractBoundsKind (0 none, 1 SingleContract, 2 SingleContractDocumentType,
 *                           3 ContractGroup)
 *   u16 pubkeyLen
 *   u8[pubkeyLen] pubkeyBytes  (compressed pubkey, or 20-byte HASH160)
 *   if contractBoundsKind != 0:
 *     u8[32] contractBoundsId  (contract id, or contract group id for kind 3)
 *   if contractBoundsKind == 2:
 *     u16 docTypeLen, u8[docTypeLen] docType (UTF-8)
 *   u8  limitsFlags        (bit 0: totalBudget follows, bit 1: expiresAt follows)
 *   if limitsFlags & 1:
 *     u64 totalBudget      (credits the key may spend over its lifetime)
 *   if limitsFlags & 2:
 *     u64 expiresAt        (block time in ms from which the key can no longer sign)
 * ```
 * Kind 3 carries the id only, never a document type.
 */
object IdentityPubkeyCodec {

    /** Encode [keys] into the add/register public-key blob documented above. */
    fun encode(keys: List<IdentityPubkey>): ByteArray {
        val out = ByteArrayOutputStream()
        val dos = DataOutputStream(out)
        dos.writeInt(keys.size)
        for (k in keys) {
            dos.writeInt(k.keyId)
            dos.writeByte(k.keyType.ffiValue)
            dos.writeByte(k.purpose.ffiValue)
            dos.writeByte(k.securityLevel.ffiValue)
            dos.writeByte(if (k.readOnly) 1 else 0)
            dos.writeByte(contractBoundsKind(k.contractBounds))
            require(k.pubkeyBytes.size <= 0xFFFF) {
                "pubkeyBytes too large: ${k.pubkeyBytes.size}"
            }
            dos.writeShort(k.pubkeyBytes.size)
            dos.write(k.pubkeyBytes)
            when (val bounds = k.contractBounds) {
                null -> Unit
                is ContractBounds.SingleContract -> dos.write(bounds.contractId)
                is ContractBounds.SingleContractDocumentType -> {
                    dos.write(bounds.contractId)
                    val dt = bounds.documentTypeName.toByteArray(Charsets.UTF_8)
                    require(dt.size <= 0xFFFF) { "documentTypeName too large: ${dt.size}" }
                    dos.writeShort(dt.size)
                    dos.write(dt)
                }
                is ContractBounds.ContractGroup -> dos.write(bounds.contractGroupId)
            }
            // Usage limits (protocol version 14): flags first, then only the values set.
            val flags = (if (k.totalBudget != null) 1 else 0) or (if (k.expiresAt != null) 2 else 0)
            dos.writeByte(flags)
            k.totalBudget?.let { dos.writeLong(it) }
            k.expiresAt?.let { dos.writeLong(it) }
        }
        return out.toByteArray()
    }

    /**
     * Discriminant matching the FFI: 0 none, 1 SingleContract, 2 with doc type,
     * 3 ContractGroup.
     */
    internal fun contractBoundsKind(bounds: ContractBounds?): Int = when (bounds) {
        null -> 0
        is ContractBounds.SingleContract -> 1
        is ContractBounds.SingleContractDocumentType -> 2
        is ContractBounds.ContractGroup -> 3
    }
}
