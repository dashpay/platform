# App Connect: the wallet-to-app login handshake

Protocol version 14 ships the `app-connect` system contract. It gives the two halves of a
wallet-to-app login one well-known contract id on every network: the wallet's encrypted answer
to an app's login request, and the manifest an app publishes so wallets know what it is and what
it needs. Nothing in the handshake is new consensus behaviour; the contract is the on-chain
mailbox and the directory, and Platform validates its documents like any other.

The contract id is `H8F9mP1BM55TE1ShsxPZHzhyinaMdY9bMmP85mkDhcJJ` and the owner id is the
all-zero identifier, like the other system contracts. Both document types are stored, mutable,
deletable, and creatable by any identity (`creationRestrictionMode: 0`).

## Why a system contract

A wallet meets an app it has never seen through a QR code or a deep link that carries nothing
but the app's contract id. To answer, the wallet needs a place to write that the app already
knows how to read, and a place to look up the app that no one else can squat. A contract with
a fixed id on mainnet, testnet and every devnet gives both without a per-network configuration
table in every wallet and every SDK, and lets nodes serve it from the compiled-in system contract
cache like DPNS or DashPay.

## The document types

### `loginKeyResponse`

Written by the wallet after the user approves a `connect` request. From Platform's point of
view a response is per request: nothing ties it to the identity beyond `$ownerId`, and nothing
stops an identity from holding several. The wallet keeps the document id of its response for
each (identity, app) locally and replaces that document on re-login; if it loses the id it
creates a new one, and the old row stays until someone who finds it deletes it. There is
deliberately no `($ownerId, contractId)` index to make that a rule: it measured at roughly 10 M
credits per document for a constraint the wallet enforces itself.

| Property | Type | Meaning |
|---|---|---|
| `contractId` | identifier, `refersTo: contract` | The app's data contract. The reference means the contract must exist when the document is written. |
| `appEphemeralPubKeyHash` | 20 bytes | `hash160` of the ephemeral public key the app put in its request. It identifies the request, and it is what the app polls for. |
| `walletEphemeralPubKey` | 33 bytes | The wallet's compressed ephemeral public key. The app combines it with its own ephemeral private key to derive the shared secret. |
| `encryptedPayload` | 60 to 572 bytes | The private keys the wallet grants the app, encrypted to the shared secret: a 28-byte envelope followed by 32 bytes per key, one to seventeen keys (the session key plus up to eight bindings with both purposes). |

All four are required. The single index, `byContractAndEphemeralKey` on
`(contractId, appEphemeralPubKeyHash)`, lets the app fetch the answers to its request with a
two-value equality query and a proof.

