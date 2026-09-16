package org.dashfoundation.dashsdk.ffi

/**
 * One Kotlin-owned copy of the cached `DpnsNameArray`. Constructor descriptor:
 * `([Ljava/lang/String;)V`.
 *
 * JNI must call `dpns_name_array_free` on every path after
 * `managed_identity_get_contested_dpns_names`, including string/array/object
 * allocation failures. No C pointer escapes this single result.
 * Android currently exposes this as an in-memory snapshot only; contested
 * labels are not restored from persistence after process death.
 */
class ContestedDpnsNamesNativeResult(
    @JvmField val labels: Array<String>,
)
