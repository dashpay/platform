//
//  SwiftExampleAppApp.swift
//  SwiftExampleApp
//
//  Created by Sam Westrich on 8/6/25.
//

import SwiftUI
import SwiftData

// Pull in logging helpers for SDKLogger and presets
import SwiftDashSDK

@main
struct SwiftExampleAppApp: App {
    @StateObject private var unifiedState = UnifiedAppState()

    init() {
        // Suppress auto layout constraint warnings in debug builds
        // These are typically harmless keyboard-related warnings
        #if DEBUG
        UserDefaults.standard.set(false, forKey: "_UIConstraintBasedLayoutLogUnsatisfiable")
        #endif
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(unifiedState)
                .environmentObject(unifiedState.walletService)
                .environmentObject(unifiedState.platformState)
                .environmentObject(unifiedState.shieldedService)
                .environmentObject(unifiedState.platformBalanceSyncService)
                .environmentObject(unifiedState.zkSyncService)
                .environment(\.modelContext, unifiedState.modelContainer.mainContext)
                .task {
                    SDKLogger.log("🚀 SwiftExampleApp: Starting initialization...", minimumLevel: .medium)
                    await unifiedState.initialize()
                    SDKLogger.log("🚀 SwiftExampleApp: Initialization complete", minimumLevel: .medium)
                }
        }
    }
}
