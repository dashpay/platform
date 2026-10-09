import Foundation

/// An exact positive DASH amount. Input uses ASCII digits and a period decimal
/// separator, without grouping, signs, or exponent notation. Unsupported locale
/// separators are rejected rather than partially parsed by Foundation.
public struct ShieldedTipAmount: Equatable, Sendable {
    public let credits: UInt64

    public init?(_ input: String) {
        let parts = input.split(separator: ".", omittingEmptySubsequences: false)
        guard (1...2).contains(parts.count),
              !parts[0].isEmpty,
              parts.allSatisfy({ !$0.isEmpty && $0.utf8.allSatisfy { (48...57).contains($0) } }),
              let whole = UInt64(parts[0]) else { return nil }
        let fraction = parts.count == 2 ? String(parts[1]) : ""
        // Additional trailing zeros are exact and safe; extra nonzero digits
        // would represent a fraction of a credit.
        guard fraction.dropFirst(11).allSatisfy({ $0 == "0" }) else { return nil }
        let digits = String(fraction.prefix(11))
        let fractionalCredits = UInt64(digits + String(repeating: "0", count: 11 - digits.count))!
        let (integralCredits, overflow) = whole.multipliedReportingOverflow(by: 100_000_000_000)
        let (credits, additionOverflow) = integralCredits.addingReportingOverflow(fractionalCredits)
        guard !overflow, !additionOverflow, credits > 0 else { return nil }
        self.credits = credits
    }

    /// Canonical confirmation text derived from the exact amount sent.
    public var dashString: String {
        let whole = credits / 100_000_000_000
        let fraction = credits % 100_000_000_000
        guard fraction != 0 else { return String(whole) }
        var digits = String(fraction)
        digits = String(repeating: "0", count: 11 - digits.count) + digits
        while digits.last == "0" { digits.removeLast() }
        return "\(whole).\(digits)"
    }
}
