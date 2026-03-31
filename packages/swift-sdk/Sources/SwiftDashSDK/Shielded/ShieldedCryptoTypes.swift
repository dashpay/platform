// ShieldedCryptoTypes.swift
// SwiftDashSDK
//
// Swift types for shielded crypto operations (decrypted notes, spendable notes).
// These complement the types in ShieldedTypes.swift with models specific to
// the client-side crypto FFI functions (decrypt_notes, build_*_bundle).

import Foundation

// MARK: - Decrypted Note

/// A successfully decrypted Orchard note returned by `decryptNotes`.
public struct DecryptedNote: Sendable {
    /// Position index of this note in the encrypted notes array that was passed to decryption.
    public let position: Int
    /// Note value in credits.
    public let value: UInt64
    /// 32-byte nullifier hash.
    public let nullifier: Data
    /// 32-byte note commitment (cmx).
    public let cmx: Data
    /// 43-byte Orchard payment address this note is sent to.
    public let address: Data
    /// 32-byte Rho (nullifier randomness domain separator).
    public let rho: Data
    /// 32-byte random seed used to derive note encryption key.
    public let rseed: Data

    public init(
        position: Int,
        value: UInt64,
        nullifier: Data,
        cmx: Data,
        address: Data,
        rho: Data,
        rseed: Data
    ) {
        self.position = position
        self.value = value
        self.nullifier = nullifier
        self.cmx = cmx
        self.address = address
        self.rho = rho
        self.rseed = rseed
    }
}

// MARK: - Spendable Note Info

/// Information about a spendable note, used as input to bundle building functions.
///
/// This contains the full note data plus the Merkle authentication path needed
/// to prove the note exists in the commitment tree.
public struct SpendableNoteInfo: Sendable {
    /// 43-byte Orchard payment address.
    public let address: Data
    /// Note value in credits.
    public let value: UInt64
    /// 32-byte Rho.
    public let rho: Data
    /// 32-byte random seed.
    public let rseed: Data
    /// Position of the note in the commitment tree.
    public let position: UInt32
    /// Merkle authentication path: array of 32 entries, each a 32-byte hash.
    public let merklePath: [Data]

    public init(
        address: Data,
        value: UInt64,
        rho: Data,
        rseed: Data,
        position: UInt32,
        merklePath: [Data]
    ) {
        self.address = address
        self.value = value
        self.rho = rho
        self.rseed = rseed
        self.position = position
        self.merklePath = merklePath
    }

    /// Convert this spendable note to a dictionary matching the JSON format expected by Rust.
    ///
    /// Format:
    /// ```json
    /// {
    ///   "address": "hex43bytes",
    ///   "value": 100000,
    ///   "rho": "hex32bytes",
    ///   "rseed": "hex32bytes",
    ///   "position": 42,
    ///   "merklePath": ["hex32bytes", ...]
    /// }
    /// ```
    /// Validate field sizes before serialization.
    public func validate() throws {
        guard address.count == 43 else {
            throw SDKError.invalidParameter("SpendableNoteInfo address must be 43 bytes, got \(address.count)")
        }
        guard rho.count == 32 else {
            throw SDKError.invalidParameter("SpendableNoteInfo rho must be 32 bytes, got \(rho.count)")
        }
        guard rseed.count == 32 else {
            throw SDKError.invalidParameter("SpendableNoteInfo rseed must be 32 bytes, got \(rseed.count)")
        }
        guard merklePath.count == 32 else {
            throw SDKError.invalidParameter("SpendableNoteInfo merklePath must have 32 entries, got \(merklePath.count)")
        }
        for (i, node) in merklePath.enumerated() {
            guard node.count == 32 else {
                throw SDKError.invalidParameter("SpendableNoteInfo merklePath[\(i)] must be 32 bytes, got \(node.count)")
            }
        }
    }

    public func toJSON() -> [String: Any] {
        return [
            "address": address.toHexString(),
            "value": value,
            "rho": rho.toHexString(),
            "rseed": rseed.toHexString(),
            "position": position,
            "merklePath": merklePath.map { $0.toHexString() }
        ]
    }
}

// MARK: - JSON Serialization Helpers

/// Convert an array of EncryptedNote models to a JSON string for the decrypt_notes FFI function.
///
/// The Rust FFI expects a JSON array of objects:
/// ```json
/// [{ "cmx": "hex32", "nullifier": "hex32", "encryptedNote": "hex216" }]
/// ```
func encryptedNotesToJSON(_ notes: [EncryptedNote]) -> String {
    let jsonArray: [[String: Any]] = notes.map { note in
        [
            "cmx": note.cmx.toHexString(),
            "nullifier": note.nullifier.toHexString(),
            "encryptedNote": note.encryptedNote.toHexString()
        ]
    }

    guard let data = try? JSONSerialization.data(withJSONObject: jsonArray, options: []),
          let string = String(data: data, encoding: .utf8)
    else {
        return "[]"
    }
    return string
}

