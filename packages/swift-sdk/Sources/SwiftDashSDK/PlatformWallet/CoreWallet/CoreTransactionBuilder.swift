import Foundation
import DashSDKFFI

/// A built, signed core transaction. Broadcast it via
/// `ManagedCoreWallet.broadcastTransaction`; its bytes are freed when this
/// object is released.
public final class CoreTransaction {
    var ffi: FFICoreTransaction

    /// The account that funded this transaction, captured at build time.
    /// `broadcastTransaction` forwards it so a failed broadcast can release
    /// the UTXO reservation `buildSigned` took on that account.
    let accountType: CoreTransactionBuilder.AccountType
    let accountIndex: UInt32

    init(
        ffi: FFICoreTransaction,
        accountType: CoreTransactionBuilder.AccountType,
        accountIndex: UInt32
    ) {
        self.ffi = ffi
        self.accountType = accountType
        self.accountIndex = accountIndex
    }

    deinit { withUnsafeMutablePointer(to: &ffi) { core_wallet_transaction_free($0) } }

    /// Network fee in duffs.
    public var fee: UInt64 { ffi.fee }

    /// Consensus-serialized signed transaction bytes (copied out).
    public var data: Data {
        guard let p = ffi.tx_bytes, ffi.tx_len > 0 else { return Data() }
        return Data(bytes: p, count: Int(ffi.tx_len))
    }
}

/// Ownership token for a Core transaction atomically funded and reserved in Rust.
public final class FinalizedCoreTransaction {
    private var nativeHandle: Handle
    public let fee: UInt64

    init(handle: Handle) throws {
        var value: UInt64 = 0
        do {
            try core_wallet_signed_transaction_v2_fee(handle, &value).check()
        } catch {
            core_wallet_signed_transaction_v2_free(handle)
            throw error
        }
        nativeHandle = handle
        fee = value
    }

    deinit {
        if nativeHandle != 0 {
            core_wallet_signed_transaction_v2_free(nativeHandle)
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
        try core_wallet_signed_transaction_v2_bytes(nativeHandle, &bytesPtr, &bytesLen).check()

        guard let bytesPtr, bytesLen > 0 else {
            throw PlatformWalletError.unknown(
                "FFI returned success but finalized transaction bytes were empty"
            )
        }
        defer { platform_wallet_bytes_free(bytesPtr, bytesLen) }
        return Data(bytes: bytesPtr, count: Int(bytesLen))
    }
}

/// key-wallet transaction builder over FFI. Add outputs and options, then call
/// `finalizeAtomic` before broadcasting via `ManagedCoreWallet`.
public final class CoreTransactionBuilder {
    public enum AccountType {
        case bip44
        case bip32
        case coinJoin

        var ffi: CoreAccountTypeFFI {
            switch self {
            case .bip44: return CORE_ACCOUNT_TYPE_FFI_BIP44
            case .bip32: return CORE_ACCOUNT_TYPE_FFI_BIP32
            case .coinJoin: return CORE_ACCOUNT_TYPE_FFI_COIN_JOIN
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
    /// Set once `buildSigned` has consumed the builder, so `deinit` skips the
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

    /// Fund from the account's UTXOs and set its change address.
    @discardableResult
    @available(*, deprecated, message: "Use finalizeAtomic; split funding/signing is not concurrency-safe")
    public func setFunding(
        wallet: ManagedPlatformWallet,
        accountType: AccountType,
        accountIndex: UInt32
    ) throws -> CoreTransactionBuilder {
        try core_wallet_tx_builder_set_funding(
            handle, wallet.handle, accountType.ffi, accountIndex
        ).check()
        return self
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

    /// Build and sign against the account; returns the signed transaction
    /// without broadcasting. Consumes the builder — it is freed on the Rust
    /// side and this instance must not be reused afterwards.
    @available(*, deprecated, message: "Use finalizeAtomic; split funding/signing is not concurrency-safe")
    public func buildSigned(
        wallet: ManagedPlatformWallet,
        accountType: AccountType,
        accountIndex: UInt32
    ) throws -> CoreTransaction {
        guard !consumed else {
            throw PlatformWalletError.unknown("CoreTransactionBuilder already consumed")
        }
        var out = FFICoreTransaction(tx_bytes: nil, tx_len: 0, fee: 0)

        let resolver = MnemonicResolver()
        let result = withExtendedLifetime(resolver) {
            core_wallet_tx_builder_build_signed(
                handle,
                wallet.handle,
                accountType.ffi,
                accountIndex,
                resolver.handle,
                &out
            )
        }
        // The FFI frees the builder on every path, so mark consumed before the check.
        consumed = true
        try result.check()

        guard out.tx_bytes != nil, out.tx_len > 0 else {
            throw PlatformWalletError.unknown("FFI returned success but tx buffer was empty")
        }

        return CoreTransaction(ffi: out, accountType: accountType, accountIndex: accountIndex)
    }

    /// Consume this configured builder, atomically select and reserve inputs,
    /// then sign after Rust has released the wallet-manager lock.
    public func finalizeAtomic(
        wallet: ManagedPlatformWallet,
        accountType: AccountType,
        accountIndex: UInt32
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
}
