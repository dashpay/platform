# DashPay shielded tips

DashPay profiles can publish a reusable Orchard receiving address. A payer resolves
DPNS to an identity, fetches its profile with proof verification, confirms the
recipient, and sends an ordinary shielded transfer. No contact request or payment
notification document is necessary.

## Address format and updates

`profile.shieldedAddress` contains 43 raw bytes: the 11-byte diversifier followed
by the 32-byte diversified transmission key. Text encoding, network prefix, and
checksum belong to the user interface. External addresses with any valid
diversifier are supported. The profile does not prove ownership of the address.

`DashPayProfile` exposes `core_payment_address`, `platform_payment_address`, and
`shielded_address`. The transparent fields use their existing 21-byte storage
form. `ProfileUpdate` uses `PaymentAddressUpdate::Keep`, `Set(bytes)`, or `Remove`
for each address. An unrelated edit preserves all payment addresses. Clients
validate Orchard decoding before publication and payment; an invalid shielded
address does not prevent displaying the rest of the profile.

Publication checks the connected chain's DashPay contract before submitting an
address field. A bundled schema alone does not demonstrate network activation.

## Dedicated local accounts

The wallet reserves ZIP-32 account indices `0x40000000..0x80000000` for DashPay
tips. A wallet-owned identity at derivation index `i` uses account
`0x40000000 + i`; identity indices outside the lower half are rejected by the tip
helper. Ordinary account allocation must stay below `0x40000000`. Use
`shielded_tip_account_index` and `is_shielded_tip_account` instead of duplicating
these constants in applications.

Call `PlatformWallet::prepare_shielded_tip_address(seed, identity_id, coordinator)`
to obtain the default address of this account. The helper binds its viewing keys
to the synchronization coordinator and flushes persistence before returning. It
does not publish the address; publication remains an explicit profile operation.
Repeated preparation derives the same account and address. An identity without a
wallet derivation index can publish an external address instead.

Tip accounts have distinct viewing keys and are excluded from ordinary receive
and automatic spending choices. Spending a tip balance is an explicit action.
The generic shielded transfer API still accepts an explicitly selected account.
A published address is publicly associated with the profile; account separation
does not eliminate correlations introduced by later transfers or provide a
blanket guarantee against future cryptographic attacks.

## Restoration and address changes

On seed restoration, discover the wallet's identities before binding and scanning
shielded accounts. Seed-backed `bind_shielded` adds each discovered identity's tip
account even if the caller supplies only account zero. This does not depend on the
current profile: removal or replacement of a published address does not remove
received funds from recovery. After discovering more identities in an existing
session, bind again. Newly bound accounts start with their own scan watermark at
zero, so the next synchronization scans their history while retaining the shared
commitment tree and previously synchronized accounts.

Seedless binding includes persisted tip viewing keys, including retired accounts.
If a newly discovered identity's key is missing, it returns `false` so the host can
perform seed-backed binding. Address rotation within a tip account uses a new
diversifier; the account viewing key detects both old and new addresses. Removing
publication neither revokes old address copies nor stops their monitoring.

An externally supplied address is not automatically owned by this wallet. Its
funds, viewing keys, and recovery belong to the external wallet.

## Resolving and paying

Use `wallet.identity().dashpay().resolve_shielded_tip(username)` to obtain a fresh
`ShieldedTipRecipient`. Display the resolved identity and address for confirmation.
`wallet.send_shielded_tip(...)` takes that confirmed recipient, re-resolves the
name and profile, and refuses payment if either changed. It never switches to a
transparent destination. This is a snapshot check: the submitted payment always
uses the address that was confirmed, even if the profile changes afterward.

Tips to a public profile address are anonymous from the receiver's perspective
unless the payer provides additional context. This feature does not implement
per-contact addresses or authenticated sender attribution.
