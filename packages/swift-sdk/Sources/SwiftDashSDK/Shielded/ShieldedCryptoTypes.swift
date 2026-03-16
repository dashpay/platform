// ShieldedCryptoTypes.swift
// SwiftDashSDK
//
// Swift types for shielded crypto operations (decrypted notes, spendable notes).
// These complement the types in ShieldedTypes.swift with models specific to
// the client-side crypto FFI functions (decrypt_notes, build_*_bundle).

import Foundation

// MARK: - Decrypted Note

/// A successfully decrypted Orchard note returned by `decryptNotes`.
public struct DecryptedNote {
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
public struct SpendableNoteInfo {
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
func spendableNotesToJSON(_ notes: [SpendableNoteInfo]) -> String {
    let jsonArray: [[String: Any]] = notes.map { $0.toJSON() }

    guard let data = try? JSONSerialization.data(withJSONObject: jsonArray, options: []),
          let string = String(data: data, encoding: .utf8)
    else {
        return "[]"
    }
    return string
}

// MARK: - Bundle JSON Parsing

/// Parse a JSON string returned by the bundle building FFI functions into an OrchardBundle.
///
/// The JSON format matches the `BundleJson` struct in Rust:
/// ```json
/// {
///   "actions": [{ "nullifier": "hex", "rk": "hex", "cmx": "hex",
///                  "encryptedNote": "hex", "cvNet": "hex", "spendAuthSig": "hex" }],
///   "anchor": "hex32",
///   "proof": "hexVariable",
///   "bindingSignature": "hex64",
///   "valueBalance": i64
/// }
/// ```
func parseBundleJSON(_ jsonString: String) throws -> OrchardBundle {
    guard let jsonData = jsonString.data(using: .utf8) else {
        throw SDKError.serializationError("Bundle JSON is not valid UTF-8")
    }

    guard let json = try JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
        throw SDKError.serializationError("Bundle JSON root is not an object")
    }

    // Parse actions array
    guard let actionsJSON = json["actions"] as? [[String: Any]] else {
        throw SDKError.serializationError("Bundle JSON missing 'actions' array")
    }

    var actions: [OrchardAction] = []
    for (i, actionObj) in actionsJSON.enumerated() {
        guard let nullifierHex = actionObj["nullifier"] as? String,
              let rkHex = actionObj["rk"] as? String,
              let cmxHex = actionObj["cmx"] as? String,
              let encNoteHex = actionObj["encryptedNote"] as? String,
              let cvNetHex = actionObj["cvNet"] as? String,
              let sigHex = actionObj["spendAuthSig"] as? String
        else {
            throw SDKError.serializationError("Action[\(i)] is missing required hex fields")
        }

        guard let nullifier = hexToData(nullifierHex),
              let rk = hexToData(rkHex),
              let cmx = hexToData(cmxHex),
              let encNote = hexToData(encNoteHex),
              let cvNet = hexToData(cvNetHex),
              let sig = hexToData(sigHex)
        else {
            throw SDKError.serializationError("Action[\(i)] contains invalid hex")
        }

        // Validate field sizes to prevent silent zero-padding
        guard nullifier.count == 32 else {
            throw SDKError.serializationError("Action[\(i)] nullifier must be 32 bytes, got \(nullifier.count)")
        }
        guard rk.count == 32 else {
            throw SDKError.serializationError("Action[\(i)] rk must be 32 bytes, got \(rk.count)")
        }
        guard cmx.count == 32 else {
            throw SDKError.serializationError("Action[\(i)] cmx must be 32 bytes, got \(cmx.count)")
        }
        guard cvNet.count == 32 else {
            throw SDKError.serializationError("Action[\(i)] cvNet must be 32 bytes, got \(cvNet.count)")
        }
        guard sig.count == 64 else {
            throw SDKError.serializationError("Action[\(i)] spendAuthSig must be 64 bytes, got \(sig.count)")
        }

        actions.append(OrchardAction(
            nullifier: nullifier,
            rk: rk,
            cmx: cmx,
            encryptedNote: encNote,
            cvNet: cvNet,
            spendAuthSig: sig
        ))
    }

    // Parse anchor
    guard let anchorHex = json["anchor"] as? String,
          let anchor = hexToData(anchorHex)
    else {
        throw SDKError.serializationError("Bundle JSON missing or invalid 'anchor'")
    }
    guard anchor.count == 32 else {
        throw SDKError.serializationError("anchor must be 32 bytes, got \(anchor.count)")
    }

    // Parse proof
    guard let proofHex = json["proof"] as? String,
          let proof = hexToData(proofHex)
    else {
        throw SDKError.serializationError("Bundle JSON missing or invalid 'proof'")
    }

    // Parse binding signature
    guard let bindingSigHex = json["bindingSignature"] as? String,
          let bindingSig = hexToData(bindingSigHex)
    else {
        throw SDKError.serializationError("Bundle JSON missing or invalid 'bindingSignature'")
    }
    guard bindingSig.count == 64 else {
        throw SDKError.serializationError("bindingSignature must be 64 bytes, got \(bindingSig.count)")
    }

    return OrchardBundle(
        actions: actions,
        anchor: anchor,
        proof: proof,
        bindingSignature: bindingSig
    )
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
