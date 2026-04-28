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
    static let walletsScreen = "wallets.screen"
    static let addWalletButton = "wallets.addWalletButton"
    static let emptyCreateWalletButton = "wallets.empty.createWalletButton"
    static let walletNameField = "createWallet.walletNameField"
    static let pinField = "createWallet.pinField"
    static let confirmPinField = "createWallet.confirmPinField"
    static let createWalletButton = "createWallet.createButton"
    static let wroteItDownToggle = "seedBackup.wroteItDownToggle"
    static let confirmSeedCreateWalletButton = "seedBackup.createWalletButton"
    static let walletInfoButton = "walletDetail.infoButton"
    static let deleteWalletButton = "walletInfo.deleteWalletButton"
}
