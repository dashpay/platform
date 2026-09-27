import Foundation

/// The per-property immutability a mutable document type declares
/// (meta-schema v3, protocol version 14).
///
/// `immutableProperties` is the type's `immutable` keyword: top-level
/// properties frozen at document creation while the rest of the document
/// stays replaceable. `immutableAllowSetting` is its `immutableAllowSetting`
/// keyword: the subset a replace may still SET while the stored document has
/// no value for the property, frozen from then on (it can then neither change
/// nor be removed).
///
/// Both lists name TOP-LEVEL properties only: listing an object freezes it
/// whole, nested values included. Consensus enforces both on every replace
/// (`DocumentImmutablePropertyChangedError`, state error code 40128), and a
/// rejected state transition is still paid for, so the UI locks the fields up
/// front rather than letting a user build a transition that cannot pass.
public struct DocumentTypeImmutability: Equatable, Sendable {
    /// Nothing frozen: every pre-v14 document type, and every v14 type that
    /// declares no `immutable` list.
    public static let none = DocumentTypeImmutability(immutable: [], allowSetting: [])

    /// The `immutable` keyword, deduplicated and sorted for display.
    public let immutableProperties: [String]

    /// The `immutableAllowSetting` keyword, deduplicated and sorted, kept
    /// exactly as authored rather than validated: DPP refuses a contract whose
    /// allowance names a property outside `immutable`, so such an entry can
    /// only reach a client through hand-edited JSON. `lockState(for:hasStoredValue:)`
    /// ignores it instead of unlocking anything.
    public let immutableAllowSetting: [String]

    /// True when the type freezes no property at all.
    public var isEmpty: Bool { immutableProperties.isEmpty }

    public init(immutable: [String], allowSetting: [String]) {
        self.immutableProperties = Set(immutable).sorted()
        self.immutableAllowSetting = Set(allowSetting).sorted()
    }

    /// Read both keywords off a document type's schema dictionary, which is
    /// the whole type object as authored in the contract. A missing keyword
    /// freezes nothing, and non-string entries are ignored.
    public init(documentTypeSchema: [String: Any]?) {
        self.init(
            immutable: DocumentTypeImmutability.names(
                documentTypeSchema?["immutable"]),
            allowSetting: DocumentTypeImmutability.names(
                documentTypeSchema?["immutableAllowSetting"])
        )
    }

    private static func names(_ value: Any?) -> [String] {
        guard let entries = value as? [Any] else { return [] }
        return entries.compactMap { $0 as? String }
    }

    /// How a replace form must treat one property of a document.
    public enum PropertyLock: Equatable, Sendable {
        /// Not immutable: editable as usual.
        case editable

        /// Frozen: any change, addition or removal is rejected by consensus.
        case frozen

        /// Listed under `immutableAllowSetting` and absent from the stored
        /// document: may be set exactly once, and is frozen from then on.
        case settableOnce
    }

    /// The lock state of `property` for a replace of a document that
    /// `hasStoredValue` for it. A settable-once property that already has a
    /// value is frozen: the allowance covers only the step from absent to
    /// present.
    public func lockState(for property: String, hasStoredValue: Bool) -> PropertyLock {
        guard immutableProperties.contains(property) else { return .editable }
        if !hasStoredValue && immutableAllowSetting.contains(property) {
            return .settableOnce
        }
        return .frozen
    }
}
