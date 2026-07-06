# DashPay ignore — cross-device design + DoS / social-graph-leak analysis

Status: the single-device **"Block sender"** design this file originally carried was
**superseded** by the shipped local-only **Ignore** (per-sender, reversible;
`ignored_senders`, `ignore_sender`/`unignore_sender`, applied in `changeset/apply.rs`).
That feature is documented in `SPEC.md` (G5) and `SYNC_CORRECTNESS_SPEC.md`; the
single-device Block design — its state/persistence/UI/test plan and the review
resolutions specific to it — is no longer reproduced here.

This file is retained **only** for the two forward-looking pieces Ignore does not yet
cover:

- **(a) Cross-device ignore** — how to make ignore sync across a user's devices via a
  single owner-scoped, self-encrypted blocklist, and the privacy reason it is **not**
  carried via `contactInfo` (§1).
- **(b) DoS / social-graph-leak analysis** — the fetch-cost / flood analysis and the
  countable-index social-graph leak that any query-level DoS filter must avoid (§2).

Owner: platform-wallet / swift-sdk.
Relates to: `SPEC.md` (G5, the shipped Ignore), `SYNC_CORRECTNESS_SPEC.md`,
`CONTACTINFO_FORMAT_SPEC.md`.

---

## 1. Cross-device ignore — a self-encrypted blocklist, NOT `contactInfo`

Ignore is per-device local state today; you re-ignore a sender on each device. Making
it sync is a **future** item on the contract / governance track — not built.

**Why not `contactInfo`.** The tempting reuse — carry an ignore flag in the
`contactInfo.privateData` blob we already sync — **breaks the DIP-15 ≥2-contacts
unlinkability gate** and is rejected. An ignore targets a *non-established* sender, so
carrying it would mean creating a `contactInfo` **about a non-contact**. That document's
public existence + `$createdAt` correlates with the inbound `contactRequest` (via the
public `userIdCreatedAt` index) to re-identify *who* you ignored: `encToUserId` is
encrypted so *who* is hidden, but the doc's *existence/count* is not. The
"`displayHidden` is precedent" argument is a false equivalence — `displayHidden` rides a
document that exists anyway (an established contact), whereas an ignore-of-a-non-contact
*creates* the leaking document. It is also mechanically blocked today:
`set_contact_info_with_external_signer` → `set_contact_metadata` hard-requires an
established contact, and the apply side drops non-established `contactInfo`.

**The design if it ships.** A **single owner-scoped, self-encrypted blocklist
document** — one document the owner encrypts to themselves (same key family as
`contactInfo`'s `privateData`, but a single owner-private list, not per-contact and not
gated by the 2-contact rule). Every device reads and applies it, so ignore (and
optionally decline) apply everywhere. Costs: each edit is a document write (credits);
it reveals only *that* a blocklist exists plus an edit count — **not** one document per
ignored victim. Its update timing should be conflated with normal profile edits so it
does not leak the per-sender existence/count. This is a contract change on the later
governance track, and the metadata-leak analysis above must be settled before building
it.

## 2. DoS / spam, and the social-graph leak a countable index would create

### 2.1 Fetch model, and why an ignore can't cut fetch cost

The received-request query is keyed by recipient:

```
where toUserId == me, order_by $createdAt, limit: 100
```

An ignore is a **local read-filter applied after fetch**: the index has no
`sender NOT IN (…)` axis and Sybil senders are unpredictable, so an ignore cannot avoid
the fetch + GroveDB proof-verify cost of an incoming request — it only hides it once
fetched.

**Threat: a sender (or a funded Sybil swarm) creates many requests.** Invalid ones are
the worst — they fail parse/validation but still cost fetch + proof-verify + parse. The
only built-in deterrent is **economic**: each `contactRequest` costs the sender
platform credits. Spam isn't free, but it isn't prevented. A naive `limit: 100,
start: None` re-fetch also lets a flood of ≥100 junk requests **bury** legitimate ones
past the first page.

**Mitigation — incremental fetch (high-water).** Track the newest `$createdAt` seen per
identity and query `WHERE toUserId == me AND $createdAt > high_water`, paginating
forward: each request is fetched exactly once, pagination can't bury legit requests past
100, and ignore/decline become one-time-on-first-sight. This bounds steady-state work to
O(new requests per sweep). It does **not** stop the *first* fetch of a request from a new
sender (impossible without server-side sender exclusion), but nothing in the protocol
can. The existing `userIdCreatedAt` index `[toUserId, $createdAt]` already serves this
range-after-equality query, so incremental fetch needs **no** contract change. *(This
high-water incremental fetch has since shipped — see `SYNC_CORRECTNESS_SPEC.md` and the
DIP-15 §8.8/§8.12 row in `DIP_CONFORMANCE_GAPS.md`.)*

### 2.2 The trap: a countable `[toUserId, $ownerId]` index leaks the inbound social graph

A natural-looking next step is a **countable** index on the recipient→sender axis so the
wallet can answer "how many pending requests do I have" / "is one sender flooding me"
from a count proof **without fetching documents**:

```
byRecipientSender = [{ toUserId: asc }, { $ownerId: asc }]   // countable — DO NOT ship as drafted
```

(The `$ownerId` of a `contactRequest` *is* the sender.) This is a **social-graph leak**
and must not ship in that form. Platform count / group-by proofs are **public, not
recipient-private**, and return cleartext `{sender_id → count}`. A countable
`[toUserId, $ownerId]` therefore lets *anyone* scrape "who contacted recipient R, with
counts" in O(log n) — the inbound social graph, in the clear.

**Resolution:** drop the per-sender `GROUP BY $ownerId` axis. At most keep an aggregate
`COUNT(*) WHERE toUserId == me` (a single number, for a pending-request badge), which
reveals only a total and not per-sender edges. Any real query-level DoS filter that
excludes ignored/rejected senders *before* fetching is a contract change (DIP /
maintainer coordination), and this graph-exposure analysis must be carried into that DIP.
