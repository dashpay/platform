import SwiftUI
import SwiftDashSDK

/// Direction icon and colour shared by the transaction list and detail views.
enum TransactionDirectionStyle {
    /// `nil` (no accounting from Rust for this wallet) reads as unknown.
    static func icon(for direction: UInt32?) -> String {
        guard let direction else { return "questionmark.circle" }
        switch direction {
        case CoreDirectionCode.incoming: return "arrow.down.circle.fill"
        case CoreDirectionCode.outgoing: return "arrow.up.circle.fill"
        case CoreDirectionCode.internalTransfer: return "arrow.triangle.2.circlepath"
        case CoreDirectionCode.coinJoin: return "shuffle.circle.fill"
        default: return "questionmark.circle"
        }
    }

    /// Internal transfers share the outgoing colour: they still pay a fee.
    static func color(for direction: UInt32?) -> Color {
        guard let direction else { return .secondary }
        switch direction {
        case CoreDirectionCode.incoming: return .green
        case CoreDirectionCode.outgoing, CoreDirectionCode.internalTransfer: return .red
        case CoreDirectionCode.coinJoin: return .blue
        default: return .secondary
        }
    }
}
