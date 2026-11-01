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

Publish through `PlatformWallet::write_dashpay_profile`. A shielded address that is
one of this wallet's bound tip addresses is published only if preparation, in this
session, proved with the seed that the identity holds the slot selecting it, and the
identity still holds that slot. Discovery can move an identity whose keys derive at
several indices, and a restored placeholder index proves nothing, so prepare the
address again after a restart or a move; preparation is idempotent. Preparation,
publication and tip sends hold a per-wallet placement lock that discovery, loading
and changeset apply take before moving an identity. An address already published
before a move keeps receiving into its old account until the profile is updated.

Tip accounts have distinct viewing keys. Hosts must exclude them from ordinary
receive, balance, and automatic spending choices, as the example apps do.
`identity_tip_account(identity_id)` returns an identity's tip account and whether this
session proved the identity holds its slot (discovery, loading, or a seed check in
preparation or a tip send). A restored index is unverified until then, because
it may be a host placeholder whose account belongs to another identity; show the tip
balance only when verified.
`shielded_balances()` returns all bound accounts separately, including tip
accounts; it does not apply this policy. Spending a tip balance is an explicit
host action.
The generic shielded spend APIs (transfer, unshield, withdraw, top-up) still accept an
explicitly selected account and apply none of the tip-slot checks below; use
`send_shielded_tip` with a `ShieldedTipSource` for tips.
A published address is publicly associated with the profile; account separation
does not eliminate correlations introduced by later transfers or provide a
blanket guarantee against future cryptographic attacks.

Address preparation requires an existing shielded bind. Bind the host's ordinary
accounts first; preparation preserves them while adding the dedicated account.

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

Parse the amount the user typed with `parse_shielded_tip_amount`: exact credits from
ASCII digits and one optional `.` or `,` separator (decimal keypads type the locale's),
refusing signs, grouping, exponents, zero, and fractions finer than one credit. Show the
parsed amount in the confirmation, not the typed text.

Use `wallet.review_shielded_tip(coordinator, username)` to obtain what the user
confirms: the fresh `ShieldedTipRecipient`, how it compares with the recipient last
confirmed for the name from this wallet (`New`, `Unchanged`, or `Changed` with the
previous one, to warn about), and whether an earlier payment to its address is
unresolved, counting the address last confirmed for the name in case the recipient
rotated it. Confirmed recipients are kept per wallet in the shielded store, keyed by
the normalized DPNS label, and survive a shielded Clear; a send records the recipient
it pays.
`wallet.send_shielded_tip(...)` takes that confirmed recipient, re-resolves the
name and profile, and refuses payment if either changed. Its `ShieldedTipSource`
names what pays: `Identity(id)` spends that identity's tip account, after the seed
proves the slot it holds; `Account(n)` spends an ordinary account, or the tip
account of a slot that holds no identity (a retired identity's funds). It never switches to a
transparent destination. This is a snapshot check: the submitted payment always
uses the address that was confirmed, even if the profile changes afterward.

A tip is recorded as unresolved before it is broadcast: its activity row is flushed to
the host, and the shielded store durably records a tip barrier (recipient, notes,
anchor) together with a parked recovery record that keeps its notes reserved across a
relaunch. The barrier is kept apart from note and recovery state, so account purges and
scans do not remove it. While a barrier to an address is pending, a tip to that address
is refused (`ShieldedTipPending`), as is a second tip while one is being sent; an
unreadable barrier blocks its wallet's tips instead of vanishing.

A barrier settles as executed when the store sees one of its notes spent, in the same
step that resolves the note and before its recovery record goes. It settles as failed
only on its own send's definitive rejection: the tip is broadcast to a single node with
no retries, and only that node's consensus rejection, or the committed result of the
wait, counts. Nothing reported later (a re-drive rejection, an anchor missing from one
node's set) settles it as failed, because another node may still hold the tip. A tip
whose outcome stays unknown therefore stays pending until its notes are seen spent, or
until the user, acknowledging that it may still execute, calls
`abandon_unresolved_shielded_tips`. While a tip is pending, a bind that would drop or
re-key its paying account and a shielded Clear are refused.

After a failure, `ShieldedTipDisposition::of(error)` decides whether the tip may be sent
again: `NotSent` (nothing broadcast, or definitively rejected) or `Unresolved` (do not
send again; shielded sync settles it). The C, Swift and Kotlin sends report the same
decision.

Tips to a public profile address are anonymous from the receiver's perspective
unless the payer provides additional context. This feature does not implement
per-contact addresses or authenticated sender attribution.
