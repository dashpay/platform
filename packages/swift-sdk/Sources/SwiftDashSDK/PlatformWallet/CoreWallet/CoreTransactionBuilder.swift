import Foundation
import DashSDKFFI

public final class CoreTransaction {
    public struct Input: Sendable, Equatable {
        public let prevTxid: Data
        public let prevVout: UInt32
        public let address: String?

        public var prevTxidDisplayHex: String {
            prevTxid.reversed().map { String(format: "%02x", $0) }.joined()
        }
    }

    public struct Output: Sendable, Equatable {
        public let address: String?
        public let valueDuffs: UInt64
        public let scriptPubKey: Data
    }

    let ffi: UnsafeMutablePointer<FFICoreTransaction>

    /// The account that funded this transaction, captured at build time.
    /// `broadcastTransaction` forwards it so a failed broadcast can release
    /// the UTXO reservation `buildSigned` took on that account.
    let accountType: CoreTransactionBuilder.AccountType
    let accountIndex: UInt32

    public let fee: UInt64
    public let txid: Data
    public let inputs: [Input]
    public let outputs: [Output]

    init(
        ffi: UnsafeMutablePointer<FFICoreTransaction>,
        accountType: CoreTransactionBuilder.AccountType,
        accountIndex: UInt32
    ) {
        self.ffi = ffi
        self.accountType = accountType
        self.accountIndex = accountIndex

        // Copy the value fields out of the heap struct now; the pointer stays
        // alive (freed in deinit) so `data` and broadcast can still read it.
        var entry = ffi.pointee
        self.fee = entry.fee
        self.txid = withUnsafeBytes(of: &entry.txid) { Data($0) }

        var inputs: [Input] = []
        if let ptr = entry.inputs, entry.inputs_count > 0 {
            inputs = (0..<Int(entry.inputs_count)).map { i in
                var input = ptr[i]
                let prevTxid = withUnsafeBytes(of: &input.prev_txid) { Data($0) }
                let address = input.address.map { String(cString: $0) }
                return Input(
                    prevTxid: prevTxid,
                    prevVout: input.prev_vout,
                    address: address
                )
            }
        }
        self.inputs = inputs

        var outputs: [Output] = []
        if let ptr = entry.outputs, entry.outputs_count > 0 {
            outputs = (0..<Int(entry.outputs_count)).map { i in
                let output = ptr[i]
                let address = output.address.map { String(cString: $0) }
                let script: Data
                if let sptr = output.script_pubkey, output.script_pubkey_len > 0 {
                    script = Data(bytes: sptr, count: Int(output.script_pubkey_len))
                } else {
                    script = Data()
                }
                return Output(
                    address: address,
                    valueDuffs: output.value_duffs,
                    scriptPubKey: script
                )
            }
        }
        self.outputs = outputs
    }

    deinit { core_wallet_transaction_free(ffi) }

    public var txidDisplayHex: String {
        txid.reversed().map { String(format: "%02x", $0) }.joined()
    }

    public var data: Data {
        guard let p = ffi.pointee.tx_bytes, ffi.pointee.tx_len > 0 else { return Data() }
        return Data(bytes: p, count: Int(ffi.pointee.tx_len))
    }
}

/// key-wallet transaction builder over FFI. Build step by step, `buildSigned`,
/// then broadcast separately via `ManagedCoreWallet.broadcastTransaction`.
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

    @discardableResult
    public func setChangeAddress(_ address: String) throws -> CoreTransactionBuilder {
        let c = strdup(address)
        defer { free(c) }
        try core_wallet_tx_builder_set_change_address(handle, c).check()
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
    public func buildSigned(
        wallet: ManagedPlatformWallet,
        accountType: AccountType,
        accountIndex: UInt32
    ) throws -> CoreTransaction {
        guard !consumed else {
            throw PlatformWalletError.unknown("CoreTransactionBuilder already consumed")
        }
        var out: UnsafeMutablePointer<FFICoreTransaction>? = nil

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

        guard let ptr = out else {
            throw PlatformWalletError.nullPointer(
                "FFI returned success but no transaction pointer"
            )
        }

        return CoreTransaction(ffi: ptr, accountType: accountType, accountIndex: accountIndex)
    }
}
