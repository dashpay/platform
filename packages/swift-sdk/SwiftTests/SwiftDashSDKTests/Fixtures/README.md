# v4.2.0-dev.1 migration fixture

`DashModel-v4.2.0-dev.1.sqlite.zlib` is a synthetic SwiftData store created
from the exact `v4.2.0-dev.1` model sources. It contains one wallet and one
BIP44 account with non-secret marker bytes; it contains no production wallet
material.

- uncompressed SQLite size: `647168` bytes
- uncompressed SHA-256: `17c2e93e655b79c43d023f41a4a4360e511d8f97af56aedfce32bd73c0158e58`
- compression: Foundation `NSData.CompressionAlgorithm.zlib`

The regression test opens a copy with the same inferred lightweight-migration
path used by DashWallet and verifies that the Core wallet records survive.
