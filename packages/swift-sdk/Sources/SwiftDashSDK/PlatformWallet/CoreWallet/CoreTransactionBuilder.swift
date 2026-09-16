import Foundation
import DashSDKFFI

/// Ownership token for a Core transaction atomically funded and reserved in Rust.
public final class FinalizedCoreTransaction {
    private var nativeHandle: Handle
    public let fee: UInt64

    init(handle: Handle) throws {
        var value: UInt64 = 0
        do {
            try core_wallet_signed_transaction_fee(handle, &value).check()
        } catch {
            core_wallet_signed_transaction_free(handle)
            throw error
        }
        nativeHandle = handle
        fee = value
    }

    deinit {
        if nativeHandle != 0 {
            core_wallet_signed_transaction_free(nativeHandle)
        }
    }

    func takeForBroadcast() throws -> Handle {
        guard nativeHandle != 0 else {
            throw PlatformWalletError.unknown("FinalizedCoreTransaction already consumed")
        }
        let value = nativeHandle
        nativeHandle = 0
        return value
    }

    func takeForAbandon() throws -> Handle { try takeForBroadcast() }

    /// Consensus-serialized signed transaction bytes (copied out) without
    /// consuming the ownership token.
    public func serializedData() throws -> Data {
        guard nativeHandle != 0 else {
            throw PlatformWalletError.unknown("FinalizedCoreTransaction already consumed")
        }

        var bytesPtr: UnsafeMutablePointer<UInt8>? = nil
        var bytesLen: UInt = 0
        try core_wallet_signed_transaction_bytes(nativeHandle, &bytesPtr, &bytesLen).check()

        guard let bytesPtr, bytesLen > 0 else {
            throw PlatformWalletError.unknown(
                "FFI returned success but finalized transaction bytes were empty"
            )
        }
        defer { platform_wallet_bytes_free(bytesPtr, bytesLen) }
        return Data(bytes: bytesPtr, count: Int(bytesLen))
    }
}

/// A built, signed Core transaction whose funding UTXOs are reserved, awaiting
/// a deferred `ManagedPlatformWallet.broadcastSigned` or
/// `ManagedPlatformWallet.releaseReservation` — the split-out result of
/// `ManagedPlatformWallet.buildSignedPayment` for BIP70/BIP270 (CTX/DashSpend)
/// flows that must sign now, POST the raw bytes to a merchant server, and
/// broadcast only on the server's ack.
///
/// **Owns the reservation token.** The native registration mints the token
/// before this object exists, so a discarded payment would orphan the token —
/// and with it the funding reservation — until key-wallet's TTL. `close()`
/// releases it exactly once, and `deinit` is the backstop for a caller that
/// never calls `close()`. Release is idempotent native-side and tokens are
/// process-unique (never reused), so releasing a token already consumed by
/// `broadcastSigned` / `releaseReservation` — or releasing twice — is a
/// harmless no-op.
///
/// Process-death note: the reservation is in-memory only. An app crash between
/// the build and the broadcast drops the registry entry and the reservation
/// together on restart, and the UTXOs become spendable again.
///
/// Kotlin parity: `ManagedPlatformWallet.SignedCoreTransaction`
/// (`ManagedPlatformWallet.kt`).
public final class SignedCoreTransaction {
    /// The transaction id (lowercase hex) the broadcast will return, computed
    /// Rust-side from the signed bytes so it matches exactly.
    public let txidHex: String
    /// The consensus-serialized signed transaction, to hand to the merchant
    /// server.
    public let rawTxBytes: Data
    /// The fee this build charged, in duffs.
    public let feeDuffs: UInt64
    /// The opaque token driving `broadcastSigned` / `releaseReservation`. Valid
    /// only for the wallet generation it was minted against, and only until one
    /// of those calls consumes it (or `close()` / `deinit` releases it).
    public let reservationToken: UInt64

    /// Non-atomic, matching `FinalizedCoreTransaction`'s single-owner handle
    /// precedent: a payment is driven by one owner, and the native release is
    /// itself idempotent.
    private var tokenOwned = true
    private let releaseToken: (UInt64) -> Void

