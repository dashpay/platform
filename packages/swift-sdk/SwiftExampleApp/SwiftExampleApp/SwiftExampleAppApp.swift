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
    @StateObject private var appState = AppState()
    
    let modelContainer: ModelContainer
    
    init() {
        do {
            self.modelContainer = try ModelContainer.appContainer()
        } catch {
            fatalError("Failed to create model container: \(error)")
        }
    }
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
                .modelContainer(modelContainer)
                .onAppear {
                    appState.initializeSDK(modelContext: modelContainer.mainContext)
                }
        }
    }
}
