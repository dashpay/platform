//
//  Identifiers.swift
//  SwiftExampleAppUITests
//
//  Accessibility identifiers used across the UI test suite. Relocated from
//  SwiftExampleAppUITests.swift so multiple test classes can share them.
//  Identifier strings are byte-for-byte the same as before; do not edit
//  without auditing every test that matches against them.
//

import Foundation

enum Identifier {
    static let walletsTab = "rootTab.wallets"
    static let identitiesTab = "rootTab.identities"
    static let settingsTab = "rootTab.settings"
    static let walletsScreen = "wallets.screen"
    static let addWalletButton = "wallets.addWalletButton"
    static let emptyCreateWalletButton = "wallets.empty.createWalletButton"
    static let walletNameField = "createWallet.walletNameField"
    static let pinField = "createWallet.pinField"
    static let confirmPinField = "createWallet.confirmPinField"
    static let createWalletButton = "createWallet.createButton"
    static let importToggle = "createWallet.importToggle"
    static let mnemonicField = "createWallet.mnemonicField"
    static let wroteItDownToggle = "seedBackup.wroteItDownToggle"
    static let confirmSeedCreateWalletButton = "seedBackup.createWalletButton"
    static let walletInfoButton = "walletDetail.infoButton"
    static let deleteWalletButton = "walletInfo.deleteWalletButton"

    enum Options {
        static let networkPicker = "options.networkPicker"
        static let networkStatusLabel = "options.networkStatusLabel"
    }

    enum Identities {
        static let addMenu = "identities.addMenu"
        static let searchWalletsMenuItem = "identities.searchWalletsMenuItem"

        /// Interpolate with a base58 identity ID — e.g. `Identifier.Identities.row("3ou…AAA")`.
        static func row(_ identityIdBase58: String) -> String {
            "identities.row.\(identityIdBase58)"
        }
    }

    enum SearchWallets {
        static let walletPicker = "searchWallets.walletPicker"
        static let searchButton = "searchWallets.searchButton"
        static let foundCountLabel = "searchWallets.foundCountLabel"
    }

    enum IdentityDetail {
        static let balanceLabel = "identityDetail.balanceLabel"
    }

    enum Transition {
        static let senderIdentityPicker = "transition.senderIdentityPicker"
        static let executeButton = "transition.executeButton"
        static let resultStatusLabel = "transition.resultStatusLabel"

        /// Per-row sender option in the senderIdentityPicker menu.
        static func senderIdentityOption(_ identityIdBase58: String) -> String {
            "transition.senderIdentityOption.\(identityIdBase58)"
        }

        /// Generic input wrapper id — covers `toIdentityId`, `amount`, etc.
        static func input(_ name: String) -> String {
            "transition.input.\(name)"
        }

        /// Recipient picker (multi-identity branch); contains a `Manually Enter Recipient` option.
        static func recipientPicker(_ inputName: String) -> String {
            "transition.input.\(inputName).recipientPicker"
        }

        /// Escape-hatch button shown only when no other identities exist.
        static func manualEntryButton(_ inputName: String) -> String {
            "transition.input.\(inputName).manualEntryButton"
        }

        /// Free-form recipient TextField, visible after either branch hits `useManualEntry = true`.
        static func manualEntryField(_ inputName: String) -> String {
            "transition.input.\(inputName).manualEntryField"
        }
    }
}
