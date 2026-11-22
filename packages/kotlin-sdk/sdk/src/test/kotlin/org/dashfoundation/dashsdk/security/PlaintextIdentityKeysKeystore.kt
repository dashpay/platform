package org.dashfoundation.dashsdk.security

/**
 * A [KeystoreManager] whose identity-key "encryption" keeps the bytes as they are, so a JVM
 * test can write `privkey.*` entries through [WalletStorage.storePrivateKey] without an
 * AndroidKeyStore. Only for presence checks ([WalletStorage.hasPrivateKey]): nothing here
 * models decryption.
 */
internal class PlaintextIdentityKeysKeystore : KeystoreManager(KeySecurityPolicy.AUTH_GATED) {
    override fun encryptForIdentityKeys(plaintext: ByteArray): KeysAliasEncryptedBlob =
        KeysAliasEncryptedBlob(
            EncryptedBlob(iv = ByteArray(0), ciphertext = plaintext.copyOf()),
            keyFingerprint = "plaintext-test",
            alias = KEYS_ALIAS_AUTH_GATED,
        )
}
