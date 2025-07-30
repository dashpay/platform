//
//  SwiftExampleAppApp.swift
//  SwiftExampleApp
//
//  Created by Sam Westrich on 8/6/25.
//

import SwiftUI
import SwiftData

@main
struct SwiftExampleAppApp: App {
    @StateObject private var unifiedState = UnifiedAppState()
    @State private var shouldResetApp = false
    
    var body: some Scene {
        WindowGroup {
            if shouldResetApp {
                // Show reset view
                VStack(spacing: 20) {
                    ProgressView("Resetting app...")
                        .scaleEffect(1.5)
                    Text("The app is being reset to its initial state.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .onAppear {
                    Task {
                        try? await Task.sleep(nanoseconds: 1_000_000_000) // 1 second
                        await resetAppState()
                    }
                }
            } else {
                ContentView()
                    .environmentObject(unifiedState)
                    .environmentObject(unifiedState.walletService)
                    .environmentObject(unifiedState.platformState)
                    .environmentObject(unifiedState.unifiedState)
                    .environment(\.modelContext, unifiedState.modelContainer.mainContext)
                    .task {
                        await unifiedState.initialize()
                    }
            }
        }
    }
    
    @MainActor
    private func resetAppState() async {
        await unifiedState.reset()
        await unifiedState.initialize()
        shouldResetApp = false
    }
}
