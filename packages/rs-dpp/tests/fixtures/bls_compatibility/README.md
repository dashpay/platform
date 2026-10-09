# BLS compatibility vectors

These public test keys and fixtures were generated before the backend migration,
from Platform `c0b4b97f8a1a8626c2f51bc8d7baeab9c65e10da` (#5307):

- `dashpay/agora-blsful` at `0c34a7a488a0bd1c9a9a2196e793b303ad35c900`;
- `blstrs_plus` 0.8.18;
- `blst` at `71a00877c75a482e7d44c4e6370c1a7fb82444be` (0.3.12).

`vectors.json` contains 16 Basic signatures (four canonical scalars × empty,
text, zero digest and 256-byte messages), five secure aggregates (one/two/three
signers, reordered keys and a duplicate key), three key-generation cases, four
scalar boundaries, and three validator/validator-set storage cases. Storage uses
`grovedb-bincode` 2.1.0 with `standard().with_big_endian()` and DPP's JSON schema.

The scalar and infinity cases characterize **historical compatibility**, not
recommended validation for a new protocol. DPP accepts nonzero scalars reduced
modulo the group order and permits an identity public key to deserialize; an
identity signature still fails verification. Replacing those rules requires a
separate protocol decision.

The frozen expected values are copied unchanged from #5307. This standalone
suite exercises the backend on `v5.1-dev` without adding the candidate backend
as a dependency. The generator uses the original blsful API.

Reproduce the corpus with the [historical generation script](../../../../../scripts/historical-test-vectors/README.md).
It runs ignored tests on a pinned checkout and writes a separate output directory.

Keep the frozen fixture unchanged when migrating the implementation. These tests
cover the listed cases, not an exhaustive proof of equivalence or every persisted
Platform structure. They do not test legacy signature hashing; the legacy field
is the public-key encoding of the same point.
