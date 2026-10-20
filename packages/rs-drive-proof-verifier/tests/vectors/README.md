# Proof-vector regression corpus

Fixture cases generated from a real Drive state (platform v4.0.0 fixtures,
protocol version 12, grovedb 5.0.0), replayed through the crate's public
`FromProof` entry points. The same fixtures are replayed byte-exact by Dash
Core's platform GUI implementation, so drift between what Drive proves and
what any client verifies fails loudly here.

Each case directory contains `manifest.json` (request parameters, block
metadata, expected outcome, pinned root hash) plus `proof.hex`,
`signature.hex`, and `quorum_pubkey.hex`. Loaders live in
`../common/mod.rs`; the suite is gated behind the `mocks` feature.

## Coverage matrix — what each family actually exercises

| family | grovedb proof replay | tenderdash BLS check | notes |
|---|---|---|---|
| identity (4 cases) | ✅ | ✅ | full pipeline through `FromProof` |
| contested vote state (3) | ✅ | ✅ | incl. `Ok(None)` proof-of-absence |
| quorum-sig (4 fixtures / 7 cases) | ✅ | ✅ | 1 positive + 3 negatives on-disk (tampered sig, wrong key, wrong block-id hash) + 3 in-test negatives that tamper the `quorum-sig-valid` response's `ResponseMetadata` (height, time_ms, core_chain_locked_height) to prove those fields are inside the signed `StateId` too |
| documents / DPNS / DashPay (4) | ✅ | ❌ (not reached) | fixture state stores placeholder payloads at document positions, so `FromProof` fails at document decode *before* the signature check; these cases pin the `DriveDocumentQuery` shape (root hash + serialized payloads byte-for-byte via `verify_proof_keep_serialized`) and the clean-`Err` decode failure |
| identity-balance corrupted proof (1) | ✅ (rejects) | ❌ (not reached) | negative: bit-flipped proof fails as a GroveDB error |

The `quorum-sig-valid` case shares its proof bytes with `identity-balance`
(the corpus has 15 distinct fixtures across 16 on-disk cases): all drive
fixtures commit to the same root hash, which is exactly the app hash the
quorum signature signs — that is what lets the positive cases run a genuine
BLS verification with real fixture key material. The 3 metadata-tamper
negatives in `vectors_quorum_sig.rs` reuse `quorum-sig-valid`'s on-disk bytes
and mutate the `ResponseMetadata` in-test rather than adding new fixture
directories, bringing the total to 19 `#[test]` functions over 16 fixture
directories.

Regenerate only deliberately (fixture-generation lives with the Dash Core
platform GUI's vector tooling); a regeneration should be its own commit.