/// Convert an array of SpendableNoteInfo models to a JSON string for bundle building FFI functions.
///
/// The Rust FFI expects a JSON array of objects with camelCase keys:
/// ```json
/// [{ "address": "hex43", "value": u64, "rho": "hex32", "rseed": "hex32",
///    "position": u32, "merklePath": ["hex32", ...32 entries] }]
/// ```
func spendableNotesToJSON(_ notes: [SpendableNoteInfo]) throws -> String {
    for (i, note) in notes.enumerated() {
        do {
            try note.validate()
        } catch {
            throw SDKError.invalidParameter("SpendableNoteInfo[\(i)]: \(error.localizedDescription)")
        }
    }
    let jsonArray: [[String: Any]] = notes.map { $0.toJSON() }

    guard let data = try? JSONSerialization.data(withJSONObject: jsonArray, options: []),
          let string = String(data: data, encoding: .utf8)
    else {
        return "[]"
    }
    return string
}

// MARK: - Shielded Bundle Handle

/// Opaque handle to a heap-allocated Orchard bundle (DashSDKOrchardBundleParams).
///
/// Returned by the `buildShieldBundle`, `buildTransferBundle`, etc. functions.
/// Pass directly to transition functions (`shieldFunds`, `shieldedTransfer`, etc.)
/// via the overloads that accept `ShieldedBundleHandle`.
///
/// Automatically freed when deallocated — do NOT call `dash_sdk_shielded_bundle_params_free`
/// manually on a handle that is managed by this class.
public final class ShieldedBundleHandle: @unchecked Sendable {
    let ptr: UnsafeMutablePointer<FFIOrchardBundleParams>

    init(_ ptr: UnsafeMutablePointer<FFIOrchardBundleParams>) {
        self.ptr = ptr
    }

    deinit {
        dash_sdk_shielded_bundle_params_free(ptr)
    }
}

/// Parse a JSON string returned by the decrypt_notes FFI function into DecryptedNote models.
///
/// The JSON format is:
/// ```json
/// [{ "position": idx, "value": u64, "nullifier": "hex32", "cmx": "hex32",
///    "address": "hex43", "rho": "hex32", "rseed": "hex32" }]
/// ```
func parseDecryptedNotesJSON(_ jsonString: String) throws -> [DecryptedNote] {
    guard let jsonData = jsonString.data(using: .utf8) else {
        throw SDKError.serializationError("Decrypted notes JSON is not valid UTF-8")
    }

    guard let jsonArray = try JSONSerialization.jsonObject(with: jsonData) as? [[String: Any]] else {
        throw SDKError.serializationError("Decrypted notes JSON root is not an array")
    }

    var notes: [DecryptedNote] = []
    for (i, obj) in jsonArray.enumerated() {
        guard let positionNum = obj["position"] as? NSNumber,
              let valueNum = obj["value"] as? NSNumber,
              let nullifierHex = obj["nullifier"] as? String,
              let cmxHex = obj["cmx"] as? String,
              let addressHex = obj["address"] as? String,
              let rhoHex = obj["rho"] as? String,
              let rseedHex = obj["rseed"] as? String
        else {
            throw SDKError.serializationError("DecryptedNote[\(i)] is missing required fields")
        }

        let position = positionNum.intValue
        let value = valueNum.uint64Value

        guard let nullifier = hexToData(nullifierHex),
              let cmx = hexToData(cmxHex),
              let address = hexToData(addressHex),
              let rho = hexToData(rhoHex),
              let rseed = hexToData(rseedHex)
        else {
            throw SDKError.serializationError("DecryptedNote[\(i)] contains invalid hex")
        }

        // Validate field sizes
        guard nullifier.count == 32 else {
            throw SDKError.serializationError("DecryptedNote[\(i)] nullifier must be 32 bytes, got \(nullifier.count)")
        }
        guard cmx.count == 32 else {
            throw SDKError.serializationError("DecryptedNote[\(i)] cmx must be 32 bytes, got \(cmx.count)")
        }
        guard address.count == 43 else {
            throw SDKError.serializationError("DecryptedNote[\(i)] address must be 43 bytes, got \(address.count)")
        }
        guard rho.count == 32 else {
            throw SDKError.serializationError("DecryptedNote[\(i)] rho must be 32 bytes, got \(rho.count)")
        }
        guard rseed.count == 32 else {
            throw SDKError.serializationError("DecryptedNote[\(i)] rseed must be 32 bytes, got \(rseed.count)")
        }

        notes.append(DecryptedNote(
            position: position,
            value: value,
            nullifier: nullifier,
            cmx: cmx,
            address: address,
            rho: rho,
            rseed: rseed
        ))
    }

    return notes
}
