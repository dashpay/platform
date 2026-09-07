import Foundation
import DashSDKFFI

/// Authoritative network outcome for a signed Core transaction.
///
/// `accepted` means Dash Core accepted the transaction into its mempool or
/// already knows it; it does not mean the transaction is mined or
/// InstantSend-locked. `unknown` must not be retried automatically because the
/// Core response may have been lost after acceptance.
public enum CoreTransactionBroadcastOutcome: Equatable, Sendable {
    case accepted(txid: String)
    case rejected(txid: String, reason: String)
    case unknown(txid: String, reason: String)

    public var txid: String {
        switch self {
        case .accepted(let txid), .rejected(let txid, _), .unknown(let txid, _):
            return txid
        }
    }

    init(
        resultCode: PlatformWalletResultCode,
        txid: String,
        reason: String
    ) throws {
        switch resultCode {
        case .success:
            self = .accepted(txid: txid)
        case .errorTransactionBroadcastRejected:
            self = .rejected(txid: txid, reason: reason)
        case .errorTransactionBroadcastUnconfirmed:
            self = .unknown(txid: txid, reason: reason)
        default:
            throw PlatformWalletError.unknown(
                "Cannot create Core broadcast outcome from \(resultCode)"
            )
        }
    }
}

/// Core wallet for UTXO management, address derivation, and transaction broadcasting.
///
/// Obtained via `ManagedPlatformWallet.coreWallet()`.
public class ManagedCoreWallet {
    let handle: Handle

