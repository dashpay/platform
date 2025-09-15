import SwiftUI

struct SelectMainNameView: View {
    let identity: IdentityModel
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    
    @State private var selectedName: String?
    
    var availableNames: [String] {
        // Only show non-contested names that the user actually owns
        identity.dpnsNames
    }
    
    var body: some View {
        NavigationView {
            Form {
                Section {
                    Text("Select which name to display as your main identity name throughout the app.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                if availableNames.isEmpty {
                    Section {
                        VStack(spacing: 12) {
                            Image(systemName: "exclamationmark.triangle")
                                .font(.largeTitle)
                                .foregroundColor(.orange)
                            Text("No Names Available")
                                .font(.headline)
                            Text("You don't have any registered DPNS names yet. Contested names cannot be selected as main names.")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .multilineTextAlignment(.center)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical)
                    }
                } else {
                    Section("Available Names") {
                        // Option to have no main name
                        HStack {
                            Text("None")
                                .foregroundColor(.secondary)
                            Spacer()
                            if selectedName == nil {
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundColor(.blue)
                            }
                        }
                        .contentShape(Rectangle())
                        .onTapGesture {
                            selectedName = nil
                        }
                        
                        // List all available names
                        ForEach(availableNames, id: \.self) { name in
                            HStack {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(name)
                                        .font(.headline)
                                    if name == identity.dpnsName {
                                        Text("First registered name")
                                            .font(.caption)
                                            .foregroundColor(.secondary)
                                    }
                                }
                                
                                Spacer()
                                
                                if selectedName == name {
                                    Image(systemName: "checkmark.circle.fill")
                                        .foregroundColor(.blue)
                                }
                            }
                            .contentShape(Rectangle())
                            .onTapGesture {
                                selectedName = name
                            }
                        }
                    }
                    
                    // Show current selection
                    if let currentMain = identity.mainDpnsName {
                        Section("Current Main Name") {
                            HStack {
                                Text(currentMain)
                                    .font(.headline)
                                Spacer()
                                Image(systemName: "star.fill")
                                    .foregroundColor(.yellow)
                            }
                        }
                    }
                }
                
                // Show contested names as information only
                if !identity.contestedDpnsNames.isEmpty {
                    Section("Contested Names") {
                        ForEach(identity.contestedDpnsNames, id: \.self) { name in
                            HStack {
                                Text(name)
                                    .foregroundColor(.secondary)
                                Spacer()
                                Label("Contested", systemImage: "flag.fill")
                                    .font(.caption)
                                    .foregroundColor(.orange)
                            }
                        }
                        
                        Text("Contested names cannot be selected as main names until they are won.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .navigationTitle("Select Main Name")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Save") {
                        saveSelection()
                    }
                    .disabled(selectedName == identity.mainDpnsName)
                }
            }
            .onAppear {
                // Initialize with current main name
                selectedName = identity.mainDpnsName
            }
        }
    }
    
    private func saveSelection() {
        // Update the identity with the new main name
        if let index = appState.identities.firstIndex(where: { $0.id == identity.id }) {
            var updatedIdentity = appState.identities[index]
            updatedIdentity.mainDpnsName = selectedName
            appState.identities[index] = updatedIdentity
            
            // Persist the selection
            appState.updateIdentityMainName(id: identity.id, mainName: selectedName)
        }
        
        dismiss()
    }
}