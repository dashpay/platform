# Tenderdash bootstrap seeds

Dashmate's mainnet and testnet defaults use `configs/defaults/tenderdash-seeds.json`.
Generate it from the public quorum list servers; do not edit seed addresses by hand:

- `https://quorums.mainnet.networks.dash.org/masternodes`
- `https://quorums.testnet.networks.dash.org/masternodes`

## Releases

`yarn release` refreshes both networks after bumping the package version. To retry:

```sh
node packages/dashmate/scripts/generate-tenderdash-seeds.js
node packages/dashmate/scripts/generate-tenderdash-seeds.js --check
```

Both servers must deploy the bootstrap metadata API from
[dashpay/quorum-list-server#14](https://github.com/dashpay/quorum-list-server/pull/14).
It supplies `lastUpdated` in Unix seconds, `platformNodeID`, and registered P2P
endpoints. Generation rejects missing metadata, caches older than 30 minutes,
HTTP errors, redirects, and requests exceeding 15 seconds. Neither snapshot is
replaced unless both networks succeed. There is no Core RPC override.

### Quorum-server outage

If a server is down or its cache is older than 30 minutes when a release must go
out, an operator can explicitly carry the last validated snapshot forward:

```sh
REUSE_TENDERDASH_SEEDS=1 yarn release ...
# or, by hand:
node packages/dashmate/scripts/generate-tenderdash-seeds.js --reuse-snapshot
```

Reuse never contacts the servers and never rewrites peers, `source`,
`lastUpdated`, or `previousSeedSetHashes`; it only sets the snapshot's top-level
`version` to the new package version, which the publishing check compares. The
committed snapshot must still pass that same check, including the seven-day age
limit, so a snapshot too old to publish is also too old to reuse. The script
prints a warning with the date the reused data was generated and the last date
the release can still be published; after that, publishing fails and the
snapshot has to be regenerated from a working server. Reuse is a manual choice
and is never selected automatically.

The generator samples up to 20 enabled, version-checked evonodes using fresh
cryptographic randomness after reading the registry. Each identity gets one
chance regardless of duplicate entries or extra endpoints; selected hosts are
unique. It sorts only the selected output, so low node IDs receive no preference.
It uses registered `addresses.platform_p2p` IPv4 endpoints, or `address` plus
`platformP2PPort` when the server supplies the legacy address format. It never
guesses ports and requires at least five eligible peers per network.

Commit the generated snapshot with the release. The NPM publishing workflow validates
its source URL, package version, peers, and timestamp offline, rejecting data over
seven days old. Publishing never regenerates tagged data; ordinary builds and
tests need no live server. The old committed Core-derived snapshots must be
regenerated after server deployment before this PR can be released.

## Existing installations

Generation retains fingerprints of previous stock seed sets. On load, Dashmate
replaces any complete historical stock set, including reordered or duplicated
entries, with current defaults. Custom, partial, empty, and other-network lists
remain unchanged. Keep this history so upgrades can skip releases.

Updates render and save under the configuration lock; failed rendering leaves the
old configuration on disk for retry. Installed nodes receive refreshed seeds when
they upgrade Dashmate. Operators can still configure their own seeds.

The HTTPS quorum server is trusted for registry data and freshness. A successful
DAPI version check does not guarantee a working Tenderdash P2P connection.
