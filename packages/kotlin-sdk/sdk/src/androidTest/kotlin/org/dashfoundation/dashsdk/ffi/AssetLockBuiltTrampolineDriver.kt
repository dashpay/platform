package org.dashfoundation.dashsdk.ffi

/**
 * Test-only binding to the native driver that runs the production
 * `asset_lock_built_trampoline` (rs-unified-sdk-jni `wallet_manager.rs`)
 * without a wallet. The export is compiled only into debug
 * (`build_android.sh --profile dev`) libraries, which is what the
 * instrumented tests load.
 */
object AssetLockBuiltTrampolineDriver {
    /**
     * Pin each of [bridges] the way a funding call does and invoke the
     * trampoline for each, in order, on one new native thread with
     * [outPoint] (must be 36 bytes) as the payload. Returns after that thread
     * has finished.
     */
    external fun invokeOnNativeThread(bridges: Array<AssetLockBuiltBridge>, outPoint: ByteArray)
}
