# Upstream bincode compatibility

Platform and GroveDB use `grovedb-bincode`, imported as `bincode`. Some dependencies,
including rust-dashcore, still request crates.io `bincode` 2.0.1. This package
re-exports the fork so those dependencies implement the same ordinary `Encode`
and `Decode` traits. It contains no codec implementation and grants no untrusted
decoding implementations to upstream types.

The root workspace selects this package with `[patch.crates-io]`. Cargo only reads
patches from the consuming application's workspace root. Applications depending
on Platform through a Git or path dependency must therefore add the same patch,
pointing to this package in their Platform checkout:

```toml
[patch.crates-io]
bincode = { path = "path/to/platform/packages/bincode-compat" }
```

Keep the checkout revision aligned with the Platform dependency. The adapter can
be removed once the upstream dependencies use `grovedb-bincode` themselves.

Platform's `PlatformDeserialize` derive selects untrusted native decoding by
default. Serialized types and their fields need `DecodeUntrusted` in addition to
ordinary `Decode` when both APIs are used. Local mock formats that contain
ordinary-only foreign types explicitly use `#[platform_serialize(unversioned,
trusted)]`. That exception must not be used for network input or imported data.
The separate `PlatformVersionedDecode` APIs retain their ordinary decoding
contract for existing internal and mock callers.
