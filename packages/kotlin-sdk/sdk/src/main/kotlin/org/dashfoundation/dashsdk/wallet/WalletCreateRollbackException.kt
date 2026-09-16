package org.dashfoundation.dashsdk.wallet

/**
 * [PlatformWalletManager.createWallet] failed AND its rollback could not
 * remove the persisted Room rows — the wallet survives on disk.
 *
 * When [mnemonicStored] is `false`, every attempt to store the phrase
 * durably also failed, and [mnemonic] carries the plaintext phrase as the
 * LAST remaining copy: callers typically hold it only in a local that dies
 * with this failure (CreateWalletScreen generates into a `try`-block
 * local), so without this field the surviving rows would be permanently
 * seedless while the phrase becomes inaccessible — the orphan-recovery
 * flow cannot help because nothing reached [org.dashfoundation.dashsdk.security.WalletStorage].
 * The UI MUST surface [mnemonic] for manual backup before discarding this
 * exception. Carrying a plaintext phrase in an exception is deliberate
 * last-resort behavior: the alternative is irrecoverable loss.
 *
 * When [mnemonicStored] is `true` the phrase is safe in storage, the
 * leftover rows load as a functional wallet, and cleanup can be retried
 * via [PlatformWalletManager.removeWallet]; [mnemonic] is `null`.
 */
class WalletCreateRollbackException(
    val walletId: ByteArray,
    val mnemonicStored: Boolean,
    val mnemonic: String?,
    message: String,
    cause: Throwable,
) : IllegalStateException(message, cause)
