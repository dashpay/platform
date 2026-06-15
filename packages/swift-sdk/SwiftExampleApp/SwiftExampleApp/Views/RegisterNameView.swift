import SwiftUI
import SwiftDashSDK
import SwiftData

struct RegisterNameView: View {
  let identity: PersistentIdentity
  /// Optional success callback invoked on the main actor right after
  /// the FFI call returns the registered full domain name. The parent
  /// uses this to refresh its own DPNS-names state — `IdentityDetailView`
  /// holds the list in `@State` (not `@Query`-bound), so the sheet
  /// dismissing alone is not enough to trigger a re-fetch.
  var onRegistered: ((String) -> Void)? = nil
  @EnvironmentObject var appState: AppState
  @EnvironmentObject var walletManager: PlatformWalletManager
  @Environment(\.dismiss) var dismiss
  // Required by `KeychainSigner` so its trampoline can fetch
  // `PersistentPublicKey` rows on a private background context. The
  // view doesn't query SwiftData itself — `modelContext.container`
  // is the only thing it threads through to the signer.
  @Environment(\.modelContext) private var modelContext

  @State private var username = ""
  @State private var isChecking = false
  @State private var isAvailable: Bool? = nil
  @State private var isContested = false
  @State private var errorMessage = ""
  @State private var showingError = false
  @State private var checkTimer: Timer? = nil
  @State private var lastCheckedName = ""
  @State private var isRegistering = false
  @State private var registrationSuccess = false

  private var normalizedUsername: String {
    // Use the FFI function to normalize the username
    let trimmed = username.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty else { return "" }

    return trimmed.withCString { namePtr in
      let result = dash_sdk_dpns_normalize_username(namePtr)
      defer {
        if let error = result.error {
          dash_sdk_error_free(error)
        }
        if let dataPtr = result.data {
          dash_sdk_string_free(dataPtr.assumingMemoryBound(to: CChar.self))
        }
      }

      if result.error == nil, let dataPtr = result.data {
        return String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
      }
      return trimmed.lowercased() // Fallback to simple lowercasing
    }
  }

  private enum ValidationStatus {
    case valid
    case notLongEnough
    case tooLong
    case invalidCharacters
    case invalidHyphenPlacement
  }

  private var validationStatus: ValidationStatus {
    let name = normalizedUsername

    // Check basic length first
    if name.count < 3 {
      return .notLongEnough
    }
    if name.count > 63 {
      return .tooLong
    }

    // Use FFI function to validate
    let isValid = name.withCString { namePtr in
      let result = dash_sdk_dpns_is_valid_username(namePtr)
      return result == 1
    }

    if isValid {
      return .valid
    }

    // If not valid, determine the specific reason
    // Check for invalid characters
    let validCharsPattern = "^[a-z0-9-]+$"
    let validCharsRegex = try? NSRegularExpression(pattern: validCharsPattern, options: [])
    let range = NSRange(location: 0, length: name.utf16.count)
    if validCharsRegex?.firstMatch(in: name, options: [], range: range) == nil {
      return .invalidCharacters
    }

    // Check hyphen rules
    if name.hasPrefix("-") || name.hasSuffix("-") || name.contains("--") {
      return .invalidHyphenPlacement
    }

    return .invalidCharacters // Default for any other invalid case
  }

  private var isValidUsername: Bool {
    // Use the FFI function directly
    guard !normalizedUsername.isEmpty else { return false }

    return normalizedUsername.withCString { namePtr in
      let result = dash_sdk_dpns_is_valid_username(namePtr)
      return result == 1
    }
  }

  private var validationMessage: String {
    // Use the FFI function to get validation message
    guard !normalizedUsername.isEmpty else { return "" }

    return normalizedUsername.withCString { namePtr in
      let result = dash_sdk_dpns_get_validation_message(namePtr)
      defer {
        if let error = result.error {
          dash_sdk_error_free(error)
        }
        if let dataPtr = result.data {
          dash_sdk_string_free(dataPtr.assumingMemoryBound(to: CChar.self))
        }
      }

      if result.error == nil, let dataPtr = result.data {
        let message = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        return message == "valid" ? "" : message
      }

      // Fallback to our own messages
      switch validationStatus {
      case .valid:
        return ""
      case .notLongEnough:
        return "Name must be at least 3 characters long"
      case .tooLong:
        return "Name must be 63 characters or less"
      case .invalidCharacters:
        return "Name can only contain letters, numbers, and hyphens"
      case .invalidHyphenPlacement:
        return "Hyphens cannot be at the start/end or consecutive"
      }
    }
  }

  private var isNameContested: Bool {
    // Only check if name is valid
    guard isValidUsername else { return false }

    // Use the FFI function to check if the name is contested
    return normalizedUsername.withCString { namePtr in
      let result = dash_sdk_dpns_is_contested_username(namePtr)
      return result == 1
    }
  }

