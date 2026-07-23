package org.dashfoundation.dashsdk.keywallet

import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.MnemonicNative

/**
 * BIP-39 mnemonic operations — port of `SwiftDashSDK/KeyWallet/Mnemonic.swift`.
 * Generation runs entirely in Rust (key-wallet-ffi); the phrase transits
 * Kotlin only to be shown for backup and stored encrypted.
 */
object Mnemonic {

    /** Mirror of the iOS `MnemonicLanguage` enum (FFILanguage ordinals). */
    enum class Language(val ffiValue: Int) {
        ENGLISH(0),
        CHINESE_SIMPLIFIED(1),
        CHINESE_TRADITIONAL(2),
        CZECH(3),
        FRENCH(4),
        ITALIAN(5),
    }

    /**
     * Generate a fresh mnemonic. [wordCount] ∈ {12, 15, 18, 21, 24}.
     * @throws org.dashfoundation.dashsdk.errors.DashSdkError on failure
     */
    fun generate(wordCount: Int = 12, language: Language = Language.ENGLISH): String {
        Sdk.initialize()
        return mapNativeErrors { MnemonicNative.generateMnemonic(wordCount, language.ffiValue) }
    }
}