The index is deliberately **not** unique. The app's ephemeral public key is public (it is in
the QR code), so a unique index that does not include `$ownerId` would let any observer
pre-create a row under the request id and block the wallet's write. Instead the app checks
every candidate it gets back. The payload is AES-GCM, and its tag is computed over additional
data that binds `contractId`, `appEphemeralPubKeyHash` and `walletEphemeralPubKey`, so
decryption filters out rows not written by a party holding the shared secret. A row from a party
that did compute the secret (anyone who saw the QR can, since `e` is public) is caught by the
identity check in [The app](#the-app) below. Either way a squatter's row costs the squatter a
document fee and is ignored. Apps must therefore query by both index values and try each result
rather than assume there is exactly one.

### `appManifest`

Published once by the owner of an app's data contract and updated when the app's requirements
change. Consensus refuses a manifest from anyone but the contract's owner: `appContractId`
carries the owner gate, `refersTo: { type: contract, propertyAgreement: { "$ownerId":
"$ownerId" } }`, so a create or replace whose writer is not the referenced contract's owner is
rejected (`ReferencedDocumentPropertyMismatchError`). A wallet therefore fetches the manifest by
`appContractId` and needs no owner check of its own.

| Property | Type | Meaning |
|---|---|---|
| `appContractId` | identifier, `refersTo: contract` with the owner gate | The app's data contract. Only its owner can write this document. |
| `name` | string, at most 64 characters | Display name, shown on the wallet's approval sheet. |
| `url` | string, at most 256 characters | The app's URL, shown on the approval sheet. |
| `authBoundsKind` | integer 0 to 3 | The contract bounds the login key must carry: `0` none, `1` the contract in `authBoundsId`, `2` the document type `authBoundsDocType` of that contract, `3` the contract group in `authBoundsId`. |
| `authBoundsId` | 32 bytes, optional | The contract or contract group id the bounds name. Absent when `authBoundsKind` is `0`. |
| `authBoundsDocType` | string, at most 64 characters, optional | The document type name, present only when `authBoundsKind` is `2`. |
| `sessionSeconds` | integer | The login key lifetime the app asks for, in seconds. |
| `sessionBudget` | integer | The login key budget the app asks for, in credits. |
| `requestedEncryptionKeys` | 0 to 768 bytes, optional | The encryption keys the wallet should register on the identity at login, packed as fixed 96-byte records (below). |

`appContractId`, `name`, `url`, `authBoundsKind`, `sessionSeconds` and `sessionBudget` are
required. The single index, `byApp` on `(appContractId)`, is unique: one manifest per app
contract, and with the owner gate, one that only the contract's owner could have written.

A contract's owner never changes, so the gate is fixed for the contract's lifetime and an
existing manifest always belongs to the current owner. Wallets should still treat a manifest
whose `$ownerId` differs from the contract's owner as absent: that cannot happen through
consensus today, but it is the invariant the wallet relies on, and checking it costs nothing
once both documents are in hand.

The bounds are a requirement: the wallet registers the login key with exactly those
`contractBounds` (see [contract-bound authentication keys](contract-bound-authentication-keys.md)),
or refuses. The lifetime and budget are the app's request; the wallet treats them as a default
and may grant less or more (see
[authentication keys with a budget or an expiry](authentication-key-limits.md)).
`authBoundsKind = 0` is legal; the expiry and budget still apply, only the scope does not.

#### `requestedEncryptionKeys`

The app cannot register keys on the user's identity; the wallet does that at login. Encryption
keys are per data contract (a Yappr user holds one pair for its DM contract, one for its social
contract, and so on), so the app lists the contracts and document types it will encrypt under
and which purposes it needs on each. At login the wallet walks this list, checks which of those
keys the identity already holds, and adds the missing ones in the same identity update as the
login key.

Document schemas admit byte arrays but not arrays of objects, so the list is packed. Each record
is 96 bytes:

| Offset | Size | Content |
|---|---|---|
| 0 | 32 | The id of the data contract the keys serve. |
| 32 | 1 | Purpose mask: bit 0 asks for an ENCRYPTION key, bit 1 for a DECRYPTION key. |
| 33 | 63 | The document type name, zero-padded; all zero for a contract-level key. |

The array holds zero to eight records, so its length is a multiple of 96 up to 768. For each
record and each purpose bit, the wallet makes sure the identity holds an enabled key with that
purpose, bound to that contract (or that document type of it), and registers one if it does not.
The bound contract or document type has to opt in with `requiresIdentityEncryptionBoundedKey` or
`requiresIdentityDecryptionBoundedKey` for the key to be registrable.

Platform does not parse `requestedEncryptionKeys`; it is a byte array to consensus, and the
layout above is a convention between apps and wallets. The Rust crate exposes the offsets and the
purpose bits as constants under
`app_connect_contract::v1::document_types::app_manifest::requested_encryption_keys`.

## The login flow

### The request

The app shows a QR code, or opens a deep link on the same device, carrying a `connect` URI:

```text
dashpay://connect?v=2&n=<network>&exp=<unix seconds>&app=<app contract id>&e=<app ephemeral public key>
```

`e` is fresh for every request. Its `hash160` is the request id under which the wallet answers.
The app keeps the matching private key in memory until the answer arrives.

### The wallet

1. Fetches the app contract, then the manifest whose `appContractId` matches. Either missing,
   the request is refused. Consensus guarantees the manifest was written by the contract's
   owner.
2. Lets the user choose an identity if the wallet holds more than one usable one.
3. Derives the session key for this request (its derivation leaf is the request id, so a
   different `e` yields a different key). If that key is already on the identity and not
   expired, the same request was already served: skip to step 6 with it.
4. For every `requestedEncryptionKeys` record, checks the identity for the bound keys it asks
   for and plans to add the missing ones.
5. Shows the approval sheet: the app's name and URL, the key's lifetime and budget (the wallet's
   grant, which the user can shorten), the bounds, and any encryption keys that will be added.
   On approval, broadcasts one identity update that adds the login key with the manifest's
   bounds and the granted limits, plus any missing bound encryption keys.
6. Encrypts the granted private keys to the app's ephemeral key and creates, or replaces, its
   `loginKeyResponse` for this app. If the identity update failed, nothing is published.

### The app

Polls the `loginKeyResponse` documents whose `contractId` is its own contract and whose
`appEphemeralPubKeyHash` is `hash160(e)`. For each result the app derives the shared secret from
`walletEphemeralPubKey` and tries to decrypt the payload; the first one whose authentication tag
verifies is the candidate answer, and the rest are discarded. The app then reads `$ownerId` from
that document, verifies that each granted key's public half is a live key on that identity, and
treats that identity as the logged-in user. It is logged in until the key expires or its budget
runs out, at which point it starts a new `connect`.

**Who logs in.** Whoever scans the request logs in, as with any QR or passkey login: if a
second party scans the same code first, the app is logged into that party's identity. The app
shows the identity it is logged in as; users verify it the same way they verify any account they
sign into. Platform gives no stronger binding because the request carries no user secret by
design (it is shown in the open).

### Signing outside the login key's scope

Anything the login key cannot sign (a DPNS registration, a DashPay contact request, a token
purchase) goes through a separate `sign` request: the app hands the wallet a complete unsigned
state transition, the wallet describes it on a sheet, and the user approves each one. That flow
does not touch this contract.

## What Platform enforces

Platform validates both document types against their schema, keeps the manifest index unique,
checks that `contractId` and `appContractId` name existing contracts, refuses a manifest whose
writer is not the owner of the contract `appContractId` names, and bills the writer. The login
key's scope, lifetime and budget are enforced by the identity key itself once registered.

What Platform does not check:

- the dependency between `authBoundsKind` and `authBoundsId` / `authBoundsDocType`: the schema
  admits a kind of `1`, `2` or `3` without an id, a kind of `2` without a document type, and an
  id or document type beside a kind of `0`. Wallets must refuse a manifest whose bounds fields
  do not match its kind;
- the layout of `requestedEncryptionKeys`;
- whether a response belongs to a real request, or was written by the wallet the request was
  made to. The app authenticates every response by decrypting it.

Those are the wallet's and the app's to verify, and all are cheap because the indexes let them
ask for exactly the documents they expect.

## Activation

The contract activates with protocol version 14. A chain born at protocol version 14 or later
registers it at genesis alongside the other system contracts; a chain upgrading from protocol
version 13 receives it from `transition_to_version_14` on the first block of the new version.
Below protocol version 14 the contract does not exist in state, and the system contract cache
reports it absent so that a lookup is billed exactly as it would be on a node that has not
upgraded.