  var body: some View {
    NavigationView {
      Form {
        Section("Choose Your Username") {
          TextField("Enter username", text: $username)
            .textContentType(.username)
            .autocapitalization(.none)
            .autocorrectionDisabled(true)
            .modifier(UsernameChangeHandler(username: $username) {
              // Cancel any existing timer
              checkTimer?.invalidate()

              // Reset availability if name changed
              if normalizedUsername != lastCheckedName {
                isAvailable = nil
                isChecking = false
              }

              errorMessage = ""
              // Update contested status
              isContested = isNameContested

              // Start new timer if name is valid
              if isValidUsername && normalizedUsername != lastCheckedName {
                checkTimer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: false) { _ in
                  Task {
                    await checkAvailabilityAutomatically()
                  }
                }
              }
            })

          if !normalizedUsername.isEmpty {
            VStack(alignment: .leading, spacing: 4) {
              HStack {
                Text("Normalized: ")
                  .font(.caption)
                  .foregroundColor(.secondary)
                Text("\(normalizedUsername).dash")
                  .font(.caption)
                  .foregroundColor(.blue)
              }

              // Show validation status
              if validationStatus != .valid {
                HStack {
                  Image(systemName: "exclamationmark.circle.fill")
                    .foregroundColor(.red)
                    .font(.caption)
                  Text(validationMessage)
                    .font(.caption)
                    .foregroundColor(.red)
                }
              }
            }
          }
        }

        Section("Name Information") {
          HStack {
            Text("Validity")
            Spacer()
            if !normalizedUsername.isEmpty {
              switch validationStatus {
              case .valid:
                Label("Valid", systemImage: "checkmark.circle.fill")
                  .foregroundColor(.green)
              case .notLongEnough:
                Label("Not Long Enough", systemImage: "xmark.circle.fill")
                  .foregroundColor(.red)
              case .tooLong:
                Label("Too Long", systemImage: "xmark.circle.fill")
                  .foregroundColor(.red)
              case .invalidCharacters, .invalidHyphenPlacement:
                Label("Not Valid", systemImage: "xmark.circle.fill")
                  .foregroundColor(.red)
              }
            } else {
              Text("Enter a name")
                .foregroundColor(.secondary)
            }
          }

          if isValidUsername {
            HStack {
              Text("Availability")
              Spacer()
              if isChecking {
                ProgressView()
                  .scaleEffect(0.8)
              } else if let available = isAvailable {
                if available {
                  Label("Available", systemImage: "checkmark.circle.fill")
                    .foregroundColor(.green)
                } else {
                  Label("Taken", systemImage: "xmark.circle.fill")
                    .foregroundColor(.red)
                }
              } else {
                Text("Not checked")
                  .foregroundColor(.secondary)
              }
            }
          }

          HStack {
            Text("Contest Status")
            Spacer()
            if isContested {
              Label("Contested", systemImage: "flag.fill")
                .foregroundColor(.orange)
            } else {
              Label("Regular", systemImage: "checkmark.circle")
                .foregroundColor(.green)
            }
          }
        }

        if isContested && !normalizedUsername.isEmpty {
          Section("Contest Warning") {
            HStack {
              Image(systemName: "exclamationmark.triangle.fill")
                .foregroundColor(.orange)
              VStack(alignment: .leading, spacing: 4) {
                Text("Contested Name")
                  .font(.headline)
                  .foregroundColor(.orange)
                Text("This name is less than 20 characters with only letters (a-z, A-Z), digits (0, 1), and hyphens. It requires a masternode vote contest to register.")
                  .font(.caption)
                  .foregroundColor(.secondary)
              }
            }
            .padding(.vertical, 4)
          }
        }

        Section {
          Button(action: registerName) {
            HStack {
              if isRegistering {
                ProgressView()
                  .progressViewStyle(CircularProgressViewStyle(tint: .white))
                  .scaleEffect(0.8)
                Text("Registering...")
              } else {
                Image(systemName: "plus.circle.fill")
                Text("Register Name")
              }
            }
            .foregroundColor(.white)
            .frame(maxWidth: .infinity)
            .padding()
            .background(isValidUsername && isAvailable == true && !isRegistering ? Color.blue : Color.gray)
            .cornerRadius(10)
          }
          .disabled(!isValidUsername || isAvailable != true || isRegistering)
          .listRowInsets(EdgeInsets())
          .listRowBackground(Color.clear)
        }
      }
      .navigationTitle("Register DPNS Name")
      .navigationBarTitleDisplayMode(.inline)
      .toolbar {
        ToolbarItem(placement: .navigationBarLeading) {
          Button("Cancel") {
            dismiss()
          }
        }
      }
      .alert(registrationSuccess ? "Success" : "Error", isPresented: $showingError) {
        Button("OK") { }
      } message: {
        Text(errorMessage)
      }
      .onDisappear {
        // Clean up timer when view disappears
        checkTimer?.invalidate()
        checkTimer = nil
      }
    }
  }

  private func checkAvailabilityAutomatically() async {
    // Store the name we're checking
    lastCheckedName = normalizedUsername

    // Start showing the checking indicator
    await MainActor.run {
      isChecking = true
    }

    // Use the SDK to check availability
    guard let sdk = appState.sdk else {
      await MainActor.run {
        errorMessage = "SDK not initialized"
        showingError = true
        isChecking = false
      }
      return
    }

    do {
      let available = try await sdk.dpnsCheckAvailability(name: normalizedUsername)

      await MainActor.run {
        isAvailable = available
        isChecking = false
        if !available {
          errorMessage = "This name is already registered"
        }
      }
    } catch {
      await MainActor.run {
        // If we get an error, assume unavailable
        isAvailable = false
        isChecking = false
        errorMessage = "Failed to check availability: \(error.localizedDescription)"
        // Don't show error alert for automatic checks
      }
    }
  }

  private func registerName() {
    // Route through the platform-wallet
    // `IdentityWallet::register_name_with_external_signer` path
    // (via `registerDpnsName(identityId:name:signer:)`). Benefits:
    //   - Signing crosses the FFI through the Swift `KeychainSigner`,
    //     same model identity registration uses — so the wallet's
    //     own seed never participates and watch-only restored
    //     wallets can register names too.
    //   - On success the Rust side appends the new name to
    //     `ManagedIdentity.dpns_names` and queues an `IdentityChangeSet`
    //     so `PersistentIdentity.dpnsName` refreshes automatically via
    //     the persister callback — no manual SwiftData upsert needed.
    guard let walletId = identity.wallet?.walletId,
          let wallet = walletManager.wallet(for: walletId) else {
      errorMessage = "No wallet available for this identity"
      showingError = true
      return
    }

    // Construct a fresh `KeychainSigner` for this registration the
    // same way `CreateIdentityView.submit()` does. The signer holds
    // a `+1`-retained C-side handle that's released in `deinit`;
    // keeping it in a local across the awaited Task is enough to
    // pin its lifetime past the FFI calls. The trampoline performs
    // the `PersistentPublicKey` → Keychain lookup on a private
    // background `ModelContext` derived from
    // `modelContext.container`.
    let signer = KeychainSigner(modelContainer: modelContext.container)

    // The DPNS document stores `label` (display form, what the user
    // typed) and `normalizedLabel` (homograph-safe lowercase, used
    // for uniqueness lookup). The SDK side derives `normalizedLabel`
    // from `label` via `convert_to_homograph_safe_chars`, so we hand
    // it the raw trimmed input — passing `normalizedUsername` here
    // makes both columns identical and the original casing /
    // i-vs-1, o-vs-0 letters are lost from the cache and every
    // subsequent display.
    let displayLabel = username.trimmingCharacters(in: .whitespacesAndNewlines)

    isRegistering = true

    Task {
      do {
        let registeredName = try await wallet.registerDpnsName(
          identityId: identity.identityId,
          name: displayLabel,
          signer: signer
        )

        await MainActor.run {
          // Rust persister callback writes the new label to
          // `PersistentIdentity` asynchronously, but the parent
          // (`IdentityDetailView`) caches the list in plain `@State`
          // populated only at `onAppear` — so `@Query` reactivity
          // doesn't help here. Fire `onRegistered` so the parent
          // can immediately add this name to its local state and
          // skip the "wait until I leave + come back" round-trip.
          //
          // Pass the bare display label (what we just registered),
          // NOT the FFI's full-domain return value
          // (`registeredName` = "name.dash"). The parent's
          // `dpnsNames` array stores bare labels — that's what
          // `managed.getDpnsNames()` returns — so passing the full
          // domain here would render "label.dash" instead of "label"
          // until the next `loadDPNSNames` round.
          onRegistered?(displayLabel)

          registrationSuccess = true
          errorMessage = isContested ?
          "Successfully started contest for \(displayLabel). Follow \(appState.currentNetwork == .mainnet ? "14 days" : "90 minutes") to resolution." :
          "Successfully registered \(registeredName)!"
          showingError = true
          isRegistering = false
        }

        // Dismiss the view after a short delay
        try? await Task.sleep(nanoseconds: 2_000_000_000)
        await MainActor.run {
          dismiss()
        }

      } catch {
        await MainActor.run {
          errorMessage = "Registration failed: \(error.localizedDescription)"
          showingError = true
          isRegistering = false
        }
      }
    }
  }

}

private struct UsernameChangeHandler: ViewModifier {
  @Binding var username: String
  let onChange: () -> Void
  func body(content: Content) -> some View {
    if #available(iOS 17.0, *) {
      content.onChange(of: username) { _, _ in onChange() }
    } else {
      content.onChange(of: username) { _ in onChange() }
    }
  }
}

// Preview removed — constructing a sample `PersistentIdentity`
// requires a mock `ModelContainer`; not worth the scaffolding for
// this dev example app. Restore via `#Preview { … }` with a
// `.modelContainer(for: PersistentIdentity.self, inMemory: true)`
// if/when previews become load-bearing.
