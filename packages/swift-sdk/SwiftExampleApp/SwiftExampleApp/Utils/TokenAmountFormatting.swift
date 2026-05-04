import Foundation

/// Conversions between a token's *display unit* (what the user types
/// and reads — e.g. `4.44`) and its *raw on-chain unit* (what the FFI
/// expects — `UInt64`, scaled by `decimals`). Without these, a token
/// with 8 decimals and a balance of `444667781` raw units would render
/// as `4.44667781` while a typed amount of `5` would be sent through
/// as `5` raw units (`0.00000005`), silently sneaking past every
/// `amount <= balance` check.
///
/// Both helpers use `Decimal` (not `Double`) so high-decimal tokens
/// with large balances don't lose precision.

/// Parse a user-entered amount in display units into raw on-chain
/// units by multiplying by `10^decimals`. Accepts both "." and ","
/// as the decimal separator so users in either locale can type
/// naturally. Returns `nil` for empty / unparseable / negative /
/// out-of-`UInt64`-range input.
///
/// The grammar is deliberately strict: digits, optionally followed by
/// a single separator and more digits — *no* thousands separators, no
/// trailing junk, no scientific notation. `Decimal(string:)` on its
/// own happily accepts a valid prefix and silently drops the rest, so
/// `"5abc"` would parse as `5` and a pasted `"1,234.56"` (after the
/// `,` → `.` normalization) would become `"1.234.56"` and parse as
/// `1.234`. Either of those would let the user submit a materially
/// different raw amount than what they think they typed.
///
/// Excess fractional digits beyond `decimals` are truncated (rounded
/// down). Silently rounding *up* would let the user submit slightly
/// more than they typed, which would surprise on a balance edge.
func parseTokenAmount(_ text: String, decimals: Int) -> UInt64? {
    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty else { return nil }

    let normalized = trimmed.replacingOccurrences(of: ",", with: ".")

    // Strict grammar: `\d+(\.\d+)?` or `\.\d+` (a leading-dot like
    // ".5" is what `.decimalPad` actually emits in some locales).
    guard isWellFormedDecimal(normalized) else { return nil }

    guard let entered = Decimal(string: normalized), entered >= 0 else {
        return nil
    }

    let dec = max(0, decimals)
    let multiplier = pow(Decimal(10), dec)
    var scaled = entered * multiplier
    var rounded = Decimal()
    NSDecimalRound(&rounded, &scaled, 0, .down)

    if rounded < 0 || rounded > Decimal(UInt64.max) { return nil }
    return (rounded as NSDecimalNumber).uint64Value
}

private func isWellFormedDecimal(_ s: String) -> Bool {
    var sawDigit = false
    var sawDot = false
    for ch in s {
        if ch.isASCII, let ascii = ch.asciiValue, ascii >= 0x30 && ascii <= 0x39 {
            sawDigit = true
        } else if ch == "." {
            if sawDot { return false }
            sawDot = true
        } else {
            return false
        }
    }
    return sawDigit
}

/// Format a raw u64 token amount with the given decimals. Uses the
/// current locale's grouping/decimal separators (so European users
/// see `4,44667781` and US users see `4.44667781`). Mirrors the
/// formatter used by `IdentityTokenRow.formattedBalance` in
/// `IdentityDetailView`.
func formatTokenAmount(_ raw: UInt64, decimals: Int) -> String {
    let dec = max(0, decimals)
    let rawDecimal = Decimal(raw)
    let divisor = pow(Decimal(10), dec)
    let scaled = divisor == 0 ? rawDecimal : (rawDecimal / divisor)
    let formatter = NumberFormatter()
    formatter.numberStyle = .decimal
    formatter.maximumFractionDigits = dec
    formatter.minimumFractionDigits = 0
    formatter.usesGroupingSeparator = true
    return formatter.string(from: scaled as NSNumber) ?? "\(raw)"
}
