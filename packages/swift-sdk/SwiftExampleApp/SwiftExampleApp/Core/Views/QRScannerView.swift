// QRScannerView.swift
// SwiftExampleApp
//
// A self-contained camera QR scanner sheet for the Send screen, plus a
// pure (UIKit-free) payload parser that decodes what a scanned/pasted
// string actually means.
//
// What it does:
//   * Presents a live camera viewfinder, reads `.qr` metadata, and calls
//     `onScan(ScannedPayment)` with a validated Dash address (and an
//     optional BIP21-style `amount=`) the moment a good code is seen.
//   * Validation is NOT re-implemented here — `QRPayloadParser` defers to
//     `DashAddress.parse(_:network:)`, which routes Core addresses through
//     the Rust FFI (`Address.validate`) and decodes Platform/Orchard
//     bech32m locally. The scanner only marshals strings in and out.
//
// States the sheet can be in (every state keeps a "Paste from Clipboard"
// affordance pinned near the bottom, so the screen is useful even with no
// camera):
//   * .scanning      — authorized + a capture device exists: live preview
//                      with a cut-out viewfinder, optional torch toggle,
//                      and transient valid/invalid feedback.
//   * .denied        — permission denied/restricted: an explanatory state
//                      with an "Open Settings" button.
//   * .noDevice      — authorized but no camera (the simulator case):
//                      an intentional "Camera not available" state that
//                      nudges the user to paste instead.
//   * .checking      — the brief window while we resolve authorization.
//
// All UI state is `@MainActor`. The capture session is configured and
// torn down on a background queue; the metadata delegate hops to the main
// actor before touching any state.

import SwiftUI
@preconcurrency import AVFoundation
import UIKit
import SwiftDashSDK

// MARK: - Pure payload parsing (no UIKit — keep this test-light)

/// The meaningful result of decoding a scanned/pasted string: a validated
/// address plus an optional amount lifted from a BIP21-style URI.
struct ScannedPayment: Equatable {
    let address: String
    /// Decimal DASH string from a BIP21-style `amount=` query param, if
    /// present and positive. `nil` when absent or unparseable.
    let amount: String?
}

/// Decodes the strings that show up in Dash payment QR codes into a
/// `ScannedPayment`, or `nil` if the embedded address isn't a recognized
/// Dash address on `network`.
///
/// Accepts:
///   * a bare address (`yMq…`, `tdash1…`, …)
///   * `dash:ADDRESS`
///   * `dash://ADDRESS`
///   * `dash:ADDRESS?amount=1.23&label=Coffee` (params after the address)
///
/// The `dash:` scheme match is case-insensitive, but the address itself is
/// kept verbatim — base58check and bech32m payloads are case-sensitive, so
/// lowercasing them would corrupt valid addresses.
enum QRPayloadParser {
    static func parse(_ raw: String, network: Network) -> ScannedPayment? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }

        // Strip an optional, case-insensitive `dash:` scheme (then an
        // optional `//`) without touching the casing of the remainder.
        var remainder = trimmed
        let schemePrefix = "dash:"
        if remainder.count >= schemePrefix.count,
           remainder.prefix(schemePrefix.count).lowercased() == schemePrefix {
            remainder = String(remainder.dropFirst(schemePrefix.count))
            if remainder.hasPrefix("//") {
                remainder = String(remainder.dropFirst(2))
            }
        }

        // The address is everything up to the query string; params follow.
        let parts = remainder.split(separator: "?", maxSplits: 1,
                                    omittingEmptySubsequences: false)
        let candidate = String(parts.first ?? "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard !candidate.isEmpty else { return nil }

        // Only accept a candidate that resolves to a known address type.
        guard DashAddress.parse(candidate, network: network).type != .unknown else {
            return nil
        }

        let amount = parts.count > 1
            ? positiveAmount(fromQuery: String(parts[1]))
            : nil

        return ScannedPayment(address: candidate, amount: amount)
    }

    /// Extract a positive `amount` value from a `key=value&key=value` query
    /// string. Returns the original (un-reformatted) string so the field is
    /// filled with exactly what the QR author intended, but only if it
    /// parses as a strictly-positive `Double`.
    private static func positiveAmount(fromQuery query: String) -> String? {
        for pair in query.split(separator: "&") {
            let kv = pair.split(separator: "=", maxSplits: 1,
                                omittingEmptySubsequences: false)
            guard kv.count == 2, kv[0].lowercased() == "amount" else { continue }
            let rawValue = String(kv[1])
            // Percent-decode in case the URI encoded the value; fall back
            // to the raw text when it wasn't encoded.
            let value = rawValue.removingPercentEncoding ?? rawValue
            if let parsed = Double(value), parsed > 0 {
                return value
            }
            return nil
        }
        return nil
    }
}

// MARK: - Scanner sheet

/// Camera QR scanner presented as a sheet. Calls `onScan` exactly once on a
/// successful read (or paste) and then dismisses itself.
struct QRScannerView: View {
    /// The states the sheet resolves into after checking permissions and
    /// hardware availability.
    private enum ScanState: Equatable {
        case checking
        case scanning
        case denied
        case noDevice
    }

    let network: Network
    let onScan: @MainActor (ScannedPayment) -> Void

    @Environment(\.dismiss) private var dismiss

    /// Owns the `AVCaptureSession`. The view drives it (stop on match,
    /// torch toggle) without the representable having to expose AVFoundation
    /// state back up the tree.
    @StateObject private var camera = CameraSessionController()

    @State private var state: ScanState = .checking
    @State private var isTorchOn = false
    /// Transient banner shown over the viewfinder when a code (or paste)
    /// doesn't validate.
    @State private var invalidMessage: String?
    /// Drives the green "matched!" flash before we dismiss.
    @State private var didMatch = false
    /// The most recent payload we rejected, with the time we rejected it —
    /// used to de-dupe a hovering invalid code so it doesn't machine-gun
    /// haptics.
    @State private var lastRejected: (payload: String, at: Date)?

    var body: some View {
        NavigationStack {
            ZStack {
                Color.black.ignoresSafeArea()

                switch state {
                case .checking:
                    ProgressView()
                        .progressViewStyle(.circular)
                        .tint(.white)
                case .scanning:
                    scanningContent
                case .denied:
                    deniedContent
                case .noDevice:
                    noDeviceContent
                }

                // Paste affordance is available in EVERY state.
                VStack {
                    Spacer()
                    pasteButton
                        .padding(.horizontal, 24)
                        .padding(.bottom, 28)
                }
            }
            .navigationTitle("Scan QR Code")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                }
            }
            .toolbarBackground(.visible, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
        }
        .onAppear(perform: resolveInitialState)
    }

    // MARK: Scanning state

    private var scanningContent: some View {
        GeometryReader { proxy in
            let cutout = viewfinderRect(in: proxy.size)
            ZStack {
                CameraPreview(
                    controller: camera,
                    rectOfInterest: cutout,
                    onResult: handleScan
                )
                .ignoresSafeArea()

                ViewfinderOverlay(cutout: cutout, matched: didMatch)
                    .ignoresSafeArea()

                // Hint + transient invalid banner, anchored to the cutout.
                VStack(spacing: 0) {
                    Spacer()
                        .frame(height: cutout.maxY + 16)
                    if let invalidMessage {
                        Text(invalidMessage)
                            .font(.footnote.weight(.semibold))
                            .foregroundColor(.white)
                            .padding(.horizontal, 14)
                            .padding(.vertical, 8)
                            .background(
                                Capsule().fill(Color.red.opacity(0.9))
                            )
                            .transition(.opacity.combined(with: .scale))
                    } else {
                        Text("Point the camera at a Dash address QR code")
                            .font(.footnote)
                            .foregroundColor(.white)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal, 32)
                    }
                    Spacer()
                }
                .frame(maxWidth: .infinity)

                // Torch toggle, bottom-trailing, only when supported.
                if CameraSessionController.hasTorch {
                    VStack {
                        Spacer()
                        HStack {
                            Spacer()
                            torchButton
                                .padding(.trailing, 24)
                                // Sit above the paste button.
                                .padding(.bottom, 96)
                        }
                    }
                }
            }
        }
    }

    private var torchButton: some View {
        Button {
            isTorchOn.toggle()
            camera.setTorch(on: isTorchOn)
        } label: {
            Image(systemName: isTorchOn ? "flashlight.on.fill" : "flashlight.off.fill")
                .font(.title3)
                .foregroundColor(.white)
                .frame(width: 48, height: 48)
                .background(.ultraThinMaterial, in: Circle())
        }
        .accessibilityLabel(isTorchOn ? "Turn off torch" : "Turn on torch")
    }

    // MARK: Denied state

    private var deniedContent: some View {
        VStack(spacing: 16) {
            Image(systemName: "camera.fill")
                .font(.system(size: 56))
                .foregroundColor(.white.opacity(0.85))
            Text("Camera access is off")
                .font(.title3.weight(.semibold))
                .foregroundColor(.white)
            Text("Allow camera access in Settings to scan a Dash address QR code. You can still paste an address below.")
                .font(.callout)
                .foregroundColor(.white.opacity(0.7))
                .multilineTextAlignment(.center)
                .padding(.horizontal, 40)
            Button {
                openSettings()
            } label: {
                Text("Open Settings")
            }
            .buttonStyle(.bordered)
            .tint(.white)
        }
        .padding(.bottom, 80)
    }

    // MARK: No-device state (simulator)

    private var noDeviceContent: some View {
        VStack(spacing: 16) {
            Image(systemName: "camera.on.rectangle")
                .font(.system(size: 56))
                .foregroundColor(.white.opacity(0.85))
            Text("Camera not available")
                .font(.title3.weight(.semibold))
                .foregroundColor(.white)
            Text("Paste an address instead.")
                .font(.callout)
                .foregroundColor(.white.opacity(0.7))
                .multilineTextAlignment(.center)
                .padding(.horizontal, 40)
        }
        .padding(.bottom, 80)
    }

    // MARK: Paste affordance

    private var pasteButton: some View {
        Button {
            handlePaste()
        } label: {
            Label("Paste from Clipboard", systemImage: "doc.on.clipboard")
                .frame(maxWidth: .infinity)
        }
        .buttonStyle(.borderedProminent)
        .controlSize(.large)
    }

    // MARK: - Geometry

    /// A centered rounded square ~65% of the width, used both as the visual
    /// viewfinder and as the camera's region of interest.
    private func viewfinderRect(in size: CGSize) -> CGRect {
        let side = min(size.width, size.height) * 0.65
        let originX = (size.width - side) / 2
        let originY = (size.height - side) / 2
        return CGRect(x: originX, y: originY, width: side, height: side)
    }

    // MARK: - Permission / availability resolution

    private func resolveInitialState() {
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized:
            state = CameraSessionController.hasCaptureDevice ? .scanning : .noDevice
        case .notDetermined:
            AVCaptureDevice.requestAccess(for: .video) { granted in
                Task { @MainActor in
                    if granted {
                        state = CameraSessionController.hasCaptureDevice ? .scanning : .noDevice
                    } else {
                        state = .denied
                    }
                }
            }
        case .denied, .restricted:
            state = .denied
        @unknown default:
            state = .denied
        }
    }

    // MARK: - Result handling

    /// Called by the camera delegate (already on the main actor) with the
    /// raw string of a decoded QR code.
    @MainActor
    private func handleScan(_ raw: String) {
        // Ignore further reads once we've matched and are animating out.
        guard !didMatch else { return }

        if let payment = QRPayloadParser.parse(raw, network: network) {
            acceptMatch(payment)
        } else {
            rejectPayload(raw, message: "Not a Dash address")
        }
    }

    @MainActor
    private func handlePaste() {
        let clipboard = UIPasteboard.general.string ?? ""
        if let payment = QRPayloadParser.parse(clipboard, network: network) {
            acceptMatch(payment)
        } else {
            showInvalid("Clipboard doesn't contain a valid address")
        }
    }

    /// Common success path for both scan and paste.
    @MainActor
    private func acceptMatch(_ payment: ScannedPayment) {
        guard !didMatch else { return }
        didMatch = true
        invalidMessage = nil
        camera.stop()
        UINotificationFeedbackGenerator().notificationOccurred(.success)
        withAnimation(.spring(response: 0.3, dampingFraction: 0.6)) {
            // `didMatch` flips the viewfinder border green via the overlay.
        }
        // Give the green flash a beat to register, then hand off & dismiss.
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.35) {
            onScan(payment)
            dismiss()
        }
    }

    /// Reject a scanned payload, de-duped so a hovering invalid code emits
    /// at most one error haptic per distinct string within ~1.5s.
    @MainActor
    private func rejectPayload(_ raw: String, message: String) {
        let now = Date()
        if let last = lastRejected,
           last.payload == raw,
           now.timeIntervalSince(last.at) < 1.5 {
            // Same code still hovering — refresh the timestamp, stay quiet.
            lastRejected = (raw, now)
            return
        }
        lastRejected = (raw, now)
        UINotificationFeedbackGenerator().notificationOccurred(.error)
        showInvalid(message)
    }

    /// Flash a transient invalid banner that auto-hides after ~1.5s.
    @MainActor
    private func showInvalid(_ message: String) {
        withAnimation(.easeInOut(duration: 0.2)) {
            invalidMessage = message
        }
        let shown = message
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
            // Only clear if this same banner is still up.
            if invalidMessage == shown {
                withAnimation(.easeInOut(duration: 0.2)) {
                    invalidMessage = nil
                }
            }
        }
    }

    private func openSettings() {
        guard let url = URL(string: UIApplication.openSettingsURLString) else { return }
        UIApplication.shared.open(url)
    }
}

// MARK: - Viewfinder overlay

/// Dims the screen and punches a rounded-square hole over the live preview,
/// stroking the cutout (green once a match lands).
private struct ViewfinderOverlay: View {
    let cutout: CGRect
    let matched: Bool

    private let cornerRadius: CGFloat = 20

    var body: some View {
        ZStack {
            // Dim everything, then cut out the viewfinder with an even-odd
            // fill so the camera shows through the hole.
            Path { path in
                path.addRect(CGRect(origin: .zero, size: UIScreen.main.bounds.size))
                path.addRoundedRect(in: cutout, cornerSize: CGSize(width: cornerRadius,
                                                                   height: cornerRadius))
            }
            .fill(Color.black.opacity(0.55), style: FillStyle(eoFill: true))

            RoundedRectangle(cornerRadius: cornerRadius)
                .stroke(matched ? Color.green : Color.white,
                        style: StrokeStyle(lineWidth: 2, lineCap: .round, lineJoin: .round))
                .frame(width: cutout.width, height: cutout.height)
                .position(x: cutout.midX, y: cutout.midY)
                .animation(.spring(response: 0.3, dampingFraction: 0.6), value: matched)
        }
        .allowsHitTesting(false)
    }
}

// MARK: - Camera preview (AVFoundation bridge)

/// `UIViewRepresentable` that hosts an `AVCaptureVideoPreviewLayer` over the
/// `.qr`-only `AVCaptureSession` owned by `CameraSessionController`. Metadata
/// results are delivered on the main actor via `onResult`.
///
/// The representable is intentionally thin: it forwards the preview layer and
/// region of interest to the controller, which the parent view also holds so
/// it can stop the session (`controller.stop()`) and toggle the torch
/// (`controller.setTorch`) without reaching back through the representable.
private struct CameraPreview: UIViewRepresentable {
    let controller: CameraSessionController
    let rectOfInterest: CGRect
    let onResult: @MainActor @Sendable (String) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(controller: controller, onResult: onResult)
    }

    func makeUIView(context: Context) -> PreviewContainerView {
        let view = PreviewContainerView()
        view.previewLayer.videoGravity = .resizeAspectFill
        controller.start(previewLayer: view.previewLayer,
                         rectOfInterest: rectOfInterest,
                         delegate: context.coordinator)
        return view
    }

    func updateUIView(_ uiView: PreviewContainerView, context: Context) {
        controller.updateRectOfInterest(rectOfInterest,
                                        previewLayer: uiView.previewLayer)
    }

    static func dismantleUIView(_ uiView: PreviewContainerView, coordinator: Coordinator) {
        coordinator.controller.stop()
    }

    // MARK: Container view whose backing layer is the preview layer

    /// A `UIView` whose layer class is `AVCaptureVideoPreviewLayer`, so the
    /// preview always tracks the view's bounds without manual frame syncing.
    final class PreviewContainerView: UIView {
        override class var layerClass: AnyClass { AVCaptureVideoPreviewLayer.self }

        var previewLayer: AVCaptureVideoPreviewLayer {
            guard let layer = layer as? AVCaptureVideoPreviewLayer else {
                // Unreachable: `layerClass` guarantees this type. Returning a
                // detached layer keeps the API non-optional without a crash.
                return AVCaptureVideoPreviewLayer()
            }
            return layer
        }
    }

    // MARK: Coordinator — the metadata delegate

    /// Bridges `AVCaptureMetadataOutput` callbacks (which fire on the
    /// session's background queue) to the main-actor `onResult` closure.
    final class Coordinator: NSObject, AVCaptureMetadataOutputObjectsDelegate {
        let controller: CameraSessionController
        private let onResult: @MainActor @Sendable (String) -> Void

        init(controller: CameraSessionController,
             onResult: @escaping @MainActor @Sendable (String) -> Void) {
            self.controller = controller
            self.onResult = onResult
        }

        func metadataOutput(_ output: AVCaptureMetadataOutput,
                            didOutput metadataObjects: [AVMetadataObject],
                            from connection: AVCaptureConnection) {
            guard
                let object = metadataObjects.first as? AVMetadataMachineReadableCodeObject,
                object.type == .qr,
                let value = object.stringValue
            else { return }

            // Delegate fires on the session queue — hop to the main actor
            // before touching any SwiftUI state.
            let handler = onResult
            Task { @MainActor in
                handler(value)
            }
        }
    }
}

// MARK: - Camera session controller

/// Owns the scanner's `AVCaptureSession` and serializes every access to it
/// onto a private background queue.
///
/// Marked `@unchecked Sendable` deliberately: the only non-`Sendable` state
/// (`session`, its inputs/outputs) is touched exclusively on `queue`, and the
/// preview-layer wiring (`previewLayer.session = session`) happens on the
/// main thread before any concurrent work begins. The `ObservableObject`
/// surface is intentionally empty today — the controller exists so the view
/// can drive start/stop/torch without leaking AVFoundation upward — but
/// conforming keeps it usable as a `@StateObject`.
private final class CameraSessionController: ObservableObject, @unchecked Sendable {
    private let session = AVCaptureSession()
    private let queue = DispatchQueue(label: "qrscanner.session")
    private var isConfigured = false

    // MARK: Static hardware queries

    /// A capture device exists — used to choose `.scanning` vs `.noDevice`
    /// before any session is built (the simulator has none).
    static var hasCaptureDevice: Bool {
        AVCaptureDevice.default(for: .video) != nil
    }

    static var hasTorch: Bool {
        AVCaptureDevice.default(for: .video)?.hasTorch ?? false
    }

    // MARK: Lifecycle

    /// Configure (once) and start the session, wiring `delegate` to a fresh
    /// metadata output limited to `.qr`. Safe to call repeatedly — it only
    /// builds inputs/outputs the first time and restarts thereafter.
    func start(previewLayer: AVCaptureVideoPreviewLayer,
               rectOfInterest: CGRect,
               delegate: AVCaptureMetadataOutputObjectsDelegate) {
        // Layer/session wiring stays on the main thread; the heavy session
        // work is dispatched to `queue`.
        previewLayer.session = session
        let converted = previewLayer.metadataOutputRectConverted(fromLayerRect: rectOfInterest)
        queue.async { [self] in
            if !isConfigured {
                configure(delegate: delegate)
            }
            applyRectOfInterest(converted)
            if !session.isRunning {
                session.startRunning()
            }
        }
    }

    private func configure(delegate: AVCaptureMetadataOutputObjectsDelegate) {
        session.beginConfiguration()
        defer { session.commitConfiguration() }

        guard
            let device = AVCaptureDevice.default(for: .video),
            let input = try? AVCaptureDeviceInput(device: device),
            session.canAddInput(input)
        else {
            return
        }
        session.addInput(input)

        let output = AVCaptureMetadataOutput()
        guard session.canAddOutput(output) else { return }
        session.addOutput(output)
        output.setMetadataObjectsDelegate(delegate, queue: queue)
        if output.availableMetadataObjectTypes.contains(.qr) {
            output.metadataObjectTypes = [.qr]
        }

        isConfigured = true
    }

    /// Re-derive and apply the region of interest after a layout change. The
    /// layer→metadata conversion needs the preview layer's geometry, so it
    /// happens on the main thread; the assignment is dispatched to `queue`.
    func updateRectOfInterest(_ rect: CGRect, previewLayer: AVCaptureVideoPreviewLayer) {
        let converted = previewLayer.metadataOutputRectConverted(fromLayerRect: rect)
        queue.async { [self] in
            applyRectOfInterest(converted)
        }
    }

    /// Must be called on `queue`. Pushes the normalized rect to the metadata
    /// output, ignoring degenerate rects (zero-size during first layout).
    private func applyRectOfInterest(_ converted: CGRect) {
        guard converted.width > 0, converted.height > 0 else { return }
        for output in session.outputs {
            if let metadataOutput = output as? AVCaptureMetadataOutput {
                metadataOutput.rectOfInterest = converted
            }
        }
    }

    func stop() {
        queue.async { [self] in
            if session.isRunning {
                session.stopRunning()
            }
        }
    }

    func setTorch(on: Bool) {
        queue.async {
            guard let device = AVCaptureDevice.default(for: .video), device.hasTorch else { return }
            do {
                try device.lockForConfiguration()
                device.torchMode = on ? .on : .off
                device.unlockForConfiguration()
            } catch {
                // Non-fatal: torch just stays in its prior state.
            }
        }
    }
}
