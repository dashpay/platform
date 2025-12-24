import Foundation

enum TokenAction: String, CaseIterable, Identifiable {
    var id: String { self.rawValue }
    case transfer = "Transfer"
    case mint = "Mint"
    case burn = "Burn"
    case claim = "Claim"
    case freeze = "Freeze"
    case unfreeze = "Unfreeze"
    case destroyFrozenFunds = "Destroy Frozen Funds"
    case directPurchase = "Direct Purchase"
    
    var systemImage: String {
        switch self {
        case .transfer: return "arrow.left.arrow.right"
        case .mint: return "plus.circle"
        case .burn: return "flame"
        case .claim: return "gift"
        case .freeze: return "snowflake"
        case .unfreeze: return "sun.max"
        case .destroyFrozenFunds: return "trash"
        case .directPurchase: return "cart"
        }
    }
    
    var isEnabled: Bool {
        // All actions are now enabled
        return true
    }
    
    var description: String {
        switch self {
        case .transfer:
            return "Transfer tokens to another identity"
        case .mint:
            return "Create new tokens (requires permission)"
        case .burn:
            return "Permanently destroy tokens"
        case .claim:
            return "Claim tokens from distribution"
        case .freeze:
            return "Temporarily lock tokens"
        case .unfreeze:
            return "Unlock frozen tokens"
        case .destroyFrozenFunds:
            return "Destroy frozen tokens"
        case .directPurchase:
            return "Purchase tokens directly"
        }
    }
}