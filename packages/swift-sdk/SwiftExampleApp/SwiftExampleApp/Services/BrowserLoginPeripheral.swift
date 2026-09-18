import CoreBluetooth
import Foundation

/// The Bluetooth LE peripheral a browser connects to for a login key.
///
/// Owns the `CBPeripheralManager`, publishes the
/// `BrowserLoginKeyProtocol` service, reassembles the browser's
/// request write, and serves the status and response characteristics.
/// Everything the UI needs is pushed through `@Published` state on the
/// main actor; the flow decisions (confirm, register, deliver) stay in
/// the view model so this class knows nothing about identities.
@MainActor
final class BrowserLoginPeripheral: NSObject, ObservableObject {
    enum RadioState: Equatable {
        case unknown
        case unauthorized
        case poweredOff
        case unsupported
        case poweredOn
        case advertising
    }

    @Published private(set) var radioState: RadioState = .unknown
    @Published private(set) var status: BrowserLoginKeyProtocol.Status = .idle
    @Published private(set) var lastError: String?

    /// Called on the main actor with each complete request write.
    var onRequest: ((Data) -> Void)?

    private var manager: CBPeripheralManager?
    private var service: CBMutableService?
    private var statusCharacteristic: CBMutableCharacteristic?
    private var responseCharacteristic: CBMutableCharacteristic?
    private var responseBytes = Data()
    /// Whether `start()` was called and the service should go up as
    /// soon as the radio reports powered on.
    private var wantsAdvertising = false

    private let localName: String

    init(localName: String) {
        self.localName = localName
        super.init()
    }

    /// Bring the radio up and advertise the service once it is ready.
    func start() {
        wantsAdvertising = true
        lastError = nil
        if manager == nil {
            // A nil queue delivers delegate callbacks on the main queue,
            // which is what the @MainActor isolation of this class expects.
            manager = CBPeripheralManager(delegate: self, queue: nil)
        } else {
            publishIfReady()
        }
    }

    /// Stop advertising and tear the service down. The response bytes
    /// are wiped so a later connection cannot read a stale key.
    func stop() {
        wantsAdvertising = false
        responseBytes.resetBytes(in: 0..<responseBytes.count)
        responseBytes = Data()
        manager?.stopAdvertising()
        manager?.removeAllServices()
        service = nil
        statusCharacteristic = nil
        responseCharacteristic = nil
        if radioState == .advertising {
            radioState = .poweredOn
        }
    }

    /// Update the status byte and notify a subscribed browser.
    func setStatus(_ status: BrowserLoginKeyProtocol.Status) {
        self.status = status
        guard let manager, let statusCharacteristic else { return }
        _ = manager.updateValue(
            Data([status.rawValue]),
            for: statusCharacteristic,
            onSubscribedCentrals: nil
        )
    }

    /// Expose the encrypted response and flip the status to `ready`.
    func deliver(response: Data) {
        responseBytes = response
        setStatus(.ready)
    }

    private func publishIfReady() {
        guard wantsAdvertising, let manager, manager.state == .poweredOn, service == nil else { return }

        let request = CBMutableCharacteristic(
            type: BrowserLoginKeyProtocol.requestCharacteristicUUID,
            properties: [.write],
            value: nil,
            permissions: [.writeable]
        )
        let statusChar = CBMutableCharacteristic(
            type: BrowserLoginKeyProtocol.statusCharacteristicUUID,
            properties: [.read, .notify],
            value: nil,
            permissions: [.readable]
        )
        let response = CBMutableCharacteristic(
            type: BrowserLoginKeyProtocol.responseCharacteristicUUID,
            properties: [.read],
            value: nil,
            permissions: [.readable]
        )
        let service = CBMutableService(type: BrowserLoginKeyProtocol.serviceUUID, primary: true)
        service.characteristics = [request, statusChar, response]

        self.service = service
        self.statusCharacteristic = statusChar
        self.responseCharacteristic = response
        manager.add(service)
    }

    private func handle(writes requests: [CBATTRequest]) {
        guard let manager, let first = requests.first else { return }
        // A write longer than the ATT MTU arrives as several prepared
        // writes with increasing offsets, delivered together. Stitch
        // them back into one buffer before parsing.
        var assembled = Data()
        for request in requests.sorted(by: { $0.offset < $1.offset }) {
            guard request.characteristic.uuid == BrowserLoginKeyProtocol.requestCharacteristicUUID else {
                manager.respond(to: first, withResult: .writeNotPermitted)
                return
            }
            guard request.offset == assembled.count, let chunk = request.value else {
                manager.respond(to: first, withResult: .invalidOffset)
                return
            }
            assembled.append(chunk)
        }
        manager.respond(to: first, withResult: .success)
        onRequest?(assembled)
    }

    private func handle(read request: CBATTRequest) {
        guard let manager else { return }
        let bytes: Data
        switch request.characteristic.uuid {
        case BrowserLoginKeyProtocol.statusCharacteristicUUID:
            bytes = Data([status.rawValue])
        case BrowserLoginKeyProtocol.responseCharacteristicUUID:
            bytes = responseBytes
        default:
            manager.respond(to: request, withResult: .readNotPermitted)
            return
        }
        guard request.offset <= bytes.count else {
            manager.respond(to: request, withResult: .invalidOffset)
            return
        }
        request.value = bytes.subdata(in: request.offset..<bytes.count)
        manager.respond(to: request, withResult: .success)
    }
}

// The manager was created with a nil queue, so CoreBluetooth calls these
// on the main queue. The `@preconcurrency` conformance keeps the methods
// main-actor isolated (checked at runtime) so the non-Sendable manager
// and ATT requests can be used in place.
extension BrowserLoginPeripheral: @preconcurrency CBPeripheralManagerDelegate {
    func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        switch peripheral.state {
        case .poweredOn:
            radioState = .poweredOn
            publishIfReady()
        case .poweredOff:
            radioState = .poweredOff
        case .unauthorized:
            radioState = .unauthorized
        case .unsupported:
            radioState = .unsupported
        default:
            radioState = .unknown
        }
    }

    func peripheralManager(
        _ peripheral: CBPeripheralManager,
        didAdd service: CBService,
        error: Error?
    ) {
        if let error {
            lastError = "Could not publish the Bluetooth service: \(error.localizedDescription)"
            return
        }
        peripheral.startAdvertising([
            CBAdvertisementDataServiceUUIDsKey: [BrowserLoginKeyProtocol.serviceUUID],
            CBAdvertisementDataLocalNameKey: localName,
        ])
    }

    func peripheralManagerDidStartAdvertising(_ peripheral: CBPeripheralManager, error: Error?) {
        if let error {
            lastError = "Could not start advertising: \(error.localizedDescription)"
        } else {
            radioState = .advertising
        }
    }

    func peripheralManager(_ peripheral: CBPeripheralManager, didReceiveRead request: CBATTRequest) {
        handle(read: request)
    }

    func peripheralManager(_ peripheral: CBPeripheralManager, didReceiveWrite requests: [CBATTRequest]) {
        handle(writes: requests)
    }
}
