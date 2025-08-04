import SwiftUI
import SwiftData
import UIKit

struct DataContractDetailsView: View {
    let contract: PersistentDataContract
    @Environment(\.dismiss) var dismiss
    @Environment(\.modelContext) private var modelContext
    @State private var showingShareSheet = false
    
    var body: some View {
        NavigationView {
            List {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Contract Information")
                        .font(.headline)
                        .padding(.bottom, 4)
                    
                    HStack {
                        Text("Name:")
                            .foregroundColor(.secondary)
                        Text(contract.name)
                    }
                    
                    HStack {
                        Text("ID:")
                            .foregroundColor(.secondary)
                        Text(contract.idBase58)
                            .font(.caption)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                    
                    HStack {
                        Text("Size:")
                            .foregroundColor(.secondary)
                        Text(ByteCountFormatter.string(fromByteCount: Int64(contract.serializedContract.count), countStyle: .binary))
                    }
                    
                    HStack {
                        Text("Created:")
                            .foregroundColor(.secondary)
                        Text(contract.createdAt, style: .date)
                    }
                    
                    HStack {
                        Text("Last Used:")
                            .foregroundColor(.secondary)
                        Text(contract.lastAccessedAt, style: .relative)
                    }
                }
                .padding(.vertical)
                
                Button(action: { showingShareSheet = true }) {
                    Label("Export Contract", systemImage: "square.and.arrow.up")
                        .foregroundColor(.blue)
                }
                .padding(.vertical)
            }
            .navigationTitle("Contract Details")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
            .sheet(isPresented: $showingShareSheet) {
                if let url = exportContract() {
                    ShareSheet(items: [url])
                }
            }
        }
        .onAppear {
            updateLastAccessedDate()
        }
    }
    
    private func exportContract() -> URL? {
        do {
            let fileName = "\(contract.name.replacingOccurrences(of: " ", with: "_"))_\(contract.idBase58.prefix(8)).json"
            let tempURL = FileManager.default.temporaryDirectory.appendingPathComponent(fileName)
            
            try contract.serializedContract.write(to: tempURL)
            return tempURL
        } catch {
            print("Failed to export contract: \(error)")
            return nil
        }
    }
    
    private func updateLastAccessedDate() {
        contract.lastAccessedAt = Date()
        do {
            try modelContext.save()
        } catch {
            print("Failed to update last accessed date: \(error)")
        }
    }
}

struct ShareSheet: UIViewControllerRepresentable {
    let items: [Any]
    
    func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }
    
    func updateUIViewController(_ uiViewController: UIActivityViewController, context: Context) {}
}

#Preview {
    DataContractDetailsView(
        contract: PersistentDataContract(
            id: Data(),
            name: "Sample Contract",
            serializedContract: Data()
        )
    )
}