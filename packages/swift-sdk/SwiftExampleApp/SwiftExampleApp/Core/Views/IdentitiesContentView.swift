// IdentitiesContentView.swift
// SwiftExampleApp
//
// Identities-only tab content. Split off from the old combined
// Wallets+Identities screen so each concern gets its own root tab.

import SwiftUI
import SwiftDashSDK
import SwiftData

struct IdentitiesContentView: View {
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService
    @State private var showingLoadIdentity = false
    @State private var showingCreateIdentity = false

    var body: some View {
        List {
            if platformState.identities.isEmpty {
                Section {
                    VStack(spacing: 12) {
                        Image(systemName: "person.crop.circle.badge.plus")
                            .font(.system(size: 40))
                            .foregroundColor(.gray)

                        Text("No Identities")
                            .font(.headline)

                        Text("Load an identity to interact with Dash Platform")
                            .font(.caption)
                            .foregroundColor(.secondary)

                        HStack(spacing: 12) {
                            Button {
                                showingCreateIdentity = true
                            } label: {
                                Label("Create Identity", systemImage: "person.crop.circle.badge.plus")
                                    .padding(.horizontal, 16)
                                    .padding(.vertical, 8)
                            }
                            .buttonStyle(.borderedProminent)

                            Button {
                                showingLoadIdentity = true
                            } label: {
                                Label("Load Identity", systemImage: "square.and.arrow.down")
                                    .padding(.horizontal, 16)
                                    .padding(.vertical, 8)
                            }
                            .buttonStyle(.bordered)
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 20)
                }
            } else {
                Section("Identities") {
                    ForEach(platformState.identities) { identity in
                        IdentityRow(identity: identity)
                            .environmentObject(platformState)
                    }
                }
            }
        }
        .navigationTitle("Identities")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Menu {
                    Button {
                        showingCreateIdentity = true
                    } label: {
                        Label("Create Identity", systemImage: "person.crop.circle.badge.plus")
                    }

                    Button {
                        showingLoadIdentity = true
                    } label: {
                        Label("Load Identity", systemImage: "square.and.arrow.down")
                    }
                } label: {
                    Image(systemName: "plus")
                }
            }
        }
        .sheet(isPresented: $showingLoadIdentity) {
            LoadIdentityView()
                .environmentObject(platformState)
        }
        .sheet(isPresented: $showingCreateIdentity) {
            CreateIdentityView()
                .environmentObject(platformState)
        }
        .refreshable {
            await platformBalanceSyncService.performSync()
        }
    }
}
