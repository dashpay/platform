# Historical source reconstruction

`fixture.store` is a synthetic SQLite store reconstructed from Platform
`fd8d8d13e5d7cea17b00df5974934ab1910e8039`. The associated iOS source is
`8094751eb2be8d52b57da3589fdd2ae2dcd0ecc6`, referenced by
[Actions run 32706880873](https://github.com/dashpay/dashwallet-ios/actions/runs/32706880873).
This source pair is not verified App Store provenance. The fixture was created
on a simulator; it contains no user data and is not extracted from an app binary.

The store contains one synthetic wallet, data contract, document type and index,
including the type-to-contract and index-to-type relationships. Its 34 entities
come from the historical factory's literal `modelTypes` list. It was created
using an unversioned `Schema` and no migration plan, as in that historical app.
Its `PersistentDocumentType` and `PersistentIndex` hashes differ from the
accepted frozen V1. The accepted V1 definitions and `dash-v1.store` remain
unchanged and cover a separate migration regression.

`manifest.json` records the exact source pair, toolchain, original fixture
checksum, schema metadata, SQLite indexes, synthetic row counts, source-file
digests, deterministic generated-file digests and capture recipe digest. The
recorded capture used Xcode 26.6 (17F113), an arm64 iPhone 17 simulator running
iOS 26.5, and Release configuration. The fixture SHA-256 is
`1cb3d6c2c299eb9afa9a2152e023acff6c108f1f4caebafc9db8da4a90fbd0da`.

## Verify and reproduce

From a full-history Platform checkout, verify the committed SQLite evidence and
regenerate all historical Swift source in memory:

```sh
python3 packages/swift-sdk/scripts/historical_schema_fixture.py --check
```

To recapture on macOS, first build the current SDK's simulator XCFramework as
described in the SDK build guide. Then export the historical models and their
dedicated capture test into a new temporary SDK directory:

```sh
python3 packages/swift-sdk/scripts/historical_schema_fixture.py \
  --prepare-sdk /tmp/dash-historical-capture-sdk
cd /tmp/dash-historical-capture-sdk
xcodebuild test -scheme SwiftDashSDK -configuration Release \
  -destination 'platform=iOS Simulator,name=iPhone 17,OS=26.5,arch=arm64' \
  -parallel-testing-enabled NO \
  -derivedDataPath /tmp/dash-historical-capture-derived \
  -resultBundlePath /tmp/dash-historical-capture.xcresult \
  -only-testing:SwiftDashSDKTests/DashHistoricalFixtureCaptureTests \
  CODE_SIGNING_ALLOWED=NO ARCHS=arm64 ENABLE_TESTABILITY=YES
xcrun xcresulttool export attachments \
  --path /tmp/dash-historical-capture.xcresult \
  --output-path /tmp/dash-historical-capture-attachments
```

Choose unused paths for another run. For comparison with this fixture, use the
recorded toolchain/runtime. The test compares the regenerated store's entity
hashes, model checksum, version identifier and indexes with the manifest before
attaching its standalone store and JSON metadata. SQLite UUIDs and timestamps
vary, so recaptured bytes are not expected to match the original file checksum.
Do not overwrite the committed fixture to make a migration or metadata check
pass; investigate differences first.

The export uses the current SDK as a build harness, while every historical model
and stored value type comes from the pinned Platform source. Only the temporary
SDK receives these 36 generated Swift files and the capture test; the shipping
SDK does not gain a second historical model graph. The normal migration tests
consume the committed SQLite fixture directly.