    /// - Parameter releaseToken: the release action, defaulting to the native
    ///   one. Injectable so tests can count releases without reaching into the
    ///   process-global registry.
    init(
        txidHex: String,
        rawTxBytes: Data,
        feeDuffs: UInt64,
        reservationToken: UInt64,
        releaseToken: @escaping (UInt64) -> Void = {
            core_wallet_signed_payment_release($0).discard()
        }
    ) {
        self.txidHex = txidHex
        self.rawTxBytes = rawTxBytes
        self.feeDuffs = feeDuffs
        self.reservationToken = reservationToken
        self.releaseToken = releaseToken
    }

    /// Release the funding reservation if this payment was neither broadcast
    /// nor released, and drop the token. Idempotent: a second `close()` is a
    /// no-op here, and calling it after `broadcastSigned` /
    /// `releaseReservation` is safe because the native release of an
    /// already-consumed token is a silent no-op.
    public func close() {
        guard tokenOwned else { return }
        tokenOwned = false
        releaseToken(reservationToken)
    }

    /// Backstop for an abandoned payment, so a caller that never calls
    /// `close()` cannot strand its funding reservation until the TTL.
    deinit {
        if tokenOwned {
            releaseToken(reservationToken)
        }
    }
}

/// key-wallet transaction builder over FFI. Add outputs and options, then call
/// `finalizeAtomic` before broadcasting via `ManagedCoreWallet`.
public final class CoreTransactionBuilder {
    public enum AccountType {
        case bip44
        case bip32
        case coinJoin
        /// Pool every spendable transparent source: BIP44 + BIP32 + all
        /// DashPay contact-receiving accounts. Change returns to BIP44 (the
        /// first pooled source). CoinJoin stays out (separate privacy
        /// domain), as do a contact's watch-only external coins. The default
        /// selector for a plain send.
        case allSpendable

        var ffi: CoreAccountTypeFFI {
            switch self {
            case .bip44: return CORE_ACCOUNT_TYPE_FFI_BIP44
            case .bip32: return CORE_ACCOUNT_TYPE_FFI_BIP32
            case .coinJoin: return CORE_ACCOUNT_TYPE_FFI_COIN_JOIN
            case .allSpendable: return CORE_ACCOUNT_TYPE_FFI_ALL_SPENDABLE
            }
        }
    }

    /// Mirrors key-wallet's `SelectionStrategy`. `all` drains the account.
    public enum SelectionStrategy {
        case smallestFirst
        case largestFirst
        case branchAndBound
        case optimalConsolidation
        case random
        case all

        var ffi: CoreSelectionStrategyFFI {
            switch self {
            case .smallestFirst: return CORE_SELECTION_STRATEGY_FFI_SMALLEST_FIRST
            case .largestFirst: return CORE_SELECTION_STRATEGY_FFI_LARGEST_FIRST
            case .branchAndBound: return CORE_SELECTION_STRATEGY_FFI_BRANCH_AND_BOUND
            case .optimalConsolidation: return CORE_SELECTION_STRATEGY_FFI_OPTIMAL_CONSOLIDATION
            case .random: return CORE_SELECTION_STRATEGY_FFI_RANDOM
            case .all: return CORE_SELECTION_STRATEGY_FFI_ALL
            }
        }
    }

    private let handle: UnsafeMutablePointer<FFITransactionBuilder>
    /// Set once a finalizer has consumed the builder, so `deinit` skips the
    /// Rust-side destroy.
    private var consumed = false

    /// `network` is the wallet network; output and change addresses are
    /// validated against it.
    public init(network: Network) throws {
        guard let handle = core_wallet_tx_builder_new(network.ffiValue) else {
            throw PlatformWalletError.nullPointer("core_wallet_tx_builder_new returned NULL")
        }
        self.handle = handle
    }

    deinit {
        if !consumed {
            core_wallet_tx_builder_destroy(handle)
        }
    }

