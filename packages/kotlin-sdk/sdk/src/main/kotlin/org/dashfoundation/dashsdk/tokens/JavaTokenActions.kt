package org.dashfoundation.dashsdk.tokens

import java.math.BigInteger

/**
 * Unmangled Java entry points for every token action carrying a protocol
 * `u64`. Values are validated across the complete unsigned 64-bit domain,
 * converted to [ULong], and then delegated to [Tokens], which preserves the
 * existing raw-bit `jlong` JNI ABI.
 */
class JavaTokenActions internal constructor(private val tokens: Tokens) {

    suspend fun mint(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        amount: BigInteger,
        issuedToIdentityId: ByteArray?,
        publicNote: String?,
        groupAction: GroupAction,
        signingKeyId: Int,
        signer: Long,
    ): String? = tokens.mint(
        identityId, tokenContractId, tokenPosition, amount.toProtocolULong(),
        issuedToIdentityId, publicNote, groupAction, signingKeyId, signer,
    )

    suspend fun burn(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        amount: BigInteger,
        publicNote: String?,
        groupAction: GroupAction,
        signingKeyId: Int,
        signer: Long,
    ): String? = tokens.burn(
        identityId, tokenContractId, tokenPosition, amount.toProtocolULong(),
        publicNote, groupAction, signingKeyId, signer,
    )

    suspend fun transfer(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        recipientId: ByteArray,
        amount: BigInteger,
        publicNote: String?,
        signingKeyId: Int,
        signer: Long,
    ): String? = tokens.transfer(
        identityId, tokenContractId, tokenPosition, recipientId,
        amount.toProtocolULong(), publicNote, signingKeyId, signer,
    )

    suspend fun setPrice(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        pricePerToken: BigInteger,
        publicNote: String?,
        groupAction: GroupAction,
        signingKeyId: Int,
        signer: Long,
    ) = tokens.setPrice(
        identityId, tokenContractId, tokenPosition, pricePerToken.toProtocolULong(),
        publicNote, groupAction, signingKeyId, signer,
    )

    suspend fun purchase(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        amount: BigInteger,
        expectedTotalCost: BigInteger,
        signingKeyId: Int,
        signer: Long,
    ) = tokens.purchase(
        identityId, tokenContractId, tokenPosition, amount.toProtocolULong(),
        expectedTotalCost.toProtocolULong(), signingKeyId, signer,
    )

    suspend fun updateMaxSupply(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        newMaxSupply: BigInteger?,
        publicNote: String?,
        groupAction: GroupAction,
        signingKeyId: Int,
        signer: Long,
    ) = tokens.updateConfig(
        identityId,
        tokenContractId,
        tokenPosition,
        TokenConfigChange.MaxSupply(newMaxSupply?.toProtocolULong()),
        publicNote,
        groupAction,
        signingKeyId,
        signer,
    )
}
