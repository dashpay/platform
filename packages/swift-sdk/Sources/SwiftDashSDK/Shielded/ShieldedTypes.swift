// ShieldedTypes.swift
// SwiftDashSDK
//
// Swift types for shielded pool (Orchard/ZK) operations.
// These match the Rust #[repr(C)] structs in rs-sdk-ffi/src/shielded/types.rs
// and rs-sdk-ffi/src/shielded/transitions/shield.rs.

import Foundation

// MARK: - FFI C-Compatible Structs
// These must have EXACTLY the same memory layout as the corresponding Rust #[repr(C)] types.

/// Matches `DashSDKSerializedAction` in Rust.
/// A serialized Orchard action for FFI.
struct FFISerializedAction {
    var nullifier: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    var rk: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    var cmx: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    var encrypted_note: UnsafePointer<UInt8>?
    var encrypted_note_len: Int
    var cv_net: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    var spend_auth_sig: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
}

/// Matches `DashSDKOrchardBundleParams` in Rust.
/// Orchard bundle parameters for FFI, shared across all shielded transition types.
struct FFIOrchardBundleParams {
    var actions: UnsafePointer<FFISerializedAction>?
    var actions_count: UInt32
    var anchor: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    var proof: UnsafePointer<UInt8>?
    var proof_len: Int
    var binding_signature: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
}

/// Matches `DashSDKShieldInput` in Rust.
/// Input entry for shield operation: address + amount + private key.
struct FFIShieldInput {
    var address: UnsafePointer<UInt8>?
    var address_len: Int
    var amount: UInt64
    var private_key: UnsafePointer<UInt8>?
}

// MARK: - Tuple Helpers

/// Type alias for a 32-byte tuple (used for nullifiers, anchors, keys, etc.)
typealias Bytes32Tuple = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
)

/// Type alias for a 36-byte tuple (used for OutPoint: 32 txid + 4 index)
typealias Bytes36Tuple = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8
)

/// Type alias for a 64-byte tuple (used for signatures)
typealias Bytes64Tuple = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
)

/// Copy Data into a 32-byte tuple, zero-padding if shorter.
func dataToBytes32(_ data: Data) -> Bytes32Tuple {
    var tuple: Bytes32Tuple = (
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    )
    data.withUnsafeBytes { src in
        withUnsafeMutableBytes(of: &tuple) { dst in
            let count = min(src.count, 32)
            if count > 0, let srcBase = src.baseAddress {
                dst.baseAddress?.copyMemory(from: srcBase, byteCount: count)
            }
        }
    }
    return tuple
}

/// Copy Data into a 36-byte tuple.
func dataToBytes36(_ data: Data) -> Bytes36Tuple {
    var tuple: Bytes36Tuple = (
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0
    )
    data.withUnsafeBytes { src in
        withUnsafeMutableBytes(of: &tuple) { dst in
            let count = min(src.count, 36)
            if count > 0, let srcBase = src.baseAddress {
                dst.baseAddress?.copyMemory(from: srcBase, byteCount: count)
            }
        }
    }
    return tuple
}

/// Copy Data into a 64-byte tuple.
func dataToBytes64(_ data: Data) -> Bytes64Tuple {
    var tuple: Bytes64Tuple = (
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    )
    data.withUnsafeBytes { src in
        withUnsafeMutableBytes(of: &tuple) { dst in
            let count = min(src.count, 64)
            if count > 0, let srcBase = src.baseAddress {
                dst.baseAddress?.copyMemory(from: srcBase, byteCount: count)
            }
        }
    }
    return tuple
}

/// Extract Data from a 32-byte tuple.
func bytes32ToData(_ tuple: Bytes32Tuple) -> Data {
    var mutable = tuple
    return withUnsafeBytes(of: &mutable) { Data($0) }
}

// MARK: - High-Level Swift Models

/// A single Orchard action (high-level Swift representation).
public struct OrchardAction: Sendable {
    /// 32-byte nullifier
    public let nullifier: Data
    /// 32-byte randomized verification key
    public let rk: Data
    /// 32-byte note commitment
    public let cmx: Data
    /// Variable-length encrypted note ciphertext
    public let encryptedNote: Data
    /// 32-byte value commitment
    public let cvNet: Data
    /// 64-byte spend authorization signature
    public let spendAuthSig: Data

