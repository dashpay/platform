package org.dashfoundation.dashsdk.ffi

/** Thin JNI surface over `platform-wallet-ffi`'s DPNS marketplace APIs. */
internal object DpnsMarketplaceNative {
    init { NativeLoader.ensureLoaded() }

    external fun search(
        walletHandle: Long,
        prefix: String,
        limit: Int,
        startAfter: ByteArray?,
    ): String

    external fun nameState(walletHandle: Long, name: String): String?
    external fun myNames(walletHandle: Long, identityId: ByteArray?): String
    external fun history(walletHandle: Long, name: String): String

    external fun setPrice(
        walletHandle: Long,
        ownerIdentityId: ByteArray,
        name: String,
        priceCredits: Long,
        signerHandle: Long,
    ): String

    external fun delist(
        walletHandle: Long,
        ownerIdentityId: ByteArray,
        name: String,
        signerHandle: Long,
    ): String

    external fun transfer(
        walletHandle: Long,
        ownerIdentityId: ByteArray,
        name: String,
        recipientIdentityId: ByteArray,
        signerHandle: Long,
    ): String

    external fun purchase(
        walletHandle: Long,
        purchaserIdentityId: ByteArray,
        name: String,
        expectedPriceCredits: Long,
        signerHandle: Long,
    ): String

    external fun sync(walletHandle: Long): String
    external fun syncStart(managerHandle: Long): Boolean
    external fun syncStop(managerHandle: Long): Boolean
    external fun syncIsRunning(managerHandle: Long): Boolean
    external fun syncIsSyncing(managerHandle: Long): Boolean
    external fun syncLastUnixSeconds(managerHandle: Long): Long
    external fun syncSetInterval(managerHandle: Long, seconds: Long): Boolean
    external fun syncNow(managerHandle: Long): LongArray
}
