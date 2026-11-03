import Foundation

extension UInt64 {
    /// Read a protocol `u64` out of a JSON value produced by DPP.
    ///
    /// DPP marks its `u64` fields with `#[json_safe_fields]`, which writes a
    /// value as a JSON NUMBER while it fits in a JavaScript safe integer
    /// (`2^53 - 1`, 9007199254740991) and as a DECIMAL STRING above that, so
    /// that a JavaScript client cannot silently round it. A cast that accepts
    /// only numbers therefore drops exactly the large values: a key whose
    /// `totalBudget` arrives as `"9007199254740992"` would be reconstructed
    /// locally as a key with no budget at all.
    ///
    /// Accepts a JSON number that is non-negative and integral, or a decimal
    /// string. Returns `nil` for `nil`, `NSNull`, a negative or fractional
    /// number, a value outside the `UInt64` range, a boolean, and anything
    /// that is not a number or a string.
    init?(jsonValue: Any?) {
        switch jsonValue {
        case let text as String:
            // The shape a value above the safe-integer ceiling arrives in.
            // `UInt64(_:)` already rejects a sign, padding, separators and
            // anything else that is not plain decimal digits.
            guard let parsed = UInt64(text) else { return nil }
            self = parsed
        case let number as NSNumber:
            // A JSON boolean parses into an `NSNumber` too, and that one DOES
            // answer an integer cast, with 1 or 0. Reject it by its CoreFoundation
            // type: reading `true` as a budget of one credit would be worse
            // than reading it as absent.
            guard CFGetTypeID(number) != CFBooleanGetTypeID() else { return nil }
            // Otherwise the bridge's conditional cast is exact (SE-0170): a
            // negative, fractional or out-of-range value fails rather than
            // being rounded or wrapped.
            guard let parsed = number as? UInt64 else { return nil }
            self = parsed
        default:
            return nil
        }
    }
}
