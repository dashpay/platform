package org.dashfoundation.dashsdk.ffi

/**
 * Loads `libdash_sdk_jni.so` exactly once per process and runs the one-time
 * Rust library initialization (`dash_sdk_init`).
 *
 * Every entry point into the SDK calls [ensureLoaded] before touching a
 * native method; the volatile fast-path makes repeated calls free.
 */
object NativeLoader {

    private const val LIBRARY_NAME = "dash_sdk_jni"

    // Set to true ONLY after both load + init succeed. A failure (e.g. a
    // transient split-install race for the .so, or an init error) leaves
    // it false so the next call retries rather than short-circuiting into
    // a poisoned state where every native entry point throws.
    @Volatile
    private var loaded = false

    /**
     * Load the native library and initialize the Rust runtime. Idempotent
     * and thread-safe; retries on a prior failed attempt.
     *
     * @throws UnsatisfiedLinkError if the library is missing for this ABI —
     *   run `packages/kotlin-sdk/build_android.sh` to produce it.
     */
    fun ensureLoaded() {
        if (loaded) return
        synchronized(this) {
            if (loaded) return
            System.loadLibrary(LIBRARY_NAME)
            SdkNative.nativeInit()
            loaded = true
        }
    }
}
