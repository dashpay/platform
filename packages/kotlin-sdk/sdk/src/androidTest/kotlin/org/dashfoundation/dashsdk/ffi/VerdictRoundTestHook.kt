package org.dashfoundation.dashsdk.ffi

/**
 * Test-only binding to the native credit-verdict round hook
 * (`rs-unified-sdk-jni/src/verdict_round_test_hook.rs`).
 *
 * The export is compiled into the native library only with the
 * `test-hooks` cargo feature (`./build_android.sh --test-hooks`). A library
 * built without it has no such symbol, and a call throws
 * [UnsatisfiedLinkError].
 */
internal object VerdictRoundTestHook {

    init { NativeLoader.ensureLoaded() }

    /**
     * Deliver one credit-verdict round to [bridge] through the real native
     * persistence path — `FFIPersister::store`, the verdict packing and the
     * JNI trampolines — as the SPV engine would after processing a
     * transaction it did not credit, which otherwise needs a synced chain.
     *
     * Output `i` pays `amounts[i]` to `addresses[i]` in the fixture
     * transaction `txTags[i]` (its vout is its position among the outputs
     * sharing that tag; the same tag and outputs always give the same txid),
     * confirmed at [blockHeight]. `verdicts[i]` is `0` for a credited output,
     * else a `UTXO_CREDIT_VERDICT_*` code with `spentAtHeights[i]`. Throws
     * [DashSDKException] when the fixture is malformed or the round fails.
     *
     * @throws UnsatisfiedLinkError when the native library was built without
     *   the `test-hooks` cargo feature.
     */
    external fun nativeDeliverUtxoCreditVerdictRound(
        bridge: NativePersistenceBridge,
        walletId: ByteArray,
        txTags: IntArray,
        addresses: Array<String>,
        amounts: LongArray,
        verdicts: ByteArray,
        spentAtHeights: IntArray,
        blockHeight: Int,
    )
}
