package org.dashfoundation.dashsdk.ffi

/**
 * Thin JNI surface over `platform-wallet-ffi`'s DPNS name give-up APIs
 * (`platform_wallet_dpns_name_give_up_choices`,
 * `platform_wallet_dpns_give_up_name`,
 * `platform_wallet_dpns_give_up_clears_main_name`), implemented in
 * `rs-unified-sdk-jni/src/dpns_give_up.rs`. The method descriptors there are
 * checked only at runtime: change both sides together and keep
 * `FfiSmokeTest.dpnsGiveUpSymbolsBindAndRejectInvalidInput` passing.
 */
internal object DpnsGiveUpNative {
    init { NativeLoader.ensureLoaded() }

    /** `{"documentId":hex,"canDelete":bool,"canTombstone":bool}`. */
    external fun choices(walletHandle: Long, ownerIdentityId: ByteArray, name: String): String

    /** `{"documentId":hex,"method":int,"tombstoneId":hex|null}`. */
    external fun giveUp(
        walletHandle: Long,
        ownerIdentityId: ByteArray,
        name: String,
        expectedDocumentId: ByteArray,
        method: Int,
        signerHandle: Long,
    ): String

    external fun clearsMainName(mainName: String?, givenUpName: String): Boolean
}