    /// Add a chosen subset of the account's UTXOs (as returned by
    /// `PlatformWalletManager.accountUtxos`) as inputs. Each must belong to
    /// the account.
    @discardableResult
    public func addInputs(
        wallet: ManagedPlatformWallet,
        accountType: AccountType,
        accountIndex: UInt32,
        utxos: [PlatformWalletManager.AccountUtxo]
    ) throws -> CoreTransactionBuilder {
        var ffiOutpoints = [OutPointFFI]()
        ffiOutpoints.reserveCapacity(utxos.count)
        for utxo in utxos {
            guard utxo.outpointTxid.count == 32 else {
                throw PlatformWalletError.unknown(
                    "outpoint txid must be 32 bytes, got \(utxo.outpointTxid.count)"
                )
            }
            var entry = OutPointFFI()
            withUnsafeMutableBytes(of: &entry.txid) { dst in
                _ = utxo.outpointTxid.copyBytes(to: dst.bindMemory(to: UInt8.self))
            }
            entry.vout = utxo.outpointVout
            ffiOutpoints.append(entry)
        }

        try ffiOutpoints.withUnsafeBufferPointer { buf in
            try core_wallet_tx_builder_add_inputs_from_outpoints(
                handle, wallet.handle, accountType.ffi, accountIndex,
                buf.baseAddress, UInt(buf.count)
            ).check()
        }
        return self
    }

    @discardableResult
    public func addOutput(address: String, amountDuffs: UInt64) throws -> CoreTransactionBuilder {
        let c = strdup(address)
        defer { free(c) }
        try core_wallet_tx_builder_add_output(handle, c, amountDuffs).check()
        return self
    }

    /// Add a zero-value OP_RETURN output carrying `data` for a MAYACHAIN-style
    /// deposit. See https://docs.mayaprotocol.com/mayachain-dev-docs/concepts/sending-transactions
    @discardableResult
    public func addOpReturn(_ data: Data) throws -> CoreTransactionBuilder {
        try data.withUnsafeBytes { buf in
            try core_wallet_tx_builder_add_op_return(
                handle,
                buf.baseAddress?.assumingMemoryBound(to: UInt8.self),
                UInt(data.count)
            ).check()
        }
        return self
    }

    @discardableResult
    public func setChangeAddress(_ address: String) throws -> CoreTransactionBuilder {
        let c = strdup(address)
        defer { free(c) }
        try core_wallet_tx_builder_set_change_address(handle, c).check()
        return self
    }

    /// Preserve outputs in insertion order for a MAYACHAIN-style deposit.
    /// See https://docs.mayaprotocol.com/mayachain-dev-docs/concepts/sending-transactions
    @discardableResult
    public func preserveOutputOrder() throws -> CoreTransactionBuilder {
        try core_wallet_tx_builder_preserve_output_order(handle).check()
        return self
    }

    /// Route change to the first selected input address (VIN0) for a MAYACHAIN-style deposit.
    /// See https://docs.mayaprotocol.com/mayachain-dev-docs/concepts/sending-transactions
    @discardableResult
    public func changeToFirstInput() throws -> CoreTransactionBuilder {
        try core_wallet_tx_builder_change_to_first_input(handle).check()
        return self
    }

    @discardableResult
    public func setFeeRate(satPerKb: UInt64) throws -> CoreTransactionBuilder {
        try core_wallet_tx_builder_set_fee_rate(handle, satPerKb).check()
        return self
    }

    @discardableResult
    public func setSelectionStrategy(_ strategy: SelectionStrategy) throws -> CoreTransactionBuilder {
        try core_wallet_tx_builder_set_selection_strategy(handle, strategy.ffi).check()
        return self
    }

    /// Fund the build from the inputs `addInputs` supplied, and nothing else.
    ///
    /// Without this, `finalizeAtomic` adds every unreserved UTXO of the funding
    /// account to the candidate pool, so seeding a subset does not restrict what
    /// gets selected. A caller draining an account in batches that each stay
    /// under the standard-transaction input limit needs this, or every batch
    /// sees the whole account and fails with a too-many-inputs error.
    @discardableResult
    public func useOnlyAddedInputs() throws -> CoreTransactionBuilder {
        try core_wallet_tx_builder_use_only_added_inputs(handle).check()
        return self
    }

    @discardableResult
    public func setCurrentHeight(_ height: UInt32) throws -> CoreTransactionBuilder {
        try core_wallet_tx_builder_set_current_height(handle, height).check()
        return self
    }

    /// `bincodeBytes` is a bincode-encoded `TransactionPayload`.
    @discardableResult
    public func setSpecialPayload(_ bincodeBytes: Data) throws -> CoreTransactionBuilder {
        try bincodeBytes.withUnsafeBytes { buf in
            try core_wallet_tx_builder_set_special_payload(
                handle,
                buf.baseAddress?.assumingMemoryBound(to: UInt8.self),
                UInt(bincodeBytes.count)
            ).check()
        }
        return self
    }

