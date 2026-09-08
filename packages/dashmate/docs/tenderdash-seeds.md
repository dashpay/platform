# Tenderdash bootstrap seeds

Dashmate ships `configs/defaults/tenderdash-seeds.json`, generated from the on-chain
Core evonode registry for mainnet and testnet. Both default configs consume this
file. Do not edit seed addresses by hand.

## Preparing a release

Run the normal release command; no Core credentials or environment setup is required:

```sh
yarn release
```

Release preparation reads the public quorum servers:

- `https://quorums.mainnet.networks.dash.org/masternodes`
- `https://quorums.testnet.networks.dash.org/masternodes`

These endpoints must support the bootstrap metadata added in
[dashpay/quorum-list-server#14](https://github.com/dashpay/quorum-list-server/pull/14).
The response must have `success: true`, a `data` array, and `lastUpdated` in Unix
seconds. Cache data more than 30 minutes old is rejected. HTTP requests time out
after 15 seconds and do not follow redirects. Old server responses missing the
new fields fail with an actionable error; deploy the server change before using
this default release path.

The generator selects up to 20 distinct evonodes and hosts with `status: ENABLED`
and `versionCheck: success`. It reads `platformNodeID` and prefers registered
`addresses.platform_p2p` endpoints, falling back to the host in `address` with
`platformP2PPort` for older Core registries. Ports are never guessed. At least five
eligible IPv4 peers per network are required; malformed or unsupported endpoints
are skipped. Registry membership and a successful DAPI version check do not
prove that a Tenderdash P2P handshake will succeed.

The committed snapshots record the package version, source URL, and server's
`lastUpdated` timestamp. The HTTPS quorum server is the trusted registry source;
its cache timestamp does not independently prove Core synchronization. No Core
block height, hash, or time is fabricated for this source.

If either network fails, no snapshot is replaced and release preparation stops.
After resolving the error, regenerate before continuing release preparation:

```sh
node packages/dashmate/scripts/generate-tenderdash-seeds.js
node packages/dashmate/scripts/generate-tenderdash-seeds.js --check
```

Commit the generated file with the release changes. The publishing workflow checks
that both snapshots match the package version and are at most seven days old,
even when it reuses cached build artifacts. A delayed release needs refreshed
snapshots in its release commit. Ordinary builds and tests remain offline and
reproducible; publishing never silently changes tagged source data.

### Optional local Core override

To use your own synced Core node instead of a public quorum server, set the
corresponding optional variable (either network can be overridden independently):

```sh
export DASHMATE_MAINNET_CLI='["dash-cli","-conf=/secure/mainnet.conf"]'
export DASHMATE_TESTNET_CLI='["dash-cli","-conf=/secure/testnet.conf"]'
```

The JSON arrays can prefix `dash-cli` with `docker exec <container>` or
`ssh <host> docker exec <container>`. Keep credentials in config or cookie files.
An explicitly configured but invalid override fails rather than silently using
the public server. This path verifies Core synchronization, a tip under 24 hours
old, and a stable block hash around `protx list valid true <height>`. It preserves
Core block provenance in the snapshot and excludes PoSe-banned and locally
Platform-banned entries. It is not an automatic fallback on public-server errors.
Both sources retain the same history of stock seed sets for config migration.

## Existing installations

The generator retains fingerprints of every previous stock seed set. On load,
Dashmate replaces an exact match to any old stock set with the current network's
snapshot, including when the config format version has not changed. It renders
and saves the change under the configuration lock; a failed render is retried on
the next command. Ordering and duplicate entries do not constitute customization.
Custom, partial, empty, and other-network lists are preserved. Keep the fingerprint
history when refreshing snapshots so installations can skip multiple releases.

Snapshots do not update continuously on installed nodes. Nodes receive fresh
bootstrap defaults when they upgrade Dashmate; operators can still supply their
own seeds between releases.