    public init(
        nullifier: Data,
        rk: Data,
        cmx: Data,
        encryptedNote: Data,
        cvNet: Data,
        spendAuthSig: Data
    ) {
        self.nullifier = nullifier
        self.rk = rk
        self.cmx = cmx
        self.encryptedNote = encryptedNote
        self.cvNet = cvNet
        self.spendAuthSig = spendAuthSig
    }
}

/// Orchard bundle parameters (high-level Swift representation).
/// Clients construct the Orchard proof and signatures independently,
/// then pass these parameters to shielded transition functions.
public struct OrchardBundle: Sendable {
    /// Actions in the bundle
    public let actions: [OrchardAction]
    /// 32-byte anchor (tree root)
    public let anchor: Data
    /// Variable-length Orchard proof bytes
    public let proof: Data
    /// 64-byte binding signature
    public let bindingSignature: Data

    public init(
        actions: [OrchardAction],
        anchor: Data,
        proof: Data,
        bindingSignature: Data
    ) {
        self.actions = actions
        self.anchor = anchor
        self.proof = proof
        self.bindingSignature = bindingSignature
    }
}

/// Input for shield operation: address + amount + private key.
public struct ShieldFundsInput: Sendable {
    /// Platform address bytes
    public let address: Data
    /// Amount to shield from this address (in credits)
    public let amount: UInt64
    /// 32-byte private key for signing
    public let privateKey: Data

    public init(address: Data, amount: UInt64, privateKey: Data) {
        self.address = address
        self.amount = amount
        self.privateKey = privateKey
    }
}

/// Withdrawal pooling mode.
public enum WithdrawalPooling: UInt8, Sendable {
    /// Never pool withdrawals
    case never = 0
    /// Pool if available
    case ifAvailable = 1
    /// Standard pooling
    case standard = 2
}

/// Status of a single nullifier as returned by query.
public struct NullifierStatus: Sendable {
    /// The 32-byte nullifier hash
    public let nullifier: Data
    /// Whether this nullifier has been spent
    public let isSpent: Bool

    public init(nullifier: Data, isSpent: Bool) {
        self.nullifier = nullifier
        self.isSpent = isSpent
    }
}

/// An encrypted note from the shielded pool.
public struct EncryptedNote: Sendable {
    /// Note commitment (hex-decoded from cmx)
    public let cmx: Data
    /// Nullifier hash
    public let nullifier: Data
    /// Encrypted note ciphertext
    public let encryptedNote: Data

    public init(cmx: Data, nullifier: Data, encryptedNote: Data) {
        self.cmx = cmx
        self.nullifier = nullifier
        self.encryptedNote = encryptedNote
    }
}

// MARK: - FFI Conversion Helpers

/// Helper that builds an array of FFISerializedAction structs from OrchardAction models.
/// The returned array and its backing Data objects must remain alive during the FFI call.
///
/// Returns (ffiActions, backingStorage) where backingStorage keeps encrypted note Data alive.
func buildFFIActions(
    from actions: [OrchardAction]
) -> ([FFISerializedAction], [Data]) {
    var ffiActions: [FFISerializedAction] = []
    // Keep encrypted note Data objects alive so pointers remain valid.
    var storage: [Data] = []

    for action in actions {
        storage.append(action.encryptedNote)

        let ffiAction = FFISerializedAction(
            nullifier: dataToBytes32(action.nullifier),
            rk: dataToBytes32(action.rk),
            cmx: dataToBytes32(action.cmx),
            encrypted_note: nil, // set below via withUnsafeBytes during call
            encrypted_note_len: action.encryptedNote.count,
            cv_net: dataToBytes32(action.cvNet),
            spend_auth_sig: dataToBytes64(action.spendAuthSig)
        )
        ffiActions.append(ffiAction)
    }

    return (ffiActions, storage)
}

/// Decode a hex string to Data. Returns nil on invalid hex.
func hexToData(_ hex: String) -> Data? {
    let cleaned = hex.trimmingCharacters(in: .whitespacesAndNewlines)
    guard cleaned.count % 2 == 0 else { return nil }
    let len = cleaned.count / 2
    var data = Data(capacity: len)
    var index = cleaned.startIndex
    for _ in 0..<len {
        let nextIndex = cleaned.index(index, offsetBy: 2)
        guard let byte = UInt8(cleaned[index..<nextIndex], radix: 16) else { return nil }
        data.append(byte)
        index = nextIndex
    }
    return data
}
