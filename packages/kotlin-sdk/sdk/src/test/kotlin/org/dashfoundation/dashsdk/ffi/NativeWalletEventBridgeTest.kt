package org.dashfoundation.dashsdk.ffi

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * The native trampoline resolves a bridge method by name and JNI descriptor
 * only when its event first fires. This pins the Kotlin side of that pair,
 * so a changed signature fails here and not on a device.
 */
class NativeWalletEventBridgeTest {

    @Test
    fun shouldDeclareTheShieldedBalanceCallbackWithTheDescriptorRustCalls() {
        // `([BJIJZ[J)V` in `rs-unified-sdk-jni/src/events.rs`
        // (`SHIELDED_BALANCE_CHANGED_DESCRIPTOR`).
        val method = NativeWalletEventBridge::class.java.getMethod(
            "onShieldedBalanceChanged",
            ByteArray::class.java,
            Long::class.javaPrimitiveType,
            Int::class.javaPrimitiveType,
            Long::class.javaPrimitiveType,
            Boolean::class.javaPrimitiveType,
            LongArray::class.java,
        )
        assertEquals(Void.TYPE, method.returnType)
    }
}
