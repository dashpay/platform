# App Connect Contract

[![Build Status](https://github.com/dashpay/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/release.yml)

System data contract for the wallet-to-app login handshake (DashPay Connect).
It gives the two halves of a login one well-known contract id on every network:

- `loginKeyResponse`: written by a wallet after the user approves an app's
  `connect` request. Carries the app's ephemeral public key hash (the request
  id), the wallet's ephemeral public key, and the session key material
  encrypted to the app. Found by the app through the
  `(contractId, appEphemeralPubKeyHash)` index, which is deliberately not
  unique (the request id is public, so uniqueness would let anyone block the
  wallet's write); the app authenticates each candidate by decrypting it. The
  wallet keeps its response's document id locally and replaces it on re-login.
- `appManifest`: published once by the owner of an app's data contract. Names
  the app, states the contract bounds its login key must carry, the session
  lifetime and budget it asks for, and the encryption key bindings it needs,
  packed as fixed 96-byte records in `encBindings`. Wallets look it up by
  `($ownerId, appContractId)`, so only the contract's owner can publish the
  manifest for it.

Both document types are created by ordinary identities
(`creationRestrictionMode: 0`), mutable and deletable. The contract activates
with protocol version 14. See `docs/protocol/app-connect.md` for the login
flow.

## Table of Contents

- [Install](#install)
- [Contributing](#contributing)
- [License](#license)

## Install

```sh
npm install @dashevo/app-connect-contract
```

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
