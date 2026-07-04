package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for the persistence callback bridge — mirrors
 * `rs-unified-sdk-jni/src/persistence.rs`.
 *
 * [createCallbacks] wraps a [NativePersistenceBridge] in a Rust-owned
 * `PersistenceCallbacks` vtable (all 32 slots) and returns the boxed
 * pointer as a `jlong` handle. The bridge object is held by a JNI
 * `GlobalRef` for the vtable's lifetime, so the caller must keep the
 * handle alive until [destroyCallbacks] frees both the vtable box and the
 * `GlobalRef`.
 *
 * The handle is NOT yet handed to a wallet manager — that wiring
 * (`platform_wallet_manager_create`) lands in a later milestone. For now
 * this only proves the vtable round-trips.
 */
internal object PersistenceNative {

    /**
     * Build a native `PersistenceCallbacks` vtable delegating to [bridge].
     *
     * @return non-zero handle to the boxed vtable + context
     */
    external fun createCallbacks(bridge: NativePersistenceBridge): Long

    /** Free a vtable handle from [createCallbacks] and its `GlobalRef`. Safe on 0. */
    external fun destroyCallbacks(handle: Long)
}
