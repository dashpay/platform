# Tenderdash bootstrap seeds

Dashmate ships `configs/defaults/tenderdash-seeds.json`, generated from the on-chain
Core evonode registry for mainnet and testnet. Both default configs consume this
file. Do not edit seed addresses by hand.

## Preparing a release

Provide access to a synced Core node on each network using JSON argument arrays:

```sh
export DASHMATE_MAINNET_CLI='["dash-cli","-conf=/secure/mainnet.conf"]'
export DASHMATE_TESTNET_CLI='["dash-cli","-conf=/secure/testnet.conf"]'
yarn release
```

The arrays can also prefix `dash-cli` with `docker exec <container>` or
`ssh <host> docker exec <container>`. Keep authentication in the node's config or
cookie file, not command arguments. Generation runs `getblockchaininfo`,
`protx list valid true <height>`, and `getblockhash <height>`; it does not modify
Core. Both nodes must have finished initial sync, have caught up to their headers,
and have a tip less than 24 hours old.

`yarn release` regenerates both snapshots before creating the release commit.
Generation records the Core height, hash, time, and package version. It selects up
to 20 distinct evonodes and hosts, excluding PoSe-banned and locally Platform-banned
entries. Registered Platform endpoints take precedence over legacy service IPs.
The current generator uses IPv4 endpoints, which all Dashmate transports support.
At least five candidates per network are required. These are registry-derived
bootstrap peers; registry membership does not guarantee a live Tenderdash handshake.

If either network fails, no snapshot is replaced and release preparation stops.
Fix Core access and retry generation before continuing release preparation:

```sh
node packages/dashmate/scripts/generate-tenderdash-seeds.js
node packages/dashmate/scripts/generate-tenderdash-seeds.js --check
```

Commit the generated file with the release changes. The publishing workflow checks
that both snapshots match the package version and are at most seven days old,
even when it reuses cached build artifacts. A delayed release needs refreshed
snapshots in its release commit. Ordinary builds and tests remain offline and
reproducible; publishing never silently changes tagged source data.

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
