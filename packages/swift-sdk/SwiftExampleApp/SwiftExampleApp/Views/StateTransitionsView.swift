import SwiftUI
import SwiftDashSDK
import DashSDKFFI

struct StateTransitionsView: View {
    @EnvironmentObject var appState: UnifiedAppState
    
    enum TransitionCategory: String, CaseIterable {
        case identity = "Identity"
        case dataContract = "Data Contract"
        case document = "Document"
        case token = "Token"
        case voting = "Voting"
        
        var icon: String {
            switch self {
            case .identity: return "person.fill"
            case .dataContract: return "doc.text.fill"
            case .document: return "doc.fill"
            case .token: return "bitcoinsign.circle.fill"
            case .voting: return "hand.raised.fill"
            }
        }
        
        var description: String {
            switch self {
            case .identity: return "Create, update, and manage identities"
            case .dataContract: return "Deploy and update data contracts"
            case .document: return "Create and manage documents"
            case .token: return "Mint, transfer, and manage tokens"
            case .voting: return "Participate in governance voting"
            }
        }
    }
    
    var body: some View {
        List {
            ForEach(TransitionCategory.allCases, id: \.self) { category in
                NavigationLink(destination: TransitionCategoryView(category: category)) {
                    HStack(spacing: 16) {
                        Image(systemName: category.icon)
                            .font(.title2)
                            .foregroundColor(.blue)
                            .frame(width: 30)
                        
                        VStack(alignment: .leading, spacing: 4) {
                            Text(category.rawValue)
                                .font(.headline)
                            Text(category.description)
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(2)
                        }
                        
                        Spacer()
                    }
                    .padding(.vertical, 8)
                }
            }
        }
        .navigationTitle("State Transitions")
        .navigationBarTitleDisplayMode(.large)
    }
}

// Preview
struct StateTransitionsView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationView {
            StateTransitionsView()
                .environmentObject(UnifiedAppState())
        }
    }
}