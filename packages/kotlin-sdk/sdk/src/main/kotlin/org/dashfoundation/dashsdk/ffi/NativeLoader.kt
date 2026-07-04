package org.dashfoundation.dashsdk.ffi

import java.util.concurrent.atomic.AtomicBoolean

/**
 * Loads `libdash_sdk_jni.so` exactly once per process and runs the one-time
 * Rust library initialization (`dash_sdk_init`).
 *
 * Every entry point into the SDK calls [ensureLoaded] before touching a
 * native method; the flag makes repeated calls free.
 */
object NativeLoader {

    private const val LIBRARY_NAME = "dash_sdk_jni"

    private val loaded = AtomicBoolean(false)

    /**
     * Load the native library and initialize the Rust runtime.
     *
     * @throws UnsatisfiedLinkError if the library is missing for this ABI —
     *   run `packages/kotlin-sdk/build_android.sh` to produce it.
     */
    fun ensureLoaded() {
        if (loaded.compareAndSet(false, true)) {
            System.loadLibrary(LIBRARY_NAME)
            SdkNative.nativeInit()
        }
    }
}
