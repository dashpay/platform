//
//  AvailabilityRequestGateTests.swift
//  SwiftExampleAppTests
//
//  Username availability lookups run concurrently; only the newest one, for
//  the name still being edited, may update the registration form.
//

import XCTest
@testable import SwiftExampleApp

final class AvailabilityRequestGateTests: XCTestCase {
    func testOlderLookupCompletingLastIsIgnored() {
        var gate = AvailabilityRequestGate()
        let first = gate.begin(name: "alice")
        let second = gate.begin(name: "bob")

        // "bob" answers first, then the slower "alice" lookup lands.
        XCTAssertTrue(gate.accepts(id: second, name: "bob"))
        XCTAssertFalse(gate.accepts(id: first, name: "alice"))
        XCTAssertFalse(gate.accepts(id: first, name: "bob"))
    }

    func testLookupForAnEditedNameIsIgnored() {
        var gate = AvailabilityRequestGate()
        let lookup = gate.begin(name: "alice")

        // The user kept typing before the answer arrived.
        XCTAssertFalse(gate.accepts(id: lookup, name: "alice2"))
    }

    func testCancelDropsTheLookupInFlight() {
        var gate = AvailabilityRequestGate()
        let lookup = gate.begin(name: "alice")
        gate.cancel()

        XCTAssertFalse(gate.accepts(id: lookup, name: "alice"))
    }

    func testCurrentLookupIsAccepted() {
        var gate = AvailabilityRequestGate()
        let lookup = gate.begin(name: "alice")

        XCTAssertTrue(gate.accepts(id: lookup, name: "alice"))
    }
}
