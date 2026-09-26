# dashvm-validation

Deterministic admission, structural bounding and logical-stack instrumentation of DashVM
contract code: the pure function from the canonical WebAssembly an author submits to the
prepared WebAssembly the engine compiles, under the DashVM table of a protocol version.

The crate has no engine, storage or state. The node and the developer tooling share it so
"will this bundle be admitted" has one answer; authority and capability checks live above it.

## What it does

Per module, cheap checks first:

1. the canonical byte cap, before any decoding;
2. full validation by the pinned `wasmparser` (the line Wasmtime 36 ships with) under an
   explicit allowlist: mutable globals, sign extension, saturating float to int, multi-value,
   bulk memory, nullable `funcref`; every other proposal is classified to a typed rejection;
3. structural caps read from the profile: functions, types, params, locals, exports,
   operators per function and per module, basic blocks, nesting, table elements, memory pages,
   data segment bytes;
4. memory and table shape: one defined 32-bit memory exported as `memory`, at most one
   defined `funcref` table with a maximum, nothing imported;
5. the import allowlist (`dash_host` envelope functions with exact signatures, `dash:<name>`
   bundle bindings; `dash_vm` is reserved) and the export rules (`dash_alloc`, at least one
   entry export, no table exports, no re-exported imports);
6. start sections rejected, custom sections stripped;
7. the portable logical-stack instrumentation: two shared host globals and a trap import, a
   per-function frame cost burned into every wrapped direct call, thunks for functions
   reachable through the table or an export;
8. re-validation of the output and a provenance check against the submitted module and the
   instrumenter's report (any drift is an internal error, never a paid rejection);
9. canonical and prepared SHA-256 hashes, domain separated.

Per bundle: validated names, bindings resolved against the target's exports with exact
signatures and cross-checked against the declared list, an acyclic dependency graph with a
deterministic initialisation order (Kahn's algorithm, canonical-name tie-break), entry checks
and a bundle digest.

## Where the numbers come from

Every limit is read from `PreparationProfile`, the projection of `PlatformVersion::dashvm`
(`packages/rs-platform-version/src/version/dashvm_versions`). The crate defines no number of
its own; a protocol version whose table is `None` cannot build a profile.

## Tests

```bash
cargo test -p dashvm-validation
```

The instrumentation golden fixture under `src/tests/fixtures` pins the exact prepared bytes of
generation 0. It may only be regenerated (`cargo test -p dashvm-validation
regenerate_golden_fixture -- --ignored`) together with a new preparation generation.
