# wasm-sdk query review (fetch/fetch_many, proofs, efficiency)

Context: review of query implementations to verify we use fetch/fetch_many correctly, avoid inefficient patterns, and handle proofs.

## Findings

- ✅ `src/queries/token.rs:getTokenPerpetualDistributionLastClaim`
  - **FIXED**: Now prefetches token configuration before making the proof-verified query. The token configuration is fetched via `TokenContractInfo` and `DataContract` queries, extracted, and cached in the trusted context provider before calling `RewardDistributionMoment::fetch()`.
- `src/queries/protocol.rs:getProtocolVersionUpgradeVoteStatus` (and proof variant TODO)
  - Uses `fetch_votes` without a proof-capable path; proof variant remains unimplemented. Protocol vote status cannot currently be verified.
- `src/queries/epoch.rs:getEvonodesProposedEpochBlocksBy*WithProofInfo`
  - Proof-supporting variants are stubbed (return errors). Evonode proposed block counts cannot be fetched with proofs yet.

## Suggested follow-ups

1) Add/enable batch identity key queries (or parallelize) and return combined proofs for `getIdentitiesContractKeys*`.
2) Implement proof-capable variants for protocol vote status and evonode proposed block counts once underlying fetch/proof traits land.
