# WASM return structs

Rust structs that the wasm SDK returns to JavaScript (directly or inside `Result`/`Option`). Use this list to scope TypeScript `toObject` / `toJSON` support in the generated bindings.

## Query response wrappers
- `ProofMetadataResponseWasm` (queries/mod.rs) – proof+metadata envelope used by proof-enabled queries in `dpns`, `data_contract`, `document`, `epoch`, `group`, `identity`, `protocol`, `system`, `token`, `voting`.
- `ResponseMetadataWasm` (queries/mod.rs) – metadata accessor on query responses.
- `ProofInfoWasm` (queries/mod.rs) – proof accessor on query responses.
- `StatusResponseWasm` (queries/system.rs) – `get_status`.
- `CurrentQuorumsInfoWasm` (queries/system.rs) – `get_current_quorums_info`.
- `PrefundedSpecializedBalanceWasm` (queries/system.rs) – `get_prefunded_specialized_balance`.
- `StateTransitionResultWasm` (queries/system.rs) – `get_state_transition_result`.
- `ProtocolVersionUpgradeStateWasm` (queries/protocol.rs) – `get_protocol_version_upgrade_state`.

## Domain-specific query structs
- `RegisterDpnsNameResult`, `DpnsUsernameInfo` (dpns.rs) – DPNS registration/resolution results.
- `IdentityNonceWasm`, `IdentityBalanceWasm`, `IdentityBalanceAndRevisionWasm` (queries/identity.rs) – identity nonce/balance helpers.
- `TokenPriceInfoWasm`, `TokenLastClaimWasm` (Option), `TokenTotalSupplyWasm` (Option) (queries/token.rs) – token info endpoints.
- `ContestedResourceVoteStateWasm` (queries/voting/state.rs) – contested resource vote state queries.

## Wallet / keys
- `KeyPair`, `KeyPairWasm` (wallet/key_generation.rs) – key generation helpers (single and batch).
- `DerivationPath`, `DerivationPathWasm`, `Dip13DerivationPathWasm` (wallet/key_derivation.rs) – derivation path helpers.
- `SeedPhraseKeyInfoWasm`, `PathDerivedKeyInfoWasm`, `DerivedKeyInfoWasm`, `DashpayContactKeyInfoWasm` (wallet/key_derivation.rs, wallet/extended_derivation.rs) – derived key material/introspection.
- `Dip14ExtendedPubKey` (wallet/dip14.rs) – conversion from DIP-14 extended private key.

## SDK construction
- `WasmSdk` (sdk.rs) – returned from `WasmSdkBuilder::build`.
- `WasmSdkError` (error.rs) – error type carried by all exported `Result` signatures.

Non-returned structs (defined but not currently used as return types) are excluded so we can focus `toObject`/`toJSON` work where it impacts consumers today.
