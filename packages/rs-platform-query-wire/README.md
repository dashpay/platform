# platform-query-wire

Shared wire→drive decoding for Dash Platform queries.

This micro-crate is the single home of the decoders that map query
wire-proto types (from `dapi-grpc`) onto `drive::query` types —
currently the v1 `getDocuments` surface (WHERE / ORDER BY / HAVING /
SELECT clauses and their field values).

## Scope

- **Decode only.** No transport, no networking, no proof
  verification, no async runtime.
- Errors surface through a neutral [`DecodeError`] (`InvalidArgument`
  for malformed wire input, `Unsupported` for well-formed input naming
  a capability the target cannot represent yet); each consumer maps it
  onto its own error surface.

## Why a dedicated crate

The decode of a wire request into a rich query is an equivalence
contract at a trust boundary: a client-side proof verifier must
interpret a request exactly as the server does, or a proof could
verify against a different query than the server answered.

- The **server** (`rs-drive-abci`) decodes every incoming v1
  `getDocuments` request through this crate.
- **Client-side verifiers** (SDK proof verification) are intended to
  decode through the same functions, so server and client wire
  interpretation cannot drift.

Hosting the shared code here — rather than in a client/SDK crate —
keeps the consensus server's dependency graph free of client-flavored
dependencies: this crate's dependencies are a strict subset of what
`rs-drive-abci` already carries.