    init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        core_wallet_destroy(handle).discard()
    }

    // MARK: - Balance

    /// Core wallet balance breakdown (lock-free atomic reads).
    public struct CoreBalance {
        /// Confirmed balance (in a block or InstantSend-locked).
        public let confirmed: UInt64
        /// Unconfirmed balance (mempool-only, also spendable).
        public let unconfirmed: UInt64
        /// Immature balance (coinbase UTXOs not yet mature).
        public let immature: UInt64
        /// Locked balance (reserved for specific purposes).
        public let locked: UInt64

        public var total: UInt64 {
            confirmed + unconfirmed + immature + locked
        }
    }

    /// Read the current balance (lock-free atomic reads).
    public func balance() throws -> CoreBalance {
        var confirmed: UInt64 = 0
        var unconfirmed: UInt64 = 0
        var immature: UInt64 = 0
        var locked: UInt64 = 0
        try core_wallet_get_balance(
            handle, &confirmed, &unconfirmed, &immature, &locked
        ).check()

        return CoreBalance(
            confirmed: confirmed,
            unconfirmed: unconfirmed,
            immature: immature,
            locked: locked
        )
    }

    /// Get the network this wallet operates on.
    public func network() throws -> Network {
        var ffiNetwork = FFINetwork(0)
        try core_wallet_get_network(handle, &ffiNetwork).check()
        return Network(ffiNetwork: ffiNetwork)
    }

    // MARK: - Addresses

    /// Get the next unused receive address for a specific BIP-44 account.
    public func nextReceiveAddress(accountIndex: UInt32 = 0) throws -> String {
        var addressPtr: UnsafeMutablePointer<CChar>? = nil
        try core_wallet_next_receive_address(handle, accountIndex, &addressPtr).check()
        guard let ptr = addressPtr else {
            throw PlatformWalletError.nullPointer(
                "core_wallet_next_receive_address returned a NULL address pointer"
            )
        }
        defer { core_wallet_free_address(ptr) }
        return String(cString: ptr)
    }

    /// Get the next unused change address for a specific BIP-44 account.
    public func nextChangeAddress(accountIndex: UInt32 = 0) throws -> String {
        var addressPtr: UnsafeMutablePointer<CChar>? = nil
        try core_wallet_next_change_address(handle, accountIndex, &addressPtr).check()
        guard let ptr = addressPtr else {
            throw PlatformWalletError.nullPointer(
                "core_wallet_next_change_address returned a NULL address pointer"
            )
        }
        defer { core_wallet_free_address(ptr) }
        return String(cString: ptr)
    }

    /// Widen the gap limit for an account, generating the addresses the wider
    /// limit now requires.
    public func setGapLimit(
        accountType: CoreTransactionBuilder.AccountType,
        accountIndex: UInt32,
        gapLimit: UInt32
    ) throws {
        try core_wallet_set_gap_limit(handle, accountType.ffi, accountIndex, gapLimit).check()
    }

    // MARK: - Message Signing

    /// Sign `message` with the private key behind `address` and return the
    /// signature as base64 — a **classic Dash signed message**, byte-for-byte
    /// compatible with dashj's `ECKey.signMessage` and Dash Core's `signmessage`
    /// RPC, and verifiable by `verifymessage`, `ECKey.verifyMessage`, and
    /// CrowdNode's server-side check.
    ///
    /// The signed digest is
    /// `SHA256d(prefix ‖ varint(message.utf8.count) ‖ message.utf8)`: the length
    /// prefix counts **UTF-8 bytes**, not `String.count`, which counts grapheme
    /// clusters and diverges for any non-ASCII text.
    /// The prefix is the historical `"\u{19}DarkCoin Signed Message:\n"` —
    /// *not* `"Dash"`. Dash inherited that string from before the rename and
    /// every existing verifier depends on it, so it can never change. The
    /// returned signature is the 65-byte BIP-137-style recoverable form
    /// (`header ‖ r ‖ s`, with `header = 27 + recoveryId + 4`, the `+ 4` marking
    /// a compressed public key), base64-encoded. A verifier recovers the public
    /// key from the digest and compares its hash to `address` — which is why
    /// only P2PKH addresses can sign: no other payload has a defined recovery
    /// comparison.
    ///
    /// This is a **proof of address ownership, not a spend** — the shape
    /// CrowdNode needs, where short strings (a withdrawal amount, the account
    /// email) are signed with the key behind the address that funded the
    /// account. No UTXO is selected, reserved, spent, or broadcast, no balance
    /// changes, and nothing is persisted, so calling it repeatedly is free and
    /// side-effect-free.
    ///
    /// `address` must be a P2PKH address of *this* wallet, on this wallet's
    /// network, already derived into one of its address pools, and belonging to
    /// a **signable funds account** (BIP44 / BIP32 / CoinJoin /
    /// DashPay-*receiving*). A watch-only DashPay **external** account holds a
    /// contact's receiving addresses whose private keys we never had, so those
    /// are refused exactly like any other address the wallet does not own.
    ///
    /// Signing is RFC6979 deterministic and low-s normalized, so the same
    /// (`address`, `message`) pair on the same seed always returns the same
    /// string.
    ///
    /// The key is derived through the Keychain mnemonic resolver Rust-side, like
    /// every other seed-backed path here, so no private key crosses the boundary
    /// and no resident seed is needed.
    ///
    /// - Parameters:
    ///   - address: the P2PKH address whose key signs; must be one this wallet
    ///     owns and has derived.
    ///   - message: the string to sign, **verbatim**. It is length-prefixed into
    ///     the digest, so trailing whitespace and newlines are significant and
    ///     the verifier must receive the identical bytes. An empty string is
    ///     valid and signable.
    /// - Returns: the base64 signature (88 characters for the 65-byte payload).
    /// - Throws: `PlatformWalletError.signingKeyUnavailable` when `address`
    ///   belongs to no signable funds account of this wallet;
    ///   `PlatformWalletError.invalidParameter` when it is unparseable, encoded
    ///   for another network, or not P2PKH.
    public func signMessage(address: String, message: String) throws -> String {
        guard !address.isEmpty else {
            throw PlatformWalletError.invalidParameter("address must not be empty")
        }

        // Resolver-backed core signer, as on every other seed-backed path.
        let coreSigner = MnemonicResolver()
        // Both arguments cross as raw UTF-8 bytes with an explicit length (no NUL
        // terminator), so an embedded NUL cannot truncate what actually gets
        // signed. `address` is non-empty per the guard above, so its
        // `baseAddress` is non-nil; an empty `message` yields a nil
        // `baseAddress`, which the FFI reads as the empty message at length 0 —
        // the one case where it accepts a null pointer.
        let addressBytes = Array(address.utf8)
        let messageBytes = Array(message.utf8)

        var signaturePtr: UnsafeMutablePointer<CChar>? = nil
        // `withExtendedLifetime` keeps the resolver alive across the synchronous
        // FFI call — the optimizer can otherwise drop it mid-call and a vtable
        // callback would use-after-free.
        let result: PlatformWalletFFIResult = withExtendedLifetime(coreSigner) {
            addressBytes.withUnsafeBufferPointer {
                addressBuf -> PlatformWalletFFIResult in
                messageBytes.withUnsafeBufferPointer {
                    messageBuf -> PlatformWalletFFIResult in
                    core_wallet_sign_message(
                        handle,
                        addressBuf.baseAddress,
                        UInt(addressBuf.count),
                        messageBuf.baseAddress,
                        UInt(messageBuf.count),
                        coreSigner.handle,
                        &signaturePtr
                    )
                }
            }
        }
        try result.check()

        guard let ptr = signaturePtr else {
            throw PlatformWalletError.nullPointer(
                "core_wallet_sign_message returned a NULL signature pointer"
            )
        }
        defer { core_wallet_free_address(ptr) }
        return String(cString: ptr)
    }

    // MARK: - Transactions

    /// Consume and broadcast an atomically finalized transaction, returning
    /// the authoritative accepted/rejected/unknown network outcome.
    ///
    /// - Important: `.errorStaleReservationToken` (native code 34) is a
    ///   **terminal** outcome, not a retryable one. It means the handle was held
    ///   until the wallet's `last_processed_height` advanced past the funding
    ///   reservation's age bound, so the transaction was refused *before* the
    ///   network was touched — nothing was sent.
    ///
    ///   Neither a retry nor an abandon is possible: `takeForBroadcast()` above
    ///   has already consumed this handle, so calling
    ///   `broadcastTransactionWithOutcome(_:)` again throws locally, and
    ///   `abandonTransaction(_:)` has nothing left to release. The refusal path
    ///   in Rust reconciles the reservation itself, releasing the inputs
    ///   owner-guarded.
    ///
    ///   **Recover by rebuilding the transaction.** The released inputs are
    ///   immediately reselectable by a fresh builder → `finalize` sequence, with
    ///   no cleanup call in between.
    public func broadcastTransactionWithOutcome(
        _ tx: FinalizedCoreTransaction
    ) throws -> CoreTransactionBroadcastOutcome {
        let transactionHandle = try tx.takeForBroadcast()
        var txidPtr: UnsafeMutablePointer<CChar>? = nil
        let result = PlatformWalletResult(core_wallet_broadcast_signed_transaction(
            handle,
            transactionHandle,
            &txidPtr
        ))

        defer {
            if let txidPtr {
                core_wallet_free_address(txidPtr)
            }
        }

        switch result.code {
        case .success, .errorTransactionBroadcastRejected,
             .errorTransactionBroadcastUnconfirmed:
            guard let txidPtr else {
                throw PlatformWalletError.nullPointer(
                    "core_wallet_broadcast_signed_transaction returned a NULL txid pointer for \(result.code)"
                )
            }
            return try CoreTransactionBroadcastOutcome(
                resultCode: result.code,
                txid: String(cString: txidPtr),
                reason: result.message ?? "<no detail from Rust>"
            )

        default:
            try result.throwIfError()
            throw PlatformWalletError.unknown(
                "core_wallet_broadcast_signed_transaction returned an unexpected success state"
            )
        }
    }

    /// Compatibility wrapper preserving the former throwing API.
    @available(*, deprecated, message: "Use broadcastTransactionWithOutcome(_:) and handle accepted/rejected/unknown")
    public func broadcastTransaction(_ tx: FinalizedCoreTransaction) throws -> String {
        switch try broadcastTransactionWithOutcome(tx) {
        case .accepted(let txid):
            return txid
        case .rejected(_, let reason):
            throw PlatformWalletError.transactionBroadcastRejected(reason)
        case .unknown(_, let reason):
            throw PlatformWalletError.transactionBroadcastUnconfirmed(reason)
        }
    }

    /// Broadcast the deferred (BIP70/BIP270) payment behind `token` and return
    /// its txid. The token is consumed atomically before the send, so a repeated
    /// or concurrent broadcast gets an error rather than a second send.
    ///
    /// An unusable token surfaces as one of three sibling errors instead:
    /// `.staleReservationToken` (34) — the reservation may already have aged out
    /// of key-wallet's TTL, rebuild the payment; `.reservationTokenConsumed`
    /// (35) — unknown, already broadcast, or already released;
    /// `.reservationWalletMismatch` (36) — minted against a different wallet
    /// generation. `.notFound` (98) means the token's wallet was removed from
    /// the manager entirely. None of them touched the network and none is
    /// retryable in place.
    ///
    /// Kotlin parity: `ManagedCoreWallet.broadcastSignedPayment`
    /// (`ManagedCoreWallet.kt`).
    func broadcastSignedPayment(token: UInt64) throws -> String {
        var txidPtr: UnsafeMutablePointer<CChar>? = nil
        try core_wallet_signed_payment_broadcast(handle, token, &txidPtr).check()
        guard let ptr = txidPtr else {
            throw PlatformWalletError.nullPointer(
                "core_wallet_signed_payment_broadcast returned a NULL txid pointer"
            )
        }
        defer { core_wallet_free_address(ptr) }
        return String(cString: ptr)
    }

    /// Consume without sending and release its reservation immediately.
    public func abandonTransaction(_ tx: FinalizedCoreTransaction) throws {
        try core_wallet_abandon_signed_transaction(
            handle,
            tx.takeForAbandon()
        ).check()
    }
}
