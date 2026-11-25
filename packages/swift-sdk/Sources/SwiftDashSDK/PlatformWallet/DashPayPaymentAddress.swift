import DashSDKFFI
import Foundation

/// Which of the three public payment addresses of a DashPay profile a value
/// belongs to. The raw value is the FFI's `DASHPAY_PROFILE_PAYMENT_ADDRESS_*`
/// kind.
public enum DashPayPaymentAddressKind: UInt32, Sendable, Hashable, CaseIterable {
    /// `corePaymentAddress`, shown as a Base58Check Core address.
    case core = 0
    /// `platformPaymentAddress`, shown as a bech32m Platform address.
    case platform = 1
    /// `shieldedAddress`, shown as a bech32m Orchard address.
    case shielded = 2
}

/// Which payment addresses a profile write may set on the connected network,
/// as `platform-wallet` reports it. Removing an address needs no support.
public struct DashPayPaymentAddressSupport: Sendable, Equatable {
    public let core: Bool
    public let platform: Bool
    public let shielded: Bool

    public init(core: Bool, platform: Bool, shielded: Bool) {
        self.core = core
        self.platform = platform
        self.shielded = shielded
    }

    /// Whether an address of `kind` can be set.
    public func supports(_ kind: DashPayPaymentAddressKind) -> Bool {
        switch kind {
        case .core: return core
        case .platform: return platform
        case .shielded: return shielded
        }
    }
}

extension DashPayProfile {
    /// The address of `kind` this profile publishes, in its stored form.
    public func paymentAddress(_ kind: DashPayPaymentAddressKind) -> Data? {
        switch kind {
        case .core: return corePaymentAddress
        case .platform: return platformPaymentAddress
        case .shielded: return shieldedAddress
        }
    }

    /// The bytes a profile stores for the address `text` of `kind` on
    /// `network`. `platform-wallet` parses and checks the address, and refuses
    /// one of another network or kind.
    ///
    /// - Throws: `PlatformWalletError` carrying the wallet's readable reason.
    public static func paymentAddressBytes(
        kind: DashPayPaymentAddressKind,
        from text: String,
        network: Network
    ) throws -> Data {
        var bytes: UnsafeMutablePointer<UInt8>?
        var len: UInt = 0
        try text.withCString { textPtr in
            try dashpay_profile_payment_address_from_string(
                kind.rawValue, textPtr, network.ffiValue, &bytes, &len
            ).check()
        }
        guard let bytes else {
            throw PlatformWalletError.nullPointer("Missing payment address bytes")
        }
        defer { platform_wallet_bytes_free(bytes, len) }
        return Data(bytes: bytes, count: Int(len))
    }

    /// The address string of `network` for the stored `bytes` of a `kind`
    /// address, or `nil` when `platform-wallet` does not read them as an
    /// address of that kind.
    public static func paymentAddressString(
        kind: DashPayPaymentAddressKind,
        bytes: Data,
        network: Network
    ) -> String? {
        var out: UnsafeMutablePointer<CChar>?
        let result = bytes.withUnsafeBytes { raw in
            dashpay_profile_payment_address_to_string(
                kind.rawValue,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                UInt(raw.count),
                network.ffiValue,
                &out
            )
        }
        guard (try? result.check()) != nil, let out else { return nil }
        defer { platform_wallet_string_free(out) }
        return String(cString: out)
    }
}

extension ManagedPlatformWallet {
    /// Which payment addresses a profile write may set on the network this
    /// wallet is connected to. Fetches the network's DashPay contract.
    public func dashPayPaymentAddressSupport() async throws -> DashPayPaymentAddressSupport {
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> DashPayPaymentAddressSupport in
            // `deinit` destroys `handle`, so the wallet stays alive for the call.
            try withExtendedLifetime(self) {
                var out = DashPayPaymentAddressSupportFFI()
                try platform_wallet_dashpay_profile_payment_address_support(handle, &out).check()
                return DashPayPaymentAddressSupport(
                    core: out.core,
                    platform: out.platform,
                    shielded: out.shielded
                )
            }
        }.value
    }
}