    /// Consume this configured builder, atomically select and reserve inputs,
    /// then sign after Rust has released the wallet-manager lock.
    public func finalizeAtomic(
        wallet: ManagedPlatformWallet,
        accountType: AccountType = .allSpendable,
        accountIndex: UInt32 = 0
    ) throws -> FinalizedCoreTransaction {
        guard !consumed else {
            throw PlatformWalletError.unknown("CoreTransactionBuilder already consumed")
        }
        var transactionHandle: Handle = 0
        let resolver = MnemonicResolver()
        let result = withExtendedLifetime(resolver) {
            core_wallet_tx_builder_finalize(
                handle,
                wallet.handle,
                accountType.ffi,
                accountIndex,
                resolver.handle,
                &transactionHandle
            )
        }
        consumed = true
        try result.check()
        guard transactionHandle != 0 else {
            throw PlatformWalletError.unknown("atomic finalizer returned an empty handle")
        }
        return try FinalizedCoreTransaction(handle: transactionHandle)
    }

    /// Consume this configured builder and, in ONE atomic native operation,
    /// select + reserve + sign the inputs and register the built transaction
    /// for deferred (BIP70/BIP270) submission. The deferred counterpart to
    /// `finalizeAtomic`: selection and insertion into the account reservation
    /// set commit as a single unit under the wallet-manager lock, so a
    /// concurrent deferred build — or a deferred build racing an immediate
    /// send — cannot double-select an input.
    ///
    /// Consumes the builder on every path, including failures; this instance
    /// must not be reused afterwards.
    ///
    /// Kotlin parity: `CoreTransactionBuilder.finalizeSignedPayment`
    /// (`CoreTransactionBuilder.kt`).
    public func finalizeSignedPayment(
        wallet: ManagedPlatformWallet,
        accountType: AccountType,
        accountIndex: UInt32
    ) throws -> SignedCoreTransaction {
        guard !consumed else {
            throw PlatformWalletError.unknown("CoreTransactionBuilder already consumed")
        }
        var token: UInt64 = 0
        var feeDuffs: UInt64 = 0
        var txidPtr: UnsafeMutablePointer<CChar>? = nil
        var tx = FFICoreTransaction(tx_bytes: nil, tx_len: 0, fee: 0)
        var bytesPtr: UnsafePointer<UInt8>? = nil
        var bytesLen: UInt = 0

        let resolver = MnemonicResolver()
        let result = withExtendedLifetime(resolver) {
            core_wallet_signed_payment_finalize(
                handle,
                wallet.handle,
                accountType.ffi,
                accountIndex,
                resolver.handle,
                &token,
                &feeDuffs,
                &txidPtr,
                &tx,
                &bytesPtr,
                &bytesLen
            )
        }
        consumed = true
        try result.check()

        defer {
            if let txidPtr {
                core_wallet_free_address(txidPtr)
            }
            core_wallet_transaction_free(&tx)
        }

        // Every guard below trips after the native register already minted the
        // token and transferred reservation ownership to it, so throwing without
        // releasing would strand the reservation until key-wallet's TTL with no
        // token left to release it. Release defensively first, mirroring the
        // owner-guarded release on the rest of the deferred path
        // (dashpay/platform#4185).
        func releaseMintedToken() {
            if token != 0 {
                core_wallet_signed_payment_release(token).discard()
            }
        }

        guard token != 0 else {
            throw PlatformWalletError.unknown(
                "deferred finalizer returned success with a zero reservation token"
            )
        }
        guard let txidCString = txidPtr else {
            releaseMintedToken()
            throw PlatformWalletError.nullPointer(
                "core_wallet_signed_payment_finalize returned a NULL txid pointer"
            )
        }
        guard let rawBytes = bytesPtr, bytesLen > 0 else {
            releaseMintedToken()
            throw PlatformWalletError.unknown(
                "deferred finalizer returned success but the tx buffer was empty"
            )
        }

        // The borrowed view into `tx`'s buffer is copied here, before the
        // deferred `core_wallet_transaction_free` frees it.
        return SignedCoreTransaction(
            txidHex: String(cString: txidCString),
            rawTxBytes: Data(bytes: rawBytes, count: Int(bytesLen)),
            feeDuffs: feeDuffs,
            reservationToken: token
        )
    }
}
