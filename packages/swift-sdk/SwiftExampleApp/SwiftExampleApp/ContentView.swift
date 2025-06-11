import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    
    var body: some View {
        TabView {
            IdentitiesView()
                .tabItem {
                    Label("Identities", systemImage: "person.3")
                }
            
            TokensView()
                .tabItem {
                    Label("Tokens", systemImage: "dollarsign.circle")
                }
            
            DocumentsView()
                .tabItem {
                    Label("Documents", systemImage: "doc.text")
                }
            
            OptionsView()
                .tabItem {
                    Label("Options", systemImage: "gearshape")
                }
        }
        .overlay {
            if appState.isLoading {
                ProgressView("Loading...")
                    .padding()
                    .background(Color.gray.opacity(0.9))
                    .cornerRadius(10)
            }
        }
        .alert("Error", isPresented: $appState.showError) {
            Button("OK") {
                appState.showError = false
            }
        } message: {
            Text(appState.errorMessage)
        }
    }
}