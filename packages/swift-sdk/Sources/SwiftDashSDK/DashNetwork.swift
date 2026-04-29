import Foundation
import DashSDKFFI

/// The Dash network the SDK is operating on.
///
/// Single Swift wrapper for `dash-network`'s `FFINetwork` C-ABI mirror
/// (the `#[repr(C)]` enum exposed by the `dash-network` Rust crate's
/// `ffi` module). Every Rust-side FFI surface — `rs-platform-wallet-ffi`,
/// `rs-sdk-ffi`, `rs-key-wallet-ffi`, `dash-spv-ffi` — takes / returns
/// `FFINetwork`; this Swift enum is the matching surface on the iOS
/// side so callers never reach for `UInt32` or per-module mirror enums.
public enum Network: UInt32, Sendable, Hashable, CaseIterable, Codable {
    case mainnet = 0
    case testnet = 1
    case devnet = 2
    case regtest = 3

    /// Build from the C-ABI `FFINetwork` mirror exposed by `dash-network`.
    public init(ffiNetwork: FFINetwork) {
        switch ffiNetwork {
        case FFINetwork_Mainnet: self = .mainnet
        case FFINetwork_Testnet: self = .testnet
        case FFINetwork_Devnet: self = .devnet
        case FFINetwork_Regtest: self = .regtest
        default: fatalError("Invalid FFINetwork value: \(ffiNetwork)")
        }
    }

    /// The matching `FFINetwork` value; pass to any function whose
    /// signature in the cbindgen-generated header takes
    /// `enum FFINetwork`. Named `ffiValue` to match the convention
    /// sibling enums in this module already use (`AccountType`,
    /// `KeyPurpose`, …).
    public var ffiValue: FFINetwork {
        switch self {
        case .mainnet: return FFINetwork_Mainnet
        case .testnet: return FFINetwork_Testnet
        case .devnet: return FFINetwork_Devnet
        case .regtest: return FFINetwork_Regtest
        }
    }

    public var displayName: String {
        switch self {
        case .mainnet: return "Mainnet"
        case .testnet: return "Testnet"
        case .regtest: return "Local"
        case .devnet:  return "Devnet"
        }
    }

    public var networkName: String {
        switch self {
        case .mainnet: return "mainnet"
        case .testnet: return "testnet"
        case .regtest: return "regtest"
        case .devnet:  return "devnet"
        }
    }

    public static var defaultNetwork: Network { .testnet }
}
