## [2.0.0](https://github.com/dashpay/platform/compare/v2.0.0-rc.16...v2.0.0) (2025-06-26)


### ⚠ BREAKING CHANGES

* **platform:** properly use withdrawal system data contract. (#2675)

### Features

* **dpp:** add detailed interval evaluation explanation functionality ([#2662](https://github.com/dashpay/platform/issues/2662))
* replace diskusage with diskusage-ng for improved functionality ([#2680](https://github.com/dashpay/platform/issues/2680))
* **sdk:** fetch token contract info ([#2670](https://github.com/dashpay/platform/issues/2670))


### Bug Fixes

* **drive-abci:** fixed issue with adding a key with contract bounds ([#2673](https://github.com/dashpay/platform/issues/2673))
* **platform:** properly use withdrawal system data contract. ([#2675](https://github.com/dashpay/platform/issues/2675))


### Continuous Integration

* add gRPC coverage check and cache management ([#2667](https://github.com/dashpay/platform/issues/2667))


### Miscellaneous Chores

* **platform:** remove dash devs discord from readme ([#2668](https://github.com/dashpay/platform/issues/2668))
* **release:** update changelog and bump version to 2.0.0-rc.17 ([#2674](https://github.com/dashpay/platform/issues/2674))
* **release:** update changelog and bump version to 2.0.0-rc.18 ([#2681](https://github.com/dashpay/platform/issues/2681))
* remove unused token meta schema and references ([#2677](https://github.com/dashpay/platform/issues/2677))
* update js dependencies to latest versions ([#2678](https://github.com/dashpay/platform/issues/2678))
* update minimatch to version 9.0.5 and brace-expansion to version 2.0.2 ([#2672](https://github.com/dashpay/platform/issues/2672))

## [2.0.0-rc.16](https://github.com/dashpay/platform/compare/v2.0.0-rc.15...v2.0.0-rc.16) (2025-06-10)


### Features

* **platform:** add finalized epoch infos query and proof functionality ([#2665](https://github.com/dashpay/platform/issues/2665))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.0.0-rc.16 ([#2666](https://github.com/dashpay/platform/issues/2666))

## [2.0.0-rc.15](https://github.com/dashpay/platform/compare/v2.0.0-rc.14...v2.0.0-rc.15) (2025-06-08)


### ⚠ BREAKING CHANGES

* **sdk:** make document state transition entropy optional, will do a replace if revision is not 1. (#2658)

### Features

* **dpp:** export more token transition fields ([#2655](https://github.com/dashpay/platform/issues/2655))
* **sdk:** add sdk wrappers for easily pushing document transitions for create, delete, purchase, replace, set price, and transfer to platform ([#2659](https://github.com/dashpay/platform/issues/2659))
* **sdk:** add token state transition functionalities to rs-sdk ([#2657](https://github.com/dashpay/platform/issues/2657))
* **sdk:** make document state transition entropy optional, will do a replace if revision is not 1. ([#2658](https://github.com/dashpay/platform/issues/2658))


### Bug Fixes

* fixes issue [#2653](https://github.com/dashpay/platform/issues/2653) Cannot decode DataContractCreateV1 with WASM-DPP ([#2654](https://github.com/dashpay/platform/issues/2654))
* **platform:** npm audit security for tar-fs ([#2656](https://github.com/dashpay/platform/issues/2656))
* **platform:** resolve direct purchase from self issue causing chain stall ([#2663](https://github.com/dashpay/platform/issues/2663))


### Documentation

* add CLAUDE.md for development guidance ([#2652](https://github.com/dashpay/platform/issues/2652))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.0.0-rc.15 ([#2664](https://github.com/dashpay/platform/issues/2664))

## [2.0.0-rc.14](https://github.com/dashpay/platform/compare/v2.0.0-rc.13...v2.0.0-rc.14) (2025-05-29)


### ⚠ BREAKING CHANGES

* **dpp:** set minimum intervals for perpetual distribution (#2622)

### Features

* add checks for authorized action takers in data contract create and update validations ([#2647](https://github.com/dashpay/platform/issues/2647))
* **dpp:** add validation for minimum group member count ([#2646](https://github.com/dashpay/platform/issues/2646))
* **dpp:** set minimum intervals for perpetual distribution ([#2622](https://github.com/dashpay/platform/issues/2622))
* **sdk:** add DataContractMismatch enum for detailed contract comparison ([#2648](https://github.com/dashpay/platform/issues/2648))


### Tests

* **drive:** add test for invalid owner on document delete ([#2643](https://github.com/dashpay/platform/issues/2643))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.0.0-rc.14 ([#2649](https://github.com/dashpay/platform/issues/2649))

## [2.0.0-rc.13](https://github.com/dashpay/platform/compare/v2.0.0-rc.12...v2.0.0-rc.13) (2025-05-28)


### ⚠ BREAKING CHANGES

* **platform:** load data contracts in their respective versions (#2644)

### Features

* **platform:** add token contract info and query ([#2641](https://github.com/dashpay/platform/issues/2641))


### Bug Fixes

* **dpp:** unclear error message for missing document types if no tokens defined ([#2639](https://github.com/dashpay/platform/issues/2639))
* **drive:** proved identity update was giving error ([#2642](https://github.com/dashpay/platform/issues/2642))
* **platform:** load data contracts in their respective versions ([#2644](https://github.com/dashpay/platform/issues/2644))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.0.0-rc.13 ([#2645](https://github.com/dashpay/platform/issues/2645))

## [2.0.0-rc.12](https://github.com/dashpay/platform/compare/v2.0.0-rc.11...v2.0.0-rc.12) (2025-05-26)


### ⚠ BREAKING CHANGES

* **dpp:** disable changes to perpetual distribution in token configuration (#2627)

### Features

* **dpp:** add marketplace rules to token configuration ([#2635](https://github.com/dashpay/platform/issues/2635))
* **dpp:** disable changes to perpetual distribution in token configuration ([#2627](https://github.com/dashpay/platform/issues/2627))
* **sdk:** add token payment info to put_document ([#2630](https://github.com/dashpay/platform/issues/2630))


### Bug Fixes

* **dashmate:** sync max-tx-bytes between tenderdash and drive ([#2625](https://github.com/dashpay/platform/issues/2625))
* **dpp:** allow changing main control group for token configuration ([#2628](https://github.com/dashpay/platform/issues/2628))
* **dpp:** correct stepwise distribution logic in evaluate.rs ([#2636](https://github.com/dashpay/platform/issues/2636))
* **dpp:** missing tags on Groups needed for deserialization ([#2624](https://github.com/dashpay/platform/issues/2624))
* **drive:** ignore time based update fields in proof verification of data contract updates ([#2634](https://github.com/dashpay/platform/issues/2634))
* **platform:** ensure document types only target valid tokens for token payments ([#2631](https://github.com/dashpay/platform/issues/2631))
* **platform:** fix evonode distribution for token perpetual distribution (part 1) ([#2623](https://github.com/dashpay/platform/issues/2623))
* **platform:** paying for a document action with tokens where tokens would be transferred to yourself as contract owner was breaking ([#2633](https://github.com/dashpay/platform/issues/2633))
* **platform:** resolved grovedb error during signing group action finalization on check tx ([#2629](https://github.com/dashpay/platform/issues/2629))


### Tests

* **drive-abci:** add tests for epoch-based token distribution for evonodes ([#2626](https://github.com/dashpay/platform/issues/2626))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.0.0-rc.12 ([#2637](https://github.com/dashpay/platform/issues/2637))

## [2.0.0-rc.11](https://github.com/dashpay/platform/compare/v2.0.0-rc.10...v2.0.0-rc.11) (2025-05-19)


### Features

* **platform:** don't allow freezing non existent identity ([#2612](https://github.com/dashpay/platform/issues/2612))
* **sdk:** token purchase and set price transitions ([#2613](https://github.com/dashpay/platform/issues/2613))


### Bug Fixes

* **dpp:** historical document type name for token direct pricing ([#2616](https://github.com/dashpay/platform/issues/2616))
* **drive:** resolve deserialization issue in check_tx for group actions ([#2619](https://github.com/dashpay/platform/issues/2619))
* **drive:** verification of token purchase can not verify the purchase cost as this can be lower than the agreed price ([#2617](https://github.com/dashpay/platform/issues/2617))
* **platform:** consensus error for invalid group position, config update won't allow group action if group action is not required, and tests ([#2614](https://github.com/dashpay/platform/issues/2614))
* **platform:** correct burn identity in group actions ([#2615](https://github.com/dashpay/platform/issues/2615))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.0.0-rc.11 ([#2618](https://github.com/dashpay/platform/issues/2618))


### Documentation

* **dashmate:** document services, configuration and cli ([#2532](https://github.com/dashpay/platform/issues/2532))


### Code Refactoring

* **dpp:** import CREDITS_PER_DUFF for use in credit converter ([#2473](https://github.com/dashpay/platform/issues/2473))

## [2.0.0-rc.10](https://github.com/dashpay/platform/compare/v2.0.0-rc.9...v2.0.0-rc.10) (2025-05-13)


### Features

* **platform:** only allow token notes when you are the proposer and use proposer notes in historical documents ([#2609](https://github.com/dashpay/platform/issues/2609))


### Bug Fixes

* **drive:** fix group action query target as tree in stateless execution ([#2608](https://github.com/dashpay/platform/issues/2608))
* **platform:** ensure group action parameters cannot be modified ([#2610](https://github.com/dashpay/platform/issues/2610))
* update package versions to 2.0.0-rc.10 ([#2611](https://github.com/dashpay/platform/issues/2611))


### Code Refactoring

* **sdk:** set group info for config update transition ([#2603](https://github.com/dashpay/platform/issues/2603))

## [2.0.0-rc.9](https://github.com/dashpay/platform/compare/v2.0.0-rc.8...v2.0.0-rc.9) (2025-05-12)


### Features

* **platform:** group action proofs ([#2605](https://github.com/dashpay/platform/issues/2605))


### Documentation

* **dapi:** document architecture and implementation ([#2539](https://github.com/dashpay/platform/issues/2539))


### Miscellaneous Chores

* update package versions to 2.0.0-rc.9 ([#2606](https://github.com/dashpay/platform/issues/2606))

## [2.0.0-rc.8](https://github.com/dashpay/platform/compare/v2.0.0-rc.7...v2.0.0-rc.8) (2025-05-08)


### Miscellaneous Chores

* integrate keyword-search-contract into yarn lock ([#2601](https://github.com/dashpay/platform/issues/2601))
* update dependencies to version 2.0.0-rc.8 ([#2602](https://github.com/dashpay/platform/issues/2602))

## [2.0.0-rc.7](https://github.com/dashpay/platform/compare/v2.0.0-rc.6...v2.0.0-rc.7) (2025-05-08)


### ⚠ BREAKING CHANGES

* **platform:** ensure correct critical security level for token transitions and allow any security level key in signing if allowed to do so with options (#2597)

### Bug Fixes

* **platform:** ensure correct critical security level for token transitions and allow any security level key in signing if allowed to do so with options ([#2597](https://github.com/dashpay/platform/issues/2597))


### Code Refactoring

* rename search-contract to token-search-contract ([#2598](https://github.com/dashpay/platform/issues/2598))
* rename token-search-contract to keyword-search-contract ([#2599](https://github.com/dashpay/platform/issues/2599))


### Miscellaneous Chores

* update dependencies to 2.0.0-rc-7 ([#2600](https://github.com/dashpay/platform/issues/2600))

## [2.0.0-rc.6](https://github.com/dashpay/platform/compare/v2.0.0-rc.5...v2.0.0-rc.6) (2025-05-07)


### Features

* **drive-abci:** improve token name localization validation ([#2593](https://github.com/dashpay/platform/issues/2593))


### Bug Fixes

* **dpp:** handle MainGroupIsNotDefinedError in token configuration ([#2594](https://github.com/dashpay/platform/issues/2594))
* **dpp:** validate group required power to prevent invalid configurations ([#2595](https://github.com/dashpay/platform/issues/2595))
* **drive:** add estimation costs for token status information when registering a contract ([#2591](https://github.com/dashpay/platform/issues/2591))
* **token-history-contract:** public note proof verification for token history contract ([#2590](https://github.com/dashpay/platform/issues/2590))


### Miscellaneous Chores

* add .gitaipconfig to .gitignore ([#2592](https://github.com/dashpay/platform/issues/2592))
* bump to 2.0.0-rc.6 ([#2596](https://github.com/dashpay/platform/issues/2596))

## [2.0.0-rc.5](https://github.com/dashpay/platform/compare/v2.0.0-rc.4...v2.0.0-rc.5) (2025-05-05)


### ⚠ BREAKING CHANGES

* **platform:** fees for data contract creation and update (#2584)

### Features

* **platform:** fees for data contract creation and update ([#2584](https://github.com/dashpay/platform/issues/2584))


### Bug Fixes

* **dpp:** do not allow mint of 0 tokens ([#2581](https://github.com/dashpay/platform/issues/2581))
* **drive-abci:** make sure all group identities exist ([#2585](https://github.com/dashpay/platform/issues/2585))
* **drive-abci:** make sure identities in token config exist ([#2583](https://github.com/dashpay/platform/issues/2583))
* **platform:** force allow choosing minting destination if no default minting destination recipient ([#2586](https://github.com/dashpay/platform/issues/2586))
* **platform:** start as paused was not working ([#2582](https://github.com/dashpay/platform/issues/2582))
* **sdk:** compare underlying returned data on proof verification ([#2580](https://github.com/dashpay/platform/issues/2580))


### Miscellaneous Chores

* update to rc.5 ([#2587](https://github.com/dashpay/platform/issues/2587))

## [2.0.0-rc.4](https://github.com/dashpay/platform/compare/v2.0.0-rc.3...v2.0.0-rc.4) (2025-04-28)


### Bug Fixes

* **platform:** document serialization v1 to fix serialization and deserialization of integers ([#2578](https://github.com/dashpay/platform/issues/2578))
* **platform:** set recipient ID in token history claim document to being required ([#2577](https://github.com/dashpay/platform/issues/2577))


### Documentation

* **dpp:** add better documentation for token configuration ([#2574](https://github.com/dashpay/platform/issues/2574))


### Code Refactoring

* **dpp:** remove document serialize consume ([#2575](https://github.com/dashpay/platform/issues/2575))
* **platform:** various improvements for proof debugging ([#2576](https://github.com/dashpay/platform/issues/2576))


### Build System

* update dependencies with known security issues ([#2572](https://github.com/dashpay/platform/issues/2572))
* update javascript elliptic lib to 6.6.1 ([#2573](https://github.com/dashpay/platform/issues/2573))


### Miscellaneous Chores

* bump to v2.0.0-rc.4 ([#2579](https://github.com/dashpay/platform/issues/2579))
* removed Ivan Shumkov as code owner at his request ([#2571](https://github.com/dashpay/platform/issues/2571))

## [2.0.0-rc.3](https://github.com/dashpay/platform/compare/v2.0.0-rc.2...v2.0.0-rc.3) (2025-04-24)


### Features

* **drive:** get token config using provider ([#2567](https://github.com/dashpay/platform/issues/2567))
* **sdk:** fetch last distribution claim moment ([#2566](https://github.com/dashpay/platform/issues/2566))


### Bug Fixes

* **drive:** allow getting tree sum value from element directly ([#2570](https://github.com/dashpay/platform/issues/2570))
* **drive:** limit in perpetual_distribution_last_paid_moment_query ([#2569](https://github.com/dashpay/platform/issues/2569))


### Miscellaneous Chores

* **platform:** enable bls-signatures by default and upgrade to rc-3 ([#2568](https://github.com/dashpay/platform/issues/2568))

## [2.0.0-rc.2](https://github.com/dashpay/platform/compare/v2.0.0-rc.1...v2.0.0-rc.2) (2025-04-21)


### Features

* **dpp:** token configuration presets ([#2561](https://github.com/dashpay/platform/issues/2561))
* **drive-abci:** do not allow old state transitions to be processed before fork ([#2564](https://github.com/dashpay/platform/issues/2564))
* **platform:** token last claim query ([#2559](https://github.com/dashpay/platform/issues/2559))


### Bug Fixes

* **dapi-grpc:** add GetTokenPerpetualDistributionLastClaim to versioned requests ([#2563](https://github.com/dashpay/platform/issues/2563))
* **drive:** add path query back to GroveDBError ([#2555](https://github.com/dashpay/platform/issues/2555))
* **token-history-contract:** fixed claim indexes ([#2562](https://github.com/dashpay/platform/issues/2562))


### Miscellaneous Chores

* update to 2.0.0-rc.2 ([#2565](https://github.com/dashpay/platform/issues/2565))

## [2.0.0-rc.1](https://github.com/dashpay/platform/compare/v2.0.0-dev.1...v2.0.0-rc.1) (2025-04-18)


### ⚠ BREAKING CHANGES

* **platform:** token payment info (#2517)

### Features

* **platform:** allow new tokens on contract update and refactor contract struct validations ([#2542](https://github.com/dashpay/platform/issues/2542))
* **platform:** direct selling of tokens to users ([#2534](https://github.com/dashpay/platform/issues/2534))
* **platform:** get identities by non-unique public key hashes ([#2507](https://github.com/dashpay/platform/issues/2507))
* **platform:** keyword search system contract ([#2523](https://github.com/dashpay/platform/issues/2523))
* **platform:** require token for document actions ([#2498](https://github.com/dashpay/platform/issues/2498))
* **platform:** token payment info ([#2517](https://github.com/dashpay/platform/issues/2517))
* **platform:** transfer to frozen account is allowed ([#2478](https://github.com/dashpay/platform/issues/2478))
* **sdk:** fetch defined token direct purchase prices ([#2544](https://github.com/dashpay/platform/issues/2544))
* **sdk:** token claim state transition ([#2522](https://github.com/dashpay/platform/issues/2522))
* **sdk:** token config update transition ([#2554](https://github.com/dashpay/platform/issues/2554))


### Bug Fixes

* **dapi:** invalid proof for destroy frozen funds transition ([#2513](https://github.com/dashpay/platform/issues/2513))
* data contract proof doesn't work  with new auto fields ([#2501](https://github.com/dashpay/platform/issues/2501))
* **dpp:** decoding invalid consensus error variants ([#2510](https://github.com/dashpay/platform/issues/2510))
* **dpp:** missing closing bracket in validate contract update ([#2541](https://github.com/dashpay/platform/issues/2541))
* **drive-abci:** data contract create transition advanced structure version ([#2543](https://github.com/dashpay/platform/issues/2543))
* group member power validation ([#2520](https://github.com/dashpay/platform/issues/2520))
* **platform:** unique token keeps history documents ([#2506](https://github.com/dashpay/platform/issues/2506))
* **sdk:** make some things public ([#2496](https://github.com/dashpay/platform/issues/2496))
* **sdk:** no process-level CryptoProvider available ([#2546](https://github.com/dashpay/platform/issues/2546))
* token distribution timestamp in the past ([#2509](https://github.com/dashpay/platform/issues/2509))
* token transfer to non-existing identity ([#2505](https://github.com/dashpay/platform/issues/2505))


### Build System

* bump wasm-bindgen to 0.2.100 to satisfy js-sys deps ([#2503](https://github.com/dashpay/platform/issues/2503))
* enforce bincode version 2.0.0-rc3 ([#2504](https://github.com/dashpay/platform/issues/2504))


### Code Refactoring

* fix clippy warnings ([#2515](https://github.com/dashpay/platform/issues/2515))
* move proof retrieval from DAPI to Drive ABCI ([#2535](https://github.com/dashpay/platform/issues/2535))


### Tests

* **dpp:** add test for group with all unilateral members ([#2514](https://github.com/dashpay/platform/issues/2514))
* **drive:** test various token distribution algorithms ([#2511](https://github.com/dashpay/platform/issues/2511))
* **platform:** Add data contract basic validation of distributions
* **platform:** distribution inverted log tests ([#2547](https://github.com/dashpay/platform/issues/2547))
* **platform:** distribution log tests ([#2548](https://github.com/dashpay/platform/issues/2548))
* **platform:** tests for exp and polynomial distributions ([#2556](https://github.com/dashpay/platform/issues/2556))
* **platform:** token distribution step decreasing tests and improvements ([#2545](https://github.com/dashpay/platform/issues/2545))


### Miscellaneous Chores

* **platform:** bump rust dashcore version to 0.39.6 ([#2553](https://github.com/dashpay/platform/issues/2553))
* **platform:** bump rust-dashcore version ([#2549](https://github.com/dashpay/platform/issues/2549))
* update to 2.0.0 rc-1 ([#2557](https://github.com/dashpay/platform/issues/2557))
* update tonic to version 0.13 ([#2540](https://github.com/dashpay/platform/issues/2540))

## [2.0.0-dev.1](https://github.com/dashpay/platform/compare/v1.8.0...v2.0.0-dev.1) (2025-03-13)


### ⚠ BREAKING CHANGES

* **platform:** token distribution fixes and tests (#2494)
* **platform:** token advanced distribution and updates (#2471)
* **sdk:** bigint for uint64 values (#2443)
* **platform:** enhance token configuration and validation mechanisms (#2439)
* **platform:** improved token validation and token config update transition (#2435)
* **dpp:** wrapping overflow issue (#2430)
* **platform:** token base support (#2383)
* optimize for x86-64-v3 cpu microarchitecture (Haswell+) (#2374)

### Features

* add token transitions to SDK and DAPI ([#2434](https://github.com/dashpay/platform/issues/2434))
* check if token is paused on token transfers
* **dpp:** extra methods for state transitions in wasm ([#2401](https://github.com/dashpay/platform/issues/2401))
* **dpp:** extra methods for state transitions in wasm ([#2462](https://github.com/dashpay/platform/issues/2462))
* **dpp:** token distribution model ([#2447](https://github.com/dashpay/platform/issues/2447))
* get proofs for tokens ([#2433](https://github.com/dashpay/platform/issues/2433))
* group queries ([#2432](https://github.com/dashpay/platform/issues/2432))
* **js-dash-sdk:** fix tests after merge
* more granular integer document property types ([#2455](https://github.com/dashpay/platform/issues/2455))
* **platform:** enhance token configuration and validation mechanisms ([#2439](https://github.com/dashpay/platform/issues/2439))
* **platform:** improved token validation and token config update transition ([#2435](https://github.com/dashpay/platform/issues/2435))
* **platform:** proof verification for many queries and a few more queries ([#2431](https://github.com/dashpay/platform/issues/2431))
* **platform:** token advanced distribution and updates ([#2471](https://github.com/dashpay/platform/issues/2471))
* **platform:** token base support ([#2383](https://github.com/dashpay/platform/issues/2383))
* **platform:** token distribution part two ([#2450](https://github.com/dashpay/platform/issues/2450))
* **sdk:** add option to request all keys ([#2445](https://github.com/dashpay/platform/issues/2445))
* **sdk:** return state transition execution error ([#2454](https://github.com/dashpay/platform/issues/2454))
* **sdk:** token and group queries ([#2449](https://github.com/dashpay/platform/issues/2449))
* validate token name localizations ([#2468](https://github.com/dashpay/platform/issues/2468))
* wasm sdk build proof-of-concept ([#2405](https://github.com/dashpay/platform/issues/2405))


### Bug Fixes

* destroy frozen funds used wrong identity and proof verification ([#2467](https://github.com/dashpay/platform/issues/2467))
* **dpp:** invalid feature flag instructions ([#2448](https://github.com/dashpay/platform/issues/2448))
* **dpp:** invalid feature flag usage ([#2477](https://github.com/dashpay/platform/issues/2477))
* **dpp:** invalid imports and tests ([#2459](https://github.com/dashpay/platform/issues/2459))
* **dpp:** wrapping overflow issue ([#2430](https://github.com/dashpay/platform/issues/2430))
* **drive:** using new rust dash core methods for reversed quorum hash to maintain backwards compatibility ([#2489](https://github.com/dashpay/platform/issues/2489))
* **platform:** token distribution fixes and tests ([#2494](https://github.com/dashpay/platform/issues/2494))
* proof result error for credit transfers in sdk ([#2451](https://github.com/dashpay/platform/issues/2451))
* **sdk:** bigint for uint64 values ([#2443](https://github.com/dashpay/platform/issues/2443))
* token already paused unpaused and frozen validation ([#2466](https://github.com/dashpay/platform/issues/2466))
* token history contract ([#2474](https://github.com/dashpay/platform/issues/2474))
* wrong order of parameters in UnauthorizedTokenActionError
* xss vulnerability in mocha ([#2469](https://github.com/dashpay/platform/issues/2469))


### Continuous Integration

* use github-hosted arm runner for release workflow ([#2452](https://github.com/dashpay/platform/issues/2452))


### Build System

* bump Alpine version to 3.21 ([#2074](https://github.com/dashpay/platform/issues/2074))
* bump rust version to 1.85 ([#2480](https://github.com/dashpay/platform/issues/2480))
* optimize for x86-64-v3 cpu microarchitecture (Haswell+) ([#2374](https://github.com/dashpay/platform/issues/2374))


### Tests

* **dpp:** fix assertion with the same value
* fix `fetchProofForStateTransition` tests and warnings ([#2460](https://github.com/dashpay/platform/issues/2460))
* fix slowdown of JS SDK unit tests ([#2475](https://github.com/dashpay/platform/issues/2475))
* fix token history contract tests ([#2470](https://github.com/dashpay/platform/issues/2470))


### Documentation

* update comment for data contract code range ([#2476](https://github.com/dashpay/platform/issues/2476))


### Miscellaneous Chores

* dapi grpc queries ([#2437](https://github.com/dashpay/platform/issues/2437))
* **dpp:** remove unnecessary type conversion
* ignore deprecated `lodash.get` ([#2441](https://github.com/dashpay/platform/issues/2441))
* **platform:** bump to version 2.0.0-dev.1 ([#2495](https://github.com/dashpay/platform/issues/2495))
* **platform:** make bls sig compatibility an optional feature ([#2440](https://github.com/dashpay/platform/issues/2440))
* **platform:** npm audit fix ([#2463](https://github.com/dashpay/platform/issues/2463))
* remove duplicated commented code
* update to latest dash core 37 ([#2483](https://github.com/dashpay/platform/issues/2483))

## [2.0.0-rc.18](https://github.com/dashpay/platform/compare/v2.0.0-rc.16...v2.0.0-rc.18) (2025-06-24)


### ⚠ BREAKING CHANGES

* **platform:** properly use withdrawal system data contract. (#2675)

### Features

* **dpp:** add detailed interval evaluation explanation functionality ([#2662](https://github.com/dashpay/platform/issues/2662))
* replace diskusage with diskusage-ng for improved functionality ([#2680](https://github.com/dashpay/platform/issues/2680))
* **sdk:** fetch token contract info ([#2670](https://github.com/dashpay/platform/issues/2670))


### Bug Fixes

* **drive-abci:** fixed issue with adding a key with contract bounds ([#2673](https://github.com/dashpay/platform/issues/2673))
* **platform:** properly use withdrawal system data contract. ([#2675](https://github.com/dashpay/platform/issues/2675))


### Continuous Integration

* add gRPC coverage check and cache management ([#2667](https://github.com/dashpay/platform/issues/2667))


### Miscellaneous Chores

* **platform:** remove dash devs discord from readme ([#2668](https://github.com/dashpay/platform/issues/2668))
* **release:** update changelog and bump version to 2.0.0-rc.17 ([#2674](https://github.com/dashpay/platform/issues/2674))
* remove unused token meta schema and references ([#2677](https://github.com/dashpay/platform/issues/2677))
* update js dependencies to latest versions ([#2678](https://github.com/dashpay/platform/issues/2678))
* update minimatch to version 9.0.5 and brace-expansion to version 2.0.2 ([#2672](https://github.com/dashpay/platform/issues/2672))

## [2.0.0-rc.17](https://github.com/dashpay/platform/compare/v2.0.0-rc.16...v2.0.0-rc.17) (2025-06-18)


### Features

* **dpp:** add detailed interval evaluation explanation functionality ([#2662](https://github.com/dashpay/platform/issues/2662))
* **sdk:** fetch token contract info ([#2670](https://github.com/dashpay/platform/issues/2670))


### Bug Fixes

* **drive-abci:** fixed issue with adding a key with contract bounds ([#2673](https://github.com/dashpay/platform/issues/2673))


### Continuous Integration

* add gRPC coverage check and cache management ([#2667](https://github.com/dashpay/platform/issues/2667))


### Miscellaneous Chores

* **platform:** remove dash devs discord from readme ([#2668](https://github.com/dashpay/platform/issues/2668))
* update minimatch to version 9.0.5 and brace-expansion to version 2.0.2 ([#2672](https://github.com/dashpay/platform/issues/2672))

## [2.0.0-rc.16](https://github.com/dashpay/platform/compare/v2.0.0-rc.14...v2.0.0-rc.16) (2025-06-10)


### ⚠ BREAKING CHANGES

* **sdk:** make document state transition entropy optional, will do a replace if revision is not 1. (#2658)

### Features

* **dpp:** export more token transition fields ([#2655](https://github.com/dashpay/platform/issues/2655))
* **platform:** add finalized epoch infos query and proof functionality ([#2665](https://github.com/dashpay/platform/issues/2665))
* **sdk:** add sdk wrappers for easily pushing document transitions for create, delete, purchase, replace, set price, and transfer to platform ([#2659](https://github.com/dashpay/platform/issues/2659))
* **sdk:** add token state transition functionalities to rs-sdk ([#2657](https://github.com/dashpay/platform/issues/2657))
* **sdk:** make document state transition entropy optional, will do a replace if revision is not 1. ([#2658](https://github.com/dashpay/platform/issues/2658))


### Bug Fixes

* fixes issue [#2653](https://github.com/dashpay/platform/issues/2653) Cannot decode DataContractCreateV1 with WASM-DPP ([#2654](https://github.com/dashpay/platform/issues/2654))
* **platform:** npm audit security for tar-fs ([#2656](https://github.com/dashpay/platform/issues/2656))
* **platform:** resolve direct purchase from self issue causing chain stall ([#2663](https://github.com/dashpay/platform/issues/2663))


### Documentation

* add CLAUDE.md for development guidance ([#2652](https://github.com/dashpay/platform/issues/2652))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.0.0-rc.15 ([#2664](https://github.com/dashpay/platform/issues/2664))

## [2.0.0-rc.15](https://github.com/dashpay/platform/compare/v2.0.0-rc.14...v2.0.0-rc.15) (2025-06-08)


### ⚠ BREAKING CHANGES

* **sdk:** make document state transition entropy optional, will do a replace if revision is not 1. (#2658)

### Features

* **dpp:** export more token transition fields ([#2655](https://github.com/dashpay/platform/issues/2655))
* **sdk:** add sdk wrappers for easily pushing document transitions for create, delete, purchase, replace, set price, and transfer to platform ([#2659](https://github.com/dashpay/platform/issues/2659))
* **sdk:** add token state transition functionalities to rs-sdk ([#2657](https://github.com/dashpay/platform/issues/2657))
* **sdk:** make document state transition entropy optional, will do a replace if revision is not 1. ([#2658](https://github.com/dashpay/platform/issues/2658))


### Bug Fixes

* fixes issue [#2653](https://github.com/dashpay/platform/issues/2653) Cannot decode DataContractCreateV1 with WASM-DPP ([#2654](https://github.com/dashpay/platform/issues/2654))
* **platform:** npm audit security for tar-fs ([#2656](https://github.com/dashpay/platform/issues/2656))
* **platform:** resolve direct purchase from self issue causing chain stall ([#2663](https://github.com/dashpay/platform/issues/2663))


### Documentation

* add CLAUDE.md for development guidance ([#2652](https://github.com/dashpay/platform/issues/2652))

## [2.0.0-rc.14](https://github.com/dashpay/platform/compare/v2.0.0-rc.12...v2.0.0-rc.14) (2025-05-29)


### ⚠ BREAKING CHANGES

* **dpp:** set minimum intervals for perpetual distribution (#2622)
* **platform:** load data contracts in their respective versions (#2644)

### Features

* add checks for authorized action takers in data contract create and update validations ([#2647](https://github.com/dashpay/platform/issues/2647))
* **dpp:** add validation for minimum group member count ([#2646](https://github.com/dashpay/platform/issues/2646))
* **dpp:** set minimum intervals for perpetual distribution ([#2622](https://github.com/dashpay/platform/issues/2622))
* **platform:** add token contract info and query ([#2641](https://github.com/dashpay/platform/issues/2641))
* **sdk:** add DataContractMismatch enum for detailed contract comparison ([#2648](https://github.com/dashpay/platform/issues/2648))


### Bug Fixes

* **dpp:** unclear error message for missing document types if no tokens defined ([#2639](https://github.com/dashpay/platform/issues/2639))
* **drive:** proved identity update was giving error ([#2642](https://github.com/dashpay/platform/issues/2642))
* **platform:** load data contracts in their respective versions ([#2644](https://github.com/dashpay/platform/issues/2644))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.0.0-rc.13 ([#2645](https://github.com/dashpay/platform/issues/2645))


### Tests

* **drive:** add test for invalid owner on document delete ([#2643](https://github.com/dashpay/platform/issues/2643))

## [2.0.0-rc.13](https://github.com/dashpay/platform/compare/v2.0.0-rc.12...v2.0.0-rc.13) (2025-05-28)


### ⚠ BREAKING CHANGES

* **platform:** load data contracts in their respective versions (#2644)

### Features

* **platform:** add token contract info and query ([#2641](https://github.com/dashpay/platform/issues/2641))


### Bug Fixes

* **dpp:** unclear error message for missing document types if no tokens defined ([#2639](https://github.com/dashpay/platform/issues/2639))
* **drive:** proved identity update was giving error ([#2642](https://github.com/dashpay/platform/issues/2642))
* **platform:** load data contracts in their respective versions ([#2644](https://github.com/dashpay/platform/issues/2644))

## [2.0.0-rc.12](https://github.com/dashpay/platform/compare/v2.0.0-rc.11...v2.0.0-rc.12) (2025-05-26)


### ⚠ BREAKING CHANGES

* **dpp:** disable changes to perpetual distribution in token configuration (#2627)

### Features

* **dpp:** add marketplace rules to token configuration ([#2635](https://github.com/dashpay/platform/issues/2635))
* **dpp:** disable changes to perpetual distribution in token configuration ([#2627](https://github.com/dashpay/platform/issues/2627))
* **sdk:** add token payment info to put_document ([#2630](https://github.com/dashpay/platform/issues/2630))


### Bug Fixes

* **dashmate:** sync max-tx-bytes between tenderdash and drive ([#2625](https://github.com/dashpay/platform/issues/2625))
* **dpp:** allow changing main control group for token configuration ([#2628](https://github.com/dashpay/platform/issues/2628))
* **dpp:** correct stepwise distribution logic in evaluate.rs ([#2636](https://github.com/dashpay/platform/issues/2636))
* **dpp:** missing tags on Groups needed for deserialization ([#2624](https://github.com/dashpay/platform/issues/2624))
* **drive:** ignore time based update fields in proof verification of data contract updates ([#2634](https://github.com/dashpay/platform/issues/2634))
* **platform:** ensure document types only target valid tokens for token payments ([#2631](https://github.com/dashpay/platform/issues/2631))
* **platform:** fix evonode distribution for token perpetual distribution (part 1) ([#2623](https://github.com/dashpay/platform/issues/2623))
* **platform:** paying for a document action with tokens where tokens would be transferred to yourself as contract owner was breaking ([#2633](https://github.com/dashpay/platform/issues/2633))
* **platform:** resolved grovedb error during signing group action finalization on check tx ([#2629](https://github.com/dashpay/platform/issues/2629))


### Tests

* **drive-abci:** add tests for epoch-based token distribution for evonodes ([#2626](https://github.com/dashpay/platform/issues/2626))

## [2.0.0-rc.11](https://github.com/dashpay/platform/compare/v2.0.0-rc.10...v2.0.0-rc.11) (2025-05-19)


### Features

* **platform:** don't allow freezing non existent identity ([#2612](https://github.com/dashpay/platform/issues/2612))
* **sdk:** token purchase and set price transitions ([#2613](https://github.com/dashpay/platform/issues/2613))


### Bug Fixes

* **dpp:** historical document type name for token direct pricing ([#2616](https://github.com/dashpay/platform/issues/2616))
* **drive:** verification of token purchase can not verify the purchase cost as this can be lower than the agreed price ([#2617](https://github.com/dashpay/platform/issues/2617))
* **platform:** consensus error for invalid group position, config update won't allow group action if group action is not required, and tests ([#2614](https://github.com/dashpay/platform/issues/2614))

## [1.8.0](https://github.com/dashpay/platform/compare/v1.7.1...v1.8.0) (2025-01-16)


### Features

* **platform:** distribute prefunded specialized balances after vote ([#2422](https://github.com/dashpay/platform/issues/2422))
* **platform:** using new rust based bls library ([#2424](https://github.com/dashpay/platform/issues/2424))


### Bug Fixes

* **drive-abci:** document purchase on mutable document from different epoch had issue ([#2420](https://github.com/dashpay/platform/issues/2420))
* **drive:** more than one key was returned when expecting only one result ([#2421](https://github.com/dashpay/platform/issues/2421))
* **sdk:** failed to deserialize consensus error ([#2410](https://github.com/dashpay/platform/issues/2410))
* try DriveDocumentQuery from DocumentQuery start field ([#2407](https://github.com/dashpay/platform/issues/2407))
* **drive-abci** rebroadcasting should not only take first 2 quorums too ([#2425](https://github.com/dashpay/platform/issues/2425))
* **dashmate:** local network starting issues ([#2394](https://github.com/dashpay/platform/issues/2394))
* **dashmate:** some group commands fail with mtime not found ([#2400](https://github.com/dashpay/platform/issues/2400))
* emergency hard fork to fix masternode voting ([#2397](https://github.com/dashpay/platform/issues/2397))


### Tests

* unify identity versioned cost coverage ([#2416](https://github.com/dashpay/platform/issues/2416))
* **sdk:** generate test vectors using testnet ([#2381](https://github.com/dashpay/platform/issues/2381))


### Miscellaneous Chores

* **drive:** increase withdrawal limits to 2000 Dash per day ([#2287](https://github.com/dashpay/platform/issues/2287))
* fix test suite configuration script ([#2402](https://github.com/dashpay/platform/issues/2402))
* resolve NPM audit warnings ([#2417](https://github.com/dashpay/platform/issues/2417))
* remove deprecated check_network_version.sh ([#2084](https://github.com/dashpay/platform/issues/2084))
* update bls library ([#2424](https://github.com/dashpay/platform/issues/2424))


### Code Refactoring

* **platform:** replace bls library ([#2257](https://github.com/dashpay/platform/issues/2257))
* **dpp:** using deprecated param to init wasm module ([#2399](https://github.com/dashpay/platform/issues/2399))


### Performance Improvements

* **dpp:** reduce JS binding size by 3x ([#2396](https://github.com/dashpay/platform/issues/2396))


### Continuous Integration

* fix artifact upload issue on release build ([#2389](https://github.com/dashpay/platform/issues/2389))


### Build System

* bump wasm-bindgen to 0.2.99 ([#2395](https://github.com/dashpay/platform/issues/2395))
* update rust to 1.83 ([#2393](https://github.com/dashpay/platform/issues/2393))



## [1.8.0-rc.1](https://github.com/dashpay/platform/compare/v1.8.0-dev.2...v1.8.0-rc.1) (2025-01-15)


### Features

* **platform:** distribute prefunded specialized balances after vote ([#2422](https://github.com/dashpay/platform/issues/2422))


### Bug Fixes

* **drive-abci:** document purchase on mutable document from different epoch had issue ([#2420](https://github.com/dashpay/platform/issues/2420))
* **drive:** more than one key was returned when expecting only one result ([#2421](https://github.com/dashpay/platform/issues/2421))
* **sdk:** failed to deserialize consensus error ([#2410](https://github.com/dashpay/platform/issues/2410))
* try DriveDocumentQuery from DocumentQuery start field ([#2407](https://github.com/dashpay/platform/issues/2407))


### Tests

* unify identity versioned cost coverage ([#2416](https://github.com/dashpay/platform/issues/2416))


### Miscellaneous Chores

* **drive:** increase withdrawal limits to 2000 Dash per day ([#2287](https://github.com/dashpay/platform/issues/2287))
* fix test suite configuration script ([#2402](https://github.com/dashpay/platform/issues/2402))
* resolve NPM audit warnings ([#2417](https://github.com/dashpay/platform/issues/2417))
* update bls library ([#2424](https://github.com/dashpay/platform/issues/2424))



## [1.8.0-dev.2](https://github.com/dashpay/platform/compare/v1.8.0-dev.1...v1.8.0-dev.2) (2024-12-19)

### ⚠ BREAKING CHANGES

* On epoch 13, masternode vote state transition validation logic will be changed. Nodes which aren't upgraded to this version will halt (#2397)
* Rust 1.83 is required to build the project (#2398)

### Bug Fixes

* **dashmate:** local network starting issues ([#2394](https://github.com/dashpay/platform/issues/2394))
* **dashmate:** some group commands fail with mtime not found ([#2400](https://github.com/dashpay/platform/issues/2400))
* emergency hard fork to fix masternode voting ([#2397](https://github.com/dashpay/platform/issues/2397))


### Performance Improvements

* **dpp:** reduce JS binding size by 3x ([#2396](https://github.com/dashpay/platform/issues/2396))


### Build System

* bump wasm-bindgen to 0.2.99 ([#2395](https://github.com/dashpay/platform/issues/2395))
* update rust to 1.83 ([#2393](https://github.com/dashpay/platform/issues/2393))


### Code Refactoring

* **dpp:** using deprecated param to init wasm module ([#2399](https://github.com/dashpay/platform/issues/2399))


### [1.7.1](https://github.com/dashpay/platform/compare/v1.7.0...v1.7.1) (2024-12-19)

### ⚠ BREAKING CHANGES

* On epoch 13, masternode vote state transition validation logic will be changed. Nodes which aren't upgraded to this version will halt (#2397)
* Rust 1.83 is required to build the project (#2398)

### Bug Fixes

* emergency hard fork to fix masternode voting ([#2397](https://github.com/dashpay/platform/issues/2397))


### Build System

* update rust to 1.83 - backport [#2393](https://github.com/dashpay/platform/issues/2393) to v1.7 ([#2398](https://github.com/dashpay/platform/issues/2398))


## [1.8.0-dev.1](https://github.com/dashpay/platform/compare/v1.7.0...v1.8.0-dev.1) (2024-12-16)

### Continuous Integration

* fix artifact upload issue on release build ([#2389](https://github.com/dashpay/platform/issues/2389))


### Miscellaneous Chores

* remove deprecated check_network_version.sh ([#2084](https://github.com/dashpay/platform/issues/2084))


### Tests

* **sdk:** generate test vectors using testnet ([#2381](https://github.com/dashpay/platform/issues/2381))


### Code Refactoring

* **platform:** replace bls library ([#2257](https://github.com/dashpay/platform/issues/2257))


### [1.7.0](https://github.com/dashpay/platform/compare/v1.6.2...v1.7.0) (2024-12-13)


### ⚠ BREAKING CHANGES

* **sdk:** `AddressList.available` removed
* **sdk:** you need to use `Waitable` to call `wait_for_response()`
* **sdk:** changed multiple args of functions in state transition broadcast logic
* **sdk:** `From<Uri> for Address` replaced with `TryFrom<Uri> for Address`
* **sdk:** `From<&str> for AddressList` replaced with `FromStr for AddressList`
* **sdk:** `FromIterator<Uri> for AddressList` replaced with `FromIterator<Address> for AddressList`
* **sdk:** `LowLevelDashCoreClient` now returns `DashCoreError` instead of `ContextProviderError`.
* **sdk:** Added `ContextProviderError::DashCoreError` variant
* **sdk:** `dash_sdk::Error::CoreClientError` now uses `DashCoreError` instead of `dashcore_rpc::Error`.

### Features

* **sdk:** ban addresses failed in sdk ([#2351](https://github.com/dashpay/platform/issues/2351))


### Bug Fixes

* **drive:** security vulnerability in hashbrown ([#2375](https://github.com/dashpay/platform/issues/2375))
* **sdk:** create channel error due to empty address ([#2317](https://github.com/dashpay/platform/issues/2317))


### Build System

* explicitly require tonic v1.12.3 ([#2357](https://github.com/dashpay/platform/issues/2357))


### Continuous Integration

* remove manage runs workflow ([#2325](https://github.com/dashpay/platform/issues/2325))
* replace actions/upload-artifact@v3 with actions/upload-artifact@v4 ([#2359](https://github.com/dashpay/platform/issues/2359))


### Miscellaneous Chores

* make protocol version 7 ([#2376](https://github.com/dashpay/platform/issues/2376))
* **dashmate:** set tenderdash version to 1 ([#2385](https://github.com/dashpay/platform/issues/2385)) 
* **dashmate:** update Core to version 22 ([#2384](https://github.com/dashpay/platform/issues/2384))
* address vulnerabilty GHSA-mwcw-c2x4-8c55 ([#2382](https://github.com/dashpay/platform/issues/2382))


### Code Refactoring

* **sdk:** unify state transition processing ([#2338](https://github.com/dashpay/platform/issues/2338))
* **sdk:** separate dash core client error

### [1.6.2](https://github.com/dashpay/platform/compare/v1.6.1...v1.6.2) (2024-12-03)


### Bug Fixes

* **dashmate:** status command fails if drive is not running ([#2364](https://github.com/dashpay/platform/issues/2364))
* **dashmate:** zero ssl verification passes without being verified ([#2365](https://github.com/dashpay/platform/issues/2365))


### Miscellaneous Chores

* ignore leveldb deprecation warnings ([#2366](https://github.com/dashpay/platform/issues/2366))


### Build System

* enable drive image for Ivy Bridge CPU ([#2363](https://github.com/dashpay/platform/issues/2363))

### [1.6.1](https://github.com/dashpay/platform/compare/v1.6.0...v1.6.1) (2024-11-29)

## [1.6.0-dev.2](https://github.com/dashpay/platform/compare/v1.6.0-dev.1...v1.6.0-dev.2) (2024-11-27)


### ⚠ BREAKING CHANGES

* **sdk:** retry broadcast operations (#2337)


### Features

* **sdk:** retry broadcast operations ([#2337](https://github.com/dashpay/platform/issues/2337))


### Reverted

* **dashmate:** update Core to version 22


### Continuous Integration

* change s3 cache provider to optimize costs ([#2344](https://github.com/dashpay/platform/issues/2344))

## [1.6.0-dev.1](https://github.com/dashpay/platform/compare/v1.5.1...v1.6.0-dev.1) (2024-11-25)


### Features

* integrate wallet contract ([#2345](https://github.com/dashpay/platform/issues/2345))
* **sdk:** fetch many and return metadata and proof to client ([#2331](https://github.com/dashpay/platform/issues/2331))
* **sdk:** including grovedb proof bytes when error in proof verification ([#2332](https://github.com/dashpay/platform/issues/2332))


### Bug Fixes

* **dashmate:** container name is already in use ([#2341](https://github.com/dashpay/platform/issues/2341))
* **dashmate:** failing doctor with drive metrics enabled ([#2348](https://github.com/dashpay/platform/issues/2348))
* **dashmate:** various ZeroSSL cert verification errors ([#2339](https://github.com/dashpay/platform/issues/2339))
* document query start after ([#2347](https://github.com/dashpay/platform/issues/2347))
* **drive:** nonce already present in past internal error ([#2343](https://github.com/dashpay/platform/issues/2343))


### Build System

* adjust docker build context ([#2336](https://github.com/dashpay/platform/issues/2336))
* use cargo binstall to speed up builds ([#2321](https://github.com/dashpay/platform/issues/2321))


### Miscellaneous Chores

* **dashmate:** update Core to version 22
* **sdk:** expose proof verifier errors ([#2333](https://github.com/dashpay/platform/issues/2333))
* **sdk:** vote poll queries encoding ([#2334](https://github.com/dashpay/platform/issues/2334))


### Continuous Integration

* improve caching ([#2329](https://github.com/dashpay/platform/issues/2329))
* prebuilt librocksdb in docker image ([#2318](https://github.com/dashpay/platform/issues/2318))
* run devcontainers workflow only on push to master ([#2295](https://github.com/dashpay/platform/issues/2295))
* switch release workflow to github runners ([#2346](https://github.com/dashpay/platform/issues/2346))
* switch test workflow to github runners ([#2319](https://github.com/dashpay/platform/issues/2319))
* use prebuilt librocksdb in github actions ([#2316](https://github.com/dashpay/platform/issues/2316))


### Tests

* hardcoded identity transfers in strategy tests ([#2322](https://github.com/dashpay/platform/issues/2322))


### [1.5.1](https://github.com/dashpay/platform/compare/v1.5.0...v1.5.1) (2024-11-04)

### Bug Fixes

* **drive:** apply batch is not using transaction in `remove_all_votes_given_by_identities` ([#2309](https://github.com/dashpay/platform/issues/2309))
* **drive:** uncommitted state if db transaction fails ([#2305](https://github.com/dashpay/platform/issues/2305))



## [1.5.0](https://github.com/dashpay/platform/compare/v1.4.1...v1.5.0) (2024-11-01)

### ⚠ BREAKING CHANGES

* **drive:** rotate always to top quorum. This is a breaking change requiring a HF. (#2290)
* **sdk:** SDK methods which broadcasting transactions will return `SdkError::Protocol(ProtocolError::Consensus(..))` instead of `DapiClientError(String)` in case of validation errors. (#2274)
* **sdk:** serialized consensus error moved to a separate gRPC header `dash-serialized-consensus-error-bin` (#2274)
* **sdk:** `DapiRequestExecutor::execute` and `DapiRequest::execute` now returns `ExecutionResult` (#2259)
* **sdk:** returned objects are `IndexMap/IndexSet` instead of previous `BTreeMap` (#2207)


### Features

* **dashmate:** add protocol version to the status command ([#2255](https://github.com/dashpay/platform/issues/2255))
* **sdk:** added transfer transition to rs-sdk ([#2289](https://github.com/dashpay/platform/issues/2289))
* **sdk:** detect stale nodes ([#2254](https://github.com/dashpay/platform/issues/2254))
* **sdk:** provide request execution information ([#2259](https://github.com/dashpay/platform/issues/2259))
* **sdk:** return consensus errors from broadcast methods ([#2274](https://github.com/dashpay/platform/issues/2274))
* **sdk:** sdk-level retry logic for `fetch` and `fetch_many` ([#2266](https://github.com/dashpay/platform/issues/2266))
* **dashmate:** cleanup zerossl certs command ([#2298](https://github.com/dashpay/platform/issues/2298))


### Bug Fixes

* **dashmate:** invalid mount path from helper ([#2296](https://github.com/dashpay/platform/issues/2296))
* **dashmate:** zero ssl certificate draft not saved ([#2297](https://github.com/dashpay/platform/issues/2297))
* **platform:** rotate always to top quorum ([#2290](https://github.com/dashpay/platform/issues/2290))
* **dapi:** invalid state transition failed with already in chain error ([#2270](https://github.com/dashpay/platform/issues/2270))
* **dashmate:** invalid drive status check ([#2248](https://github.com/dashpay/platform/issues/2248))
* **dashmate:** invalid platform version in the status command ([#2249](https://github.com/dashpay/platform/issues/2249))
* document query start to support pagination ([#2284](https://github.com/dashpay/platform/issues/2284))
* **sdk:** `AddressListError` is private ([#2278](https://github.com/dashpay/platform/issues/2278))
* **sdk:** opposite retry trigger ([#2265](https://github.com/dashpay/platform/issues/2265))
* **sdk:** wrong order of objects returned by Drive ([#2207](https://github.com/dashpay/platform/issues/2207))
* seed nodes aren't responding ([#2276](https://github.com/dashpay/platform/issues/2276))


### Code Refactoring

* platform version refactoring into sub versions ([#2269](https://github.com/dashpay/platform/issues/2269))


### Miscellaneous Chores

* add partial eq to document query  ([#2253](https://github.com/dashpay/platform/issues/2253))
* **dashmate:** report port check errors ([#2245](https://github.com/dashpay/platform/issues/2245))
* move BLS Sigs import to Rust Dash Core ([#2252](https://github.com/dashpay/platform/issues/2252))
* update to latest rust dash core with x11 optional ([#2251](https://github.com/dashpay/platform/issues/2251))


## [1.5.0-rc.2](https://github.com/dashpay/platform/compare/v1.5.0-rc.1...v1.5.0-rc.2) (2024-10-31)


### ⚠ BREAKING CHANGES

* **platform:** rotate always to top quorum (#2290)

### Bug Fixes

* **dashmate:** cleanup zerossl certs command ([#2298](https://github.com/dashpay/platform/issues/2298))
* **dashmate:** invalid mount path from helper ([#2296](https://github.com/dashpay/platform/issues/2296))
* **dashmate:** zero ssl certificate draft not saved ([#2297](https://github.com/dashpay/platform/issues/2297))
* **platform:** rotate always to top quorum ([#2290](https://github.com/dashpay/platform/issues/2290))


## [1.5.0-rc.1](https://github.com/dashpay/platform/compare/v1.4.1...v1.5.0-rc.1) (2024-10-30)


### ⚠ BREAKING CHANGES

* **sdk:** return consensus errors from broadcast methods (#2274)
* **sdk:** provide request execution information (#2259)
* **sdk:** wrong order of objects returned by Drive (#2207)

### Features

* **dashmate:** add protocol version to the status command ([#2255](https://github.com/dashpay/platform/issues/2255))
* **sdk:** added transfer transition to rs-sdk ([#2289](https://github.com/dashpay/platform/issues/2289))
* **sdk:** detect stale nodes ([#2254](https://github.com/dashpay/platform/issues/2254))
* **sdk:** provide request execution information ([#2259](https://github.com/dashpay/platform/issues/2259))
* **sdk:** return consensus errors from broadcast methods ([#2274](https://github.com/dashpay/platform/issues/2274))
* **sdk:** sdk-level retry logic for `fetch` and `fetch_many` ([#2266](https://github.com/dashpay/platform/issues/2266))


### Bug Fixes

* **dapi:** invalid state transition failed with already in chain error ([#2270](https://github.com/dashpay/platform/issues/2270))
* **dashmate:** invalid drive status check ([#2248](https://github.com/dashpay/platform/issues/2248))
* **dashmate:** invalid platform version in the status command ([#2249](https://github.com/dashpay/platform/issues/2249))
* document query start to support pagination ([#2284](https://github.com/dashpay/platform/issues/2284))
* **sdk:** `AddressListError` is private ([#2278](https://github.com/dashpay/platform/issues/2278))
* **sdk:** opposite retry trigger ([#2265](https://github.com/dashpay/platform/issues/2265))
* **sdk:** wrong order of objects returned by Drive ([#2207](https://github.com/dashpay/platform/issues/2207))
* seed nodes aren't responding ([#2276](https://github.com/dashpay/platform/issues/2276))


### Miscellaneous Chores

* add partial eq to document query  ([#2253](https://github.com/dashpay/platform/issues/2253))
* **dashmate:** report port check errors ([#2245](https://github.com/dashpay/platform/issues/2245))
* move BLS Sigs import to Rust Dash Core ([#2252](https://github.com/dashpay/platform/issues/2252))
* update to latest rust dash core with x11 optional ([#2251](https://github.com/dashpay/platform/issues/2251))


### Code Refactoring

* platform version refactoring into sub versions ([#2269](https://github.com/dashpay/platform/issues/2269))


### [1.4.1](https://github.com/dashpay/platform/compare/v1.4.0...v1.4.1) (2024-10-12)


### ⚠ BREAKING CHANGES

* **sdk:** improve mock context provider async processing (#2232)

### Bug Fixes

* **sdk:** testnet chain sync failed ([#2236](https://github.com/dashpay/platform/issues/2236))


### Miscellaneous Chores

* add some extra unit tests


### Code Refactoring

* minor fixes and extra comments
* **sdk:** improve mock context provider async processing ([#2232](https://github.com/dashpay/platform/issues/2232))

## [1.4.0](https://github.com/dashpay/platform/compare/v1.4.0-dev.8...v1.4.0) (2024-10-10)


### Features

* **dpp:** added identity public key private key validation methods ([#2235](https://github.com/dashpay/platform/issues/2235))
* **sdk:** fix client tls connections ([#2223](https://github.com/dashpay/platform/issues/2223))
* **dpp:** add a convenience method to get the public key data for a private key depending on the key type ([#2214](https://github.com/dashpay/platform/issues/2214))
* **platform:** add owner keys to identities, fixed verification of use of owner keys ([#2215](https://github.com/dashpay/platform/issues/2215))
* **sdk:** enable withdrawals v1 in JS SDK ([#2201](https://github.com/dashpay/platform/issues/2201))
* start network with latest version if genesis version not set ([#2206](https://github.com/dashpay/platform/issues/2206))
* **dashmate:** confirm a node reset ([#2160](https://github.com/dashpay/platform/issues/2160))
* **platform:** do not switch to oldest quorums in validator set update ([#2167](https://github.com/dashpay/platform/issues/2167))
* **platform:** get current quorum info  ([#2168](https://github.com/dashpay/platform/issues/2168))
* **platform:** withdrawals polishing and fixes for mainnet ([#2166](https://github.com/dashpay/platform/issues/2166))
* **sdk:** change default network to mainnet ([#2161](https://github.com/dashpay/platform/issues/2161))


### Bug Fixes

* **sdk:** added signing_withdrawal_key_to_use to withdraw sdk call ([#2234](https://github.com/dashpay/platform/issues/2234))
* **platform:** fixed Platform State deserialization issue ([#2227](https://github.com/dashpay/platform/issues/2227))
* cookie accepts cookie name, path, and domain with out of bounds characters ([#2211](https://github.com/dashpay/platform/issues/2211))
* **drive:** set sign height when rebroadcasting ([#2210](https://github.com/dashpay/platform/issues/2210))
* **sdk:** small sdk improvements and fixes for v1.4 ([#2200](https://github.com/dashpay/platform/issues/2200))
* **drive-abci:** fix network upgrade to version 4 ([#2189](https://github.com/dashpay/platform/issues/2189))
* **dashmate:** collect docker stats in the doctor command ([#2180](https://github.com/dashpay/platform/issues/2180))
* **dashmate:** validate external IP ([#2183](https://github.com/dashpay/platform/issues/2183))
* **platform:** matched withdrawal fees to actual processing cost ([#2186](https://github.com/dashpay/platform/issues/2186))
* **platform:** withdrawal automatic retries after core rejection ([#2185](https://github.com/dashpay/platform/issues/2185))
* **platform:** withdrawal limits ([#2182](https://github.com/dashpay/platform/issues/2182))
* **sdk:** get node status ([#2139](https://github.com/dashpay/platform/issues/2139))
* **dapi:** getStatus cache invalidation ([#2155](https://github.com/dashpay/platform/issues/2155))
* **dapi:** invalid mainnet seed ports ([#2173](https://github.com/dashpay/platform/issues/2173))
* **dashmate:** cannot read properties of undefined (reading 'expires') ([#2164](https://github.com/dashpay/platform/issues/2164))
* **dashmate:** colors[updated] is not a function ([#2157](https://github.com/dashpay/platform/issues/2157))
* **dashmate:** doctor fails collecting to big logs ([#2158](https://github.com/dashpay/platform/issues/2158))
* **dashmate:** port marks as closed if ipv6 is not disabled ([#2162](https://github.com/dashpay/platform/issues/2162))
* **dashmate:** remove confusing short flag name ([#2165](https://github.com/dashpay/platform/issues/2165))


### Miscellaneous Chores

* **dpp:** add method for decoding identifier with unknown string encoding ([#2230](https://github.com/dashpay/platform/issues/2230))
* **drive:** log invalid state on deserialisation ([#2220](https://github.com/dashpay/platform/issues/2220))
* **sdk:** expose drive module in public API for rs-sdk ([#2217](https://github.com/dashpay/platform/issues/2217))
* update dependences ([#2072](https://github.com/dashpay/platform/issues/2072))
* bump GroveDB dependency ([#2196](https://github.com/dashpay/platform/issues/2196))
* **drive:** improve withdrawal logging ([#2203](https://github.com/dashpay/platform/issues/2203))
* **drive:** logs and metrics for withdrawal daily limit ([#2192](https://github.com/dashpay/platform/issues/2192))
* **release:** replace colima with native docker in macOS builds ([#2188](https://github.com/dashpay/platform/issues/2188))
* **dashmate:** do not call mint on masternodes ([#2172](https://github.com/dashpay/platform/issues/2172))
* **platform:** protocol version 4 creation ([#2153](https://github.com/dashpay/platform/issues/2153))


### Code Refactoring

* **sdk:** contested resource as struct type ([#2225](https://github.com/dashpay/platform/issues/2225))
* **drive:** remove duplicated withdrawal amount validation ([#2191](https://github.com/dashpay/platform/issues/2191))


### Build System

* devcontainer support ([#2179](https://github.com/dashpay/platform/issues/2179))


### Continuous Integration

* prebuild dev containers ([#2184](https://github.com/dashpay/platform/issues/2184))
* build dashmate on macos14


### Tests

* **test-suite:** enable withdrawal tests ([#2202](https://github.com/dashpay/platform/issues/2202))
* **dashmate:** e2e tests failing due to DKG interval check ([#2171](https://github.com/dashpay/platform/issues/2171))


### Documentation

* **dashmate:** document logging configuration ([#2156](https://github.com/dashpay/platform/issues/2156))
* update README ([#2219](https://github.com/dashpay/platform/issues/2219))


### ⚠ BREAKING CHANGES

* **platform:** add owner keys to identities, fixed verification of use of owner keys. While these are breaking changes, they will only happen in Protocol V4. (#2215)
* **platform:** matched withdrawal fees to actual processing cost. Since fees change it is is a breaking change that will take effect in v4 of the protocol. (#2186)
* **platform:** withdrawal automatic retries after core rejection. This is a breaking change that will be marked as active in v1.4 (#2185)
* **platform:** withdrawal limits. This is breaking, and will be activated in version 1.4 (#2182)
* **sdk:** Now if network is not specified, JS SDK will connect to mainnet. (#2161)
* **dashmate:** confirm a node reset. This change will break any non interactive execution of reset command so now the force flag must be provided to skip the reset confirmation. (#2160)
* **platform:** withdrawals polishing and fixes for mainnet. Updating in V4 hard fork. (#2166)
* **platform:** do not switch to oldest quorums in validator set update. This is included as a change in protocol version 4. (#2167)


## [1.4.0-dev.8](https://github.com/dashpay/platform/compare/v1.4.0-dev.7...v1.4.0-dev.8) (2024-10-08)


### Features

* **sdk:** fix client tls connections ([#2223](https://github.com/dashpay/platform/issues/2223))


### Bug Fixes

* **platform:** fixed Platform State deserialization issue ([#2227](https://github.com/dashpay/platform/issues/2227))

## [1.4.0-dev.7](https://github.com/dashpay/platform/compare/v1.4.0-dev.6...v1.4.0-dev.7) (2024-10-07)


### Miscellaneous Chores

* **drive:** log invalid state on deserialisation ([#2220](https://github.com/dashpay/platform/issues/2220))

## [1.4.0-dev.6](https://github.com/dashpay/platform/compare/v1.4.0-dev.5...v1.4.0-dev.6) (2024-10-07)


### Miscellaneous Chores

* **sdk:** expose drive module in public API for rs-sdk ([#2217](https://github.com/dashpay/platform/issues/2217))
* update dependences ([#2072](https://github.com/dashpay/platform/issues/2072))

## [1.4.0-dev.5](https://github.com/dashpay/platform/compare/v1.4.0-dev.4...v1.4.0-dev.5) (2024-10-07)


### ⚠ BREAKING CHANGES

* **platform:** add owner keys to identities, fixed verification of use of owner keys (#2215)

### Features

* **dpp:** add a convenience method to get the public key data for a private key depending on the key type ([#2214](https://github.com/dashpay/platform/issues/2214))
* **platform:** add owner keys to identities, fixed verification of use of owner keys ([#2215](https://github.com/dashpay/platform/issues/2215))

## [1.4.0-dev.4](https://github.com/dashpay/platform/compare/v1.4.0-dev.3...v1.4.0-dev.4) (2024-10-05)


### Features

* **sdk:** enable withdrawals v1 in JS SDK ([#2201](https://github.com/dashpay/platform/issues/2201))
* start network with latest version if genesis version not set ([#2206](https://github.com/dashpay/platform/issues/2206))


### Bug Fixes

* cookie accepts cookie name, path, and domain with out of bounds characters ([#2211](https://github.com/dashpay/platform/issues/2211))
* **drive:** set sign height when rebroadcasting ([#2210](https://github.com/dashpay/platform/issues/2210))
* **sdk:** small sdk improvements and fixes for v1.4 ([#2200](https://github.com/dashpay/platform/issues/2200))


### Code Refactoring

* **drive:** remove duplicated withdrawal amount validation ([#2191](https://github.com/dashpay/platform/issues/2191))


### Miscellaneous Chores

* bump GroveDB dependency ([#2196](https://github.com/dashpay/platform/issues/2196))
* **drive:** improve withdrawal logging ([#2203](https://github.com/dashpay/platform/issues/2203))
* **drive:** logs and metrics for withdrawal daily limit ([#2192](https://github.com/dashpay/platform/issues/2192))
* **release:** replace colima with native docker in macOS builds ([#2188](https://github.com/dashpay/platform/issues/2188))


### Tests

* **test-suite:** enable withdrawal tests ([#2202](https://github.com/dashpay/platform/issues/2202))

## [1.4.0-dev.2](https://github.com/dashpay/platform/compare/v1.4.0-dev.1...v1.4.0-dev.2) (2024-09-30)


### ⚠ BREAKING CHANGES

* **platform:** matched withdrawal fees to actual processing cost (#2186)
* **platform:** withdrawal automatic retries after core rejection (#2185)
* **platform:** withdrawal limits (#2182)

### Features

* **dashmate:** collect docker stats in the doctor command ([#2180](https://github.com/dashpay/platform/issues/2180))
* **dashmate:** validate external IP ([#2183](https://github.com/dashpay/platform/issues/2183))
* **platform:** matched withdrawal fees to actual processing cost ([#2186](https://github.com/dashpay/platform/issues/2186))
* **platform:** withdrawal automatic retries after core rejection ([#2185](https://github.com/dashpay/platform/issues/2185))
* **platform:** withdrawal limits ([#2182](https://github.com/dashpay/platform/issues/2182))
* **sdk:** get node status ([#2139](https://github.com/dashpay/platform/issues/2139))


### Build System

* devcontainer support ([#2179](https://github.com/dashpay/platform/issues/2179))


### Continuous Integration

* prebuild dev containers ([#2184](https://github.com/dashpay/platform/issues/2184))

## [1.4.0-dev.1](https://github.com/dashpay/platform/compare/v1.3.0...v1.4.0-dev.1) (2024-09-27)


### ⚠ BREAKING CHANGES

* **sdk:** change default network to mainnet (#2161)
* **dashmate:** confirm a node reset (#2160)
* **platform:** withdrawals polishing and fixes for mainnet (#2166)
* **platform:** do not switch to oldest quorums in validator set update (#2167)

### Features

* **dashmate:** confirm a node reset ([#2160](https://github.com/dashpay/platform/issues/2160))
* **platform:** do not switch to oldest quorums in validator set update ([#2167](https://github.com/dashpay/platform/issues/2167))
* **platform:** get current quorum info  ([#2168](https://github.com/dashpay/platform/issues/2168))
* **platform:** withdrawals polishing and fixes for mainnet ([#2166](https://github.com/dashpay/platform/issues/2166))
* **sdk:** change default network to mainnet ([#2161](https://github.com/dashpay/platform/issues/2161))


### Bug Fixes

* **dapi:** getStatus cache invalidation ([#2155](https://github.com/dashpay/platform/issues/2155))
* **dapi:** invalid mainnet seed ports ([#2173](https://github.com/dashpay/platform/issues/2173))
* **dashmate:** cannot read properties of undefined (reading 'expires') ([#2164](https://github.com/dashpay/platform/issues/2164))
* **dashmate:** colors[updated] is not a function ([#2157](https://github.com/dashpay/platform/issues/2157))
* **dashmate:** doctor fails collecting to big logs ([#2158](https://github.com/dashpay/platform/issues/2158))
* **dashmate:** port marks as closed if ipv6 is not disabled ([#2162](https://github.com/dashpay/platform/issues/2162))
* **dashmate:** remove confusing short flag name ([#2165](https://github.com/dashpay/platform/issues/2165))


### Continuous integration

* build dashmate package on macos14


### Documentation

* **dashmate:** document logging configuration ([#2156](https://github.com/dashpay/platform/issues/2156))


### Tests

* **dashmate:** e2e tests failing due to DKG interval check ([#2171](https://github.com/dashpay/platform/issues/2171))


### Miscellaneous Chores

* **dashmate:** do not call mint on masternodes ([#2172](https://github.com/dashpay/platform/issues/2172))
* **platform:** protocol version 4 creation ([#2153](https://github.com/dashpay/platform/issues/2153))


### [1.3.1](https://github.com/dashpay/platform/compare/v1.3.0...v1.3.1) (2024-09-27)

### Bug Fixes

* **dapi:** getStatus cache invalidation ([#2155](https://github.com/dashpay/platform/issues/2155))
* **dapi:** invalid mainnet seed ports ([#2173](https://github.com/dashpay/platform/issues/2173))
* **dashmate:** cannot read properties of undefined (reading 'expires') ([#2164](https://github.com/dashpay/platform/issues/2164))
* **dashmate:** colors[updated] is not a function ([#2157](https://github.com/dashpay/platform/issues/2157))
* **dashmate:** doctor fails collecting to big logs ([#2158](https://github.com/dashpay/platform/issues/2158))
* **dashmate:** port marks as closed if ipv6 is not disabled ([#2162](https://github.com/dashpay/platform/issues/2162))


### Tests

* **dashmate:** e2e tests failing due to DKG interval check ([#2171](https://github.com/dashpay/platform/issues/2171))


## [1.3.0](https://github.com/dashpay/platform/compare/v1.2.0...v1.3.0) (2024-09-19)

### Features

* **platform:** query many identity balances at a time ([#2112](https://github.com/dashpay/platform/pull/2112))
* **platform:** query block count per Evonode proposed in any given epoch ([#2114](https://github.com/dashpay/platform/pull/2114))
* **platform:** contests on testnet/devnet/local should take less time ([#2115](https://github.com/dashpay/platform/pull/2115))
* **dapi:** implement getIdentityBalance ([#2105](https://github.com/dashpay/platform/pull/2105))
* **dashmate:** doctor diagnostic ([#2085](https://github.com/dashpay/platform/pull/2085))
* **dashmate:** enhance core logging configuration ([#2121](https://github.com/dashpay/platform/pull/2121))
* **platform:** support Tenderdash upgrade ([#2136](https://github.com/dashpay/platform/pull/2136))

### Bug Fixes

* **sdk:** use proofs when waiting for asset lock ([#2067](https://github.com/dashpay/platform/pull/2067))
* **platform:** contested username distribution ([#2118](https://github.com/dashpay/platform/pull/2118))
* **drive-abci:** require 75 percent of active, not total hpmns ([#2127](https://github.com/dashpay/platform/pull/2127))
* **drive-abci:** cleanup of later contests on testnet only ([#2134](https://github.com/dashpay/platform/pull/2134))
* **platform:** contested username time fix ([#2137](https://github.com/dashpay/platform/pull/2137))
* **dashmate:** invalid debug core log path ([#2143](https://github.com/dashpay/platform/pull/2143))
* **tests:** fix upgrade tests because of 51% limit on v1 ([#2151](https://github.com/dashpay/platform/pull/2151))
* **platform:** add limits to identity balances and proposed block counts queries ([#2148](https://github.com/dashpay/platform/pull/2148))
* body-parser vulnerability ([#2119](https://github.com/dashpay/platform/pull/2119))

### Miscellaneous Chores

* add change base branch script ([#2082](https://github.com/dashpay/platform/pull/2082))
* emergency version upgrade to v1.3 Protocol Version 2 ([#2138](https://github.com/dashpay/platform/pull/2138))
* disable config from testnet propagates network test ([#2149](https://github.com/dashpay/platform/pull/2149))
* **js-sdk** connect to mainnet by default ([#2146](https://github.com/dashpay/platform/pull/2146))

### Build System

* cache Rust dependencies build as a docker layer ([#1900](https://github.com/dashpay/platform/pull/1900))
* **drive-abci:** debug docker image with grovedb visualizer and tokio console ([#2012](https://github.com/dashpay/platform/pull/2012))
* **dashmate:** update tenderdash image to fix-wrong-proposer-at-round ([#2140](https://github.com/dashpay/platform/pull/2140))
* bump rs-tenderdash-abci to 1.2.0+1.3.0 ([#2147](https://github.com/dashpay/platform/pull/2147))

### ⚠ BREAKING CHANGES

* **platform:** contested username distribution ([#2118](https://github.com/dashpay/platform/pull/2118))
* **platform:** contests on testnet/devnet/local should take less time ([#2115](https://github.com/dashpay/platform/pull/2115))
* **dashmate:** docker logs rotation ([#2125](https://github.com/dashpay/platform/pull/2125))

**Full Changelog**: [https://github.com/dashpay/platform/compare/v1.2.0...v1.3.0](https://github.com/dashpay/platform/compare/v1.2.0...v1.3.0)

## [1.3.0-dev.7](https://github.com/dashpay/platform/compare/v1.3.0-dev.6...v1.3.0-dev.7) (2024-09-18)


### Bug Fixes

* **dashmate:** invalid debug core log path ([#2143](https://github.com/dashpay/platform/issues/2143))


### Miscellaneous Chores

* change Upgrade 4 Epochs Later ([#2144](https://github.com/dashpay/platform/issues/2144))

## [1.3.0-dev.6](https://github.com/dashpay/platform/compare/v1.3.0-dev.5...v1.3.0-dev.6) (2024-09-18)


### ⚠ BREAKING CHANGES

* **platform:** support Tenderdash upgrade (#2136)
* **platform:** contested username time fix (#2137)
* **platform:** emergency version upgrade to v1.3 Protocol Version 2 (#2138)
* 
### Bug Fixes

* **platform:** contested username time fix ([#2137](https://github.com/dashpay/platform/issues/2137))

## [1.3.0-dev.5](https://github.com/dashpay/platform/compare/v1.3.0-dev.4...v1.3.0-dev.5) (2024-09-16)


### Bug Fixes

* **drive-abci:** cleanup of later contests on testnet only ([#2134](https://github.com/dashpay/platform/issues/2134))

## [1.3.0-dev.4](https://github.com/dashpay/platform/compare/v1.3.0-dev.3...v1.3.0-dev.4) (2024-09-16)


### Bug Fixes

* require75p of active not total hpmns ([#2129](https://github.com/dashpay/platform/issues/2129))

## [1.3.0-dev.3](https://github.com/dashpay/platform/compare/v1.3.0-dev.2...v1.3.0-dev.3) (2024-09-16)


### Bug Fixes

* **drive-abci:** require 75 percent of active, not total hpmns ([#2127](https://github.com/dashpay/platform/issues/2127))

## [1.3.0-dev.2](https://github.com/dashpay/platform/compare/v1.3.0-dev.1...v1.3.0-dev.2) (2024-09-16)


### ⚠ BREAKING CHANGES

* **platform:** contests on testnet should take less time (#2115)
* **platform:** contested username distribution (#2118)

### Features

* **platform:** contests on testnet should take less time ([#2115](https://github.com/dashpay/platform/issues/2115))


### Bug Fixes

* body-parser vulnerability ([#2119](https://github.com/dashpay/platform/issues/2119))
* **platform:** contested username distribution ([#2118](https://github.com/dashpay/platform/issues/2118))


### Miscellaneous Chores

* **dashmate:** update platform images to `1-dev` ([#2120](https://github.com/dashpay/platform/issues/2120))


### Styles

* **dapi:** better api for identities balances ([#2122](https://github.com/dashpay/platform/issues/2122))

## [1.3.0-dev.1](https://github.com/dashpay/platform/compare/v1.2.0...v1.3.0-dev.1) (2024-09-12)


### Features

* query block count per Evonode proposed in any given epoch ([#2114](https://github.com/dashpay/platform/issues/2114))
* query many identity balances at a time ([#2112](https://github.com/dashpay/platform/issues/2112))


### Bug Fixes

* **sdk:** use proofs when waiting for asset lock ([#2067](https://github.com/dashpay/platform/issues/2067))


### Build System

* cache Rust dependencies build as a docker layer ([#1900](https://github.com/dashpay/platform/issues/1900))
* **drive-abci:** debug docker image with grovedb visualizer and tokio console ([#2012](https://github.com/dashpay/platform/issues/2012))


### Miscellaneous Chores

* add change base branch script ([#2082](https://github.com/dashpay/platform/issues/2082))
* creation of protocol V2 ([#2104](https://github.com/dashpay/platform/issues/2104))

## [1.2.0](https://github.com/dashpay/platform/compare/v1.1.1...v1.2.0) (2024-08-30)


### Features

* **dapi:** serve even if tenderdash is not connected ([#2086](https://github.com/dashpay/platform/issues/2086))
* **dashmate:** validate SSL certificate files ([#2089](https://github.com/dashpay/platform/issues/2089))
* platform status endpoint ([#2088](https://github.com/dashpay/platform/issues/2088))
* script to check which nodes are updated to v1.1 ([#2083](https://github.com/dashpay/platform/issues/2083))


### Bug Fixes

* **dashmate:** docker-compose version is obsolete ([#2073](https://github.com/dashpay/platform/issues/2073))
* replay issue when round is 0 on replay. ([#2091](https://github.com/dashpay/platform/issues/2091))
* security vulnerability in webpack ([#2090](https://github.com/dashpay/platform/issues/2090))


### Miscellaneous Chores

* **dashmate:** update tenderdash version ([#2093](https://github.com/dashpay/platform/issues/2093))
* **dashmate:** update tenderdash version to 1.2.0 ([#2078](https://github.com/dashpay/platform/issues/2078))
* update rust to 1.80 ([#2070](https://github.com/dashpay/platform/issues/2070))



## [1.2.0-rc.1](https://github.com/dashpay/platform/compare/v1.1.1...v1.2.0-rc.1) (2024-08-30)


### Features

* **dapi:** serve even if tenderdash is not connected ([#2086](https://github.com/dashpay/platform/issues/2086))
* **dashmate:** validate SSL certificate files ([#2089](https://github.com/dashpay/platform/issues/2089))
* platform status endpoint ([#2088](https://github.com/dashpay/platform/issues/2088))
* script to check which nodes are updated to v1.1 ([#2083](https://github.com/dashpay/platform/issues/2083))


### Bug Fixes

* **dashmate:** docker-compose version is obsolete ([#2073](https://github.com/dashpay/platform/issues/2073))
* replay issue when round is 0 on replay. ([#2091](https://github.com/dashpay/platform/issues/2091))
* security vulnerability in webpack ([#2090](https://github.com/dashpay/platform/issues/2090))


### Miscellaneous Chores

* **dashmate:** update tenderdash version ([#2093](https://github.com/dashpay/platform/issues/2093))
* **dashmate:** update tenderdash version to 1.2.0 ([#2078](https://github.com/dashpay/platform/issues/2078))
* update rust to 1.80 ([#2070](https://github.com/dashpay/platform/issues/2070))

### [1.1.1](https://github.com/dashpay/platform/compare/v1.1.0...v1.1.1) (2024-08-25)


### Features

* **dashmate:** update testnet config ([#2079](https://github.com/dashpay/platform/issues/2079))


### Miscellaneous Chores

* **dashmate:** update tenderdash version to 1.2.0 ([#2078](https://github.com/dashpay/platform/issues/2078))


## [1.1.0](https://github.com/dashpay/platform/compare/v1.1.0-dev.1...v1.1.0) (2024-08-24)


### ⚠ BREAKING CHANGES

* **drive:** just in time fee update fixes (#2075)
* do not allow contested documents for the first three epochs (#2066)
* **drive-abci:** fix wrong fields in dash top level domain  (#2065)
* **platform:** fix reference of items between epochs (#2064)
* **sdk:** mock sdk cannot find quorum keys in offline mode (#2061)
* **sdk:** overflow when using &&sdk in DapiRequestExecutor (#2060)

### Features

* **dashmate:** add `dashmate doctor` command ([#2024](https://github.com/dashpay/platform/issues/2024))
* **dashmate:** compress doctor report and other improvements ([#2071](https://github.com/dashpay/platform/issues/2071))
* **dashmate:** configure proposer and tx limits ([#2057](https://github.com/dashpay/platform/issues/2057))
* **dpp:** function for getting enabled matching public keys in identities ([#2052](https://github.com/dashpay/platform/issues/2052))
* where clauses recognize nested properties


### Bug Fixes

* add back the matches on system properties
* **dapi:** getTotalCreditsOnPlatform missing parts ([#2059](https://github.com/dashpay/platform/issues/2059))
* **dashmate:** core reindex command not working ([#2054](https://github.com/dashpay/platform/issues/2054))
* **dashmate:** the reset platform command doesn't remove data ([#2053](https://github.com/dashpay/platform/issues/2053))
* **drive-abci:** fix wrong fields in dash top level domain  ([#2065](https://github.com/dashpay/platform/issues/2065))
* **drive:** just in time fee update fixes ([#2075](https://github.com/dashpay/platform/issues/2075))
* **platform:** fix reference of items between epochs ([#2064](https://github.com/dashpay/platform/issues/2064))
* **sdk:** mock sdk cannot find quorum keys in offline mode ([#2061](https://github.com/dashpay/platform/issues/2061))
* **sdk:** overflow when using &&sdk in DapiRequestExecutor ([#2060](https://github.com/dashpay/platform/issues/2060))


### Code Refactoring

* rename getTotalCreditsOnPlatform ([#2056](https://github.com/dashpay/platform/issues/2056))


### Miscellaneous Chores

* do not allow contested documents for the first three epochs ([#2066](https://github.com/dashpay/platform/issues/2066))

## [1.1.0-dev.1](https://github.com/dashpay/platform/compare/v1.0.2...v1.1.0-dev.1) (2024-08-13)


### ⚠ BREAKING CHANGES

* masternode reward payouts are changed so previously created state won't be compatible (#2032)
* previously created networks won't be supported since genesis configuration is changed (#2042)
* added genesis core height in misc tree so previously created state won't be compatible (#2038)

### Features

* configure wait for ST result timeout ([#2045](https://github.com/dashpay/platform/issues/2045))
* **dashmate:** configure tenderdash connections ([#2048](https://github.com/dashpay/platform/issues/2048))
* **drive-abci:** skip state transition txs if time limit is reached on prepare_proposal ([#2041](https://github.com/dashpay/platform/issues/2041))
* **platform:** store/fetch genesis core height in misc tree ([#2038](https://github.com/dashpay/platform/issues/2038))
* **platform:** total credits on platform query and fix for reward distribution ([#2032](https://github.com/dashpay/platform/issues/2032))


### Miscellaneous Chores

* **dashmate:** update consensus params ([#2042](https://github.com/dashpay/platform/issues/2042))
* **dashmate:** update tenderdash seed ([#2040](https://github.com/dashpay/platform/issues/2040))
* ignore security vulnerability 1098397 ([#2044](https://github.com/dashpay/platform/issues/2044))

### [1.0.2](https://github.com/dashpay/platform/compare/v1.0.1...v1.0.2) (2024-07-31)


### Features

* **dashmate:** a flag to keep data on reset ([#2026](https://github.com/dashpay/platform/issues/2026))


### Bug Fixes

* **dashmate:** status command shows tenderdash error before activation ([#2028](https://github.com/dashpay/platform/issues/2028))
* **dashmate:** unnecessary core indexes are required ([#2025](https://github.com/dashpay/platform/issues/2025))

### [1.0.1](https://github.com/dashpay/platform/compare/v1.0.0...v1.0.1) (2024-07-29)


### Miscellaneous Chores

* bump dash-spv version to 2.0.0

## [1.0.0](https://github.com/dashpay/platform/compare/v1.0.0-rc.2...v1.0.0) (2024-07-29)


### Features

* sdk to return proofs if requested ([#2014](https://github.com/dashpay/platform/issues/2014))


### Bug Fixes

* **dashmate:** imported node is not starting ([#2009](https://github.com/dashpay/platform/issues/2009))
* **dashmate:** remove `dash-cli` from protx registration instructions ([#2018](https://github.com/dashpay/platform/issues/2018))
* epoch protocol version setting ([#2013](https://github.com/dashpay/platform/issues/2013))


### Build System

* update tenderdash to 1.1.0 ([#2017](https://github.com/dashpay/platform/issues/2017))


### Miscellaneous Chores

* **dashmate:** configure mainnet ([#2016](https://github.com/dashpay/platform/issues/2016))
* update to GroveDB Version 1.0.0 ([#2015](https://github.com/dashpay/platform/issues/2015))

## [1.0.0-rc.2](https://github.com/dashpay/platform/compare/v1.0.0-rc.1...v1.0.0-rc.2) (2024-07-25)


### ⚠ BREAKING CHANGES

* **platform:** genesis state from core block time (#2003)

### Features

* specify transition names within documents batch ([#2007](https://github.com/dashpay/platform/issues/2007))


### Bug Fixes

* dpns js sdk fix for identity record rename ([#2001](https://github.com/dashpay/platform/issues/2001))
* **platform:** core info is lost between genesis and first block ([#2004](https://github.com/dashpay/platform/issues/2004))
* **platform:** genesis state from core block time ([#2003](https://github.com/dashpay/platform/issues/2003))
* sdk should ignore transient fields when verifying proofs ([#2000](https://github.com/dashpay/platform/issues/2000))
* **strategy-tests:** document delete transitions were not selecting identity correctly
* two error messages had typos ([#2005](https://github.com/dashpay/platform/issues/2005))


### Miscellaneous Chores

* **dashmate:** update genesis config to the latest testnet ([#1998](https://github.com/dashpay/platform/issues/1998))


### Build System

* update to tenderdash 1.1.0-dev.3, rs-tenderdash-abci 1.1.0-dev.1 ([#2008](https://github.com/dashpay/platform/issues/2008))

## [1.0.0-rc.1](https://github.com/dashpay/platform/compare/v1.0.0-beta.4...v1.0.0-rc.1) (2024-07-24)


### ⚠ BREAKING CHANGES

* **platform:** system data contracts should not have an owner (#1992)
* **platform:** transient properties (#1990)
* **platform:** document types should not have a contested unique index with a unique index  (#1984)
* **platform:** add hyphen to match for contested documents on Dashpay (#1982)

### Features

* **drive:** added config for grovedb verify on startup ([#1975](https://github.com/dashpay/platform/issues/1975))
* **platform:** system data contracts should not have an owner ([#1992](https://github.com/dashpay/platform/issues/1992))
* **platform:** transient properties ([#1990](https://github.com/dashpay/platform/issues/1990))
* use all eligible identities and slightly more robust checking


### Bug Fixes

* **dapi:** can't parse masternode list diff ([#1988](https://github.com/dashpay/platform/issues/1988))
* **drive:** unknown mn_rr fork height ([#1994](https://github.com/dashpay/platform/issues/1994))
* improve efficiency of identity random sampling
* only clone the eligible identities
* **platform:** add hyphen to match for contested documents on Dashpay ([#1982](https://github.com/dashpay/platform/issues/1982))
* **platform:** document types should not have a contested unique index with a unique index  ([#1984](https://github.com/dashpay/platform/issues/1984))
* select random identities for strategy documents
* spent asset lock estimated fees, and misc ([#1993](https://github.com/dashpay/platform/issues/1993))
* **strategy-tests:** key ids for new identities with extra keys were not calculated properly ([#1991](https://github.com/dashpay/platform/issues/1991))
* **strategy-tests:** transfer keys were being disabled ([#1995](https://github.com/dashpay/platform/issues/1995))
* voting test


### Miscellaneous Chores

* chose capable identities for random documents
* **dapi:** enable logger for reconnectable stream ([#1986](https://github.com/dashpay/platform/issues/1986))


### Build System

* update tenderdash to 1.1.0-dev.1 ([#1985](https://github.com/dashpay/platform/issues/1985))
* update tenderdash to 1.1.0-dev.2 ([#1996](https://github.com/dashpay/platform/issues/1996))

## [1.0.0-beta.4](https://github.com/dashpay/platform/compare/v1.0.0-beta.3...v1.0.0-beta.4) (2024-07-19)


### ⚠ BREAKING CHANGES

* **drive:** don't use `0.0.0.0` as default listen IP (#1976)

### Bug Fixes

* **dashmate:** configure devnet quorums ([#1979](https://github.com/dashpay/platform/issues/1979))
* **drive:** drive and tenderdash are constantly restarting ([#1978](https://github.com/dashpay/platform/issues/1978))
* expected service to be a string with ip address and port ([#1980](https://github.com/dashpay/platform/issues/1980))


### Code Refactoring

* **drive:** don't use private bound for public trait ([#1974](https://github.com/dashpay/platform/issues/1974))


### Miscellaneous Chores

* **drive:** don't use `0.0.0.0` as default listen IP ([#1976](https://github.com/dashpay/platform/issues/1976))

## [1.0.0-beta.3](https://github.com/dashpay/platform/compare/v1.0.0-beta.2...v1.0.0-beta.3) (2024-07-17)


### ⚠ BREAKING CHANGES

* **platform:** updated fees (#1971)
* **platform:** max field size and some clean up of versioning (#1970)

### Features

* **dpp:** decomposed integer types for document type properties ([#1968](https://github.com/dashpay/platform/issues/1968))
* **platform:** max field size and some clean up of versioning ([#1970](https://github.com/dashpay/platform/issues/1970))


### Continuous Integration

* fix release docker images ([#1969](https://github.com/dashpay/platform/issues/1969))


### Miscellaneous Chores

* activate platform on EHF fork ([#1972](https://github.com/dashpay/platform/issues/1972))
* add comments to the platform.proto file ([#1641](https://github.com/dashpay/platform/issues/1641))
* **platform:** updated fees ([#1971](https://github.com/dashpay/platform/issues/1971))

## [1.0.0-beta.2](https://github.com/dashpay/platform/compare/v1.0.0-beta.1...v1.0.0-beta.2) (2024-07-16)


### Continuous Integration

* fix docker build for release ([#1965](https://github.com/dashpay/platform/issues/1965))

## [1.0.0-beta.1](https://github.com/dashpay/platform/compare/v1.0.0-dev.16...v1.0.0-beta.1) (2024-07-16)


### ⚠ BREAKING CHANGES

* **platform:** disable credit withdrawals in V1 (#1961)
* **drive-abci:** rotate quorums when all quorums members have had a chance to propose a block (#1942)
* allowed to make required fields optional (#1919)
* **dpp:** data contract validation issues (#1851)
* **platform:** proofs v1 support (#1934)
* **dpp:** do not allow `dependentSchemas` (#1888)
* **sdk:** impl Fetch/FetchMany for masternode voting endpoints (#1864)

### Features

* contender serialization ([#1882](https://github.com/dashpay/platform/issues/1882))
* **dashmate:** import existing Core data ([#1915](https://github.com/dashpay/platform/issues/1915))
* **dashmate:** verify system requirements ([#1914](https://github.com/dashpay/platform/issues/1914))
* **drive-abci:** rotate quorums when all quorums members have had a chance to propose a block ([#1942](https://github.com/dashpay/platform/issues/1942))
* **drive:** platform version patching and state migrations ([#1941](https://github.com/dashpay/platform/issues/1941))
* integrate grovedb visualizer ([#1933](https://github.com/dashpay/platform/issues/1933))
* **platform:** proofs v1 support ([#1934](https://github.com/dashpay/platform/issues/1934))
* **platform:** update to versioned grove db ([#1943](https://github.com/dashpay/platform/issues/1943))
* remove votes of removed masternodes when collateral is moved ([#1894](https://github.com/dashpay/platform/issues/1894))
* **sdk:** impl Fetch/FetchMany for masternode voting endpoints ([#1864](https://github.com/dashpay/platform/issues/1864))
* **sdk:** support mocking of error responses ([#1926](https://github.com/dashpay/platform/issues/1926))
* versioning of action conversion ([#1957](https://github.com/dashpay/platform/issues/1957))


### Bug Fixes

* Abstain and Lock trees for votes are now always first and fixed some limits ([#1921](https://github.com/dashpay/platform/issues/1921))
* added description keyword to schema for contested index
* allowed to make required fields optional ([#1919](https://github.com/dashpay/platform/issues/1919))
* build broken after merge of contested unique indexes validation ([#1892](https://github.com/dashpay/platform/issues/1892))
* cleanup fix and remove identitiesIdsOnly Vote State query ([#1890](https://github.com/dashpay/platform/issues/1890))
* contested document resolution fixes 2 and improvement to masternode vote ([#1904](https://github.com/dashpay/platform/issues/1904))
* contested resources query fixes ([#1896](https://github.com/dashpay/platform/issues/1896))
* contested unique indexes can only be on non mutable document types ([#1891](https://github.com/dashpay/platform/issues/1891))
* **dashmate:** cannot read properties of null (reading '1') ([#1939](https://github.com/dashpay/platform/issues/1939))
* **dashmate:** restart platform waits for DKG ([#1944](https://github.com/dashpay/platform/issues/1944))
* **dpp:** data contract validation issues ([#1851](https://github.com/dashpay/platform/issues/1851))
* **dpp:** document factory wouldn't allow delete transitions for immutable document types ([#1956](https://github.com/dashpay/platform/issues/1956))
* **drive:** add validation that an identity can not apply to be a contender in a contest twice. ([#1923](https://github.com/dashpay/platform/issues/1923))
* **drive:** contested document resolution with masternode voting batch empty fix ([#1880](https://github.com/dashpay/platform/issues/1880))
* **drive:** panic if PlatformState has serialisation error ([#1945](https://github.com/dashpay/platform/issues/1945))
* **drive:** valid instant lock signatures marked as invalid ([#1946](https://github.com/dashpay/platform/issues/1946))
* duplicate fields defined in DPNS contract
* final clean up and fixing of contested resource voting PR
* fixed voting strategy tests and cleanup
* import fix for drive refactoring ([#1959](https://github.com/dashpay/platform/issues/1959))
* incorrect proofs are returned for various state transitions ([#1912](https://github.com/dashpay/platform/issues/1912))
* merkle root hash verification failed on devnet ([#1929](https://github.com/dashpay/platform/issues/1929))
* minor issues detected by github actions ([#1928](https://github.com/dashpay/platform/issues/1928))
* **sdk:** panic GrpcContextProvider on async call inside sync code ([#1870](https://github.com/dashpay/platform/issues/1870))
* **sdk:** state transition broadcast missing contract provider ([#1913](https://github.com/dashpay/platform/issues/1913))
* small fix fixing compilation
* small fix for test: test_document_creation_on_contested_unique_index
* some document error messages didnt specify the corresponding property ([#1873](https://github.com/dashpay/platform/issues/1873))
* sum tree verification with specialized balances ([#1899](https://github.com/dashpay/platform/issues/1899))
* voting proofs work as intended and various fixes ([#1910](https://github.com/dashpay/platform/issues/1910))


### Build System

* update rs-tenderdash-abci to 1.0.0-dev.1 ([#1909](https://github.com/dashpay/platform/issues/1909))
* upgrade rs-tenderdash-abci to v1.0.0 and tenderdash to v1.0.0 ([#1918](https://github.com/dashpay/platform/issues/1918))
* use ubuntu-platform github runner hardware for all github actions ([#1920](https://github.com/dashpay/platform/issues/1920))


### Styles

* **drive:** update formatting


### Tests

* fix documentTransition.hasPrefundedBalance is not a function ([#1931](https://github.com/dashpay/platform/issues/1931))
* **sdk:** disable failing tests for bugs scheduled for future ([#1930](https://github.com/dashpay/platform/issues/1930))
* **sdk:** increase test coverage of masternode voting ([#1906](https://github.com/dashpay/platform/issues/1906))
* **sdk:** masternode voting SDK tests ([#1893](https://github.com/dashpay/platform/issues/1893))
* **sdk:** regenerate test vectors for masternode voting ([#1927](https://github.com/dashpay/platform/issues/1927))
* temporary skip withdrawal tests


### Code Refactoring

* changed Epoch serialization to make it slightly more efficient ([#1953](https://github.com/dashpay/platform/issues/1953))
* cleanup of warnings and fix tests
* extract document faker to crate ([#1887](https://github.com/dashpay/platform/issues/1887))
* fees to use version system ([#1911](https://github.com/dashpay/platform/issues/1911))
* final drive refactoring ([#1958](https://github.com/dashpay/platform/issues/1958))
* move rs-random-document to separate crate ([#1952](https://github.com/dashpay/platform/issues/1952))
* multiplier to version system and tests for refunds ([#1950](https://github.com/dashpay/platform/issues/1950))
* rename DriveQuery to DriveDocumentQuery ([#1954](https://github.com/dashpay/platform/issues/1954))
* use library for feature version ([#1938](https://github.com/dashpay/platform/issues/1938))


### Continuous Integration

* explicitly authenticate AWS ([#1960](https://github.com/dashpay/platform/issues/1960))


### Miscellaneous Chores

* autogenerated grpc code
* better logging for devnet upgrade protocol test ([#1925](https://github.com/dashpay/platform/issues/1925))
* **dashmate:** core RPC platform services authentication ([#1883](https://github.com/dashpay/platform/issues/1883))
* **dashmate:** enable Core RPC whitelists ([#1962](https://github.com/dashpay/platform/issues/1962))
* **dashmate:** provide debug information if version check fails ([#1936](https://github.com/dashpay/platform/issues/1936))
* **dpp:** do not allow `dependentSchemas` ([#1888](https://github.com/dashpay/platform/issues/1888))
* **drive:** additional logging and minor refactoring ([#1947](https://github.com/dashpay/platform/issues/1947))
* **platform:** disable credit withdrawals in V1 ([#1961](https://github.com/dashpay/platform/issues/1961))
* removed unused dpp code on state transition actions (old duplicate) ([#1955](https://github.com/dashpay/platform/issues/1955))
* renamed back vote_choices to votes on places where it had been incorrectly changed
* revisit system data contracts ([#1889](https://github.com/dashpay/platform/issues/1889))
* temp squash of masternode voting into higher branch ([#1877](https://github.com/dashpay/platform/issues/1877))
* update Cargo lock
* update masternode voting tests after merging in v1
* update to latest GroveDB (Proofs v1)
* update to latest grovedb 1.0.0-rc.2 ([#1948](https://github.com/dashpay/platform/issues/1948))
* validate that contested index is unique ([#1881](https://github.com/dashpay/platform/issues/1881))


### Documentation

* add llvm to README.md ([#1908](https://github.com/dashpay/platform/issues/1908))
* badge link for CI was broken in README.md ([#1932](https://github.com/dashpay/platform/issues/1932))
* update readme to add cmake ([#1837](https://github.com/dashpay/platform/issues/1837))

## [1.0.0-dev.16](https://github.com/dashpay/platform/compare/v1.0.0-dev.15...v1.0.0-dev.16) (2024-06-29)


### ⚠ BREAKING CHANGES

* **drive:** verify instant lock signatures with Drive (#1875)
* **dapi:** replace `getMnListDiff` with a streaming endpoint (#1859)
* **dapi:** disable unnecessary for v1 endpoints (#1857)
* **sdk:** dapi-grpc generated files overwritten on conflicting features (#1854)

### Features

* **dapi:** introduce `getBestBlockHeight` endpoint ([#1863](https://github.com/dashpay/platform/issues/1863))
* **dpp:** random documents based on JSON schema ([#1710](https://github.com/dashpay/platform/issues/1710))
* make data contract factory and json schema validator public


### Bug Fixes

* **dashmate:** background SSL renewal stuck on error ([#1897](https://github.com/dashpay/platform/issues/1897))
* **dashmate:** failed to read docker data on update ([#1903](https://github.com/dashpay/platform/issues/1903))
* **sdk:** dapi-grpc generated files overwritten on conflicting features ([#1854](https://github.com/dashpay/platform/issues/1854))
* **sdk:** invalid error returned when identity create fails ([#1856](https://github.com/dashpay/platform/issues/1856))
* security vulnerabilities in NPM deps ([#1860](https://github.com/dashpay/platform/issues/1860))
* validator field didn't need to be public for JsonSchemaValidator


### Performance Improvements

* **dapi:** cache `getBestBlockHash` endpoint ([#1867](https://github.com/dashpay/platform/issues/1867))
* **dapi:** cache `getBlockchainStatus` endpoint ([#1866](https://github.com/dashpay/platform/issues/1866))
* **dapi:** get many transactions at once ([#1858](https://github.com/dashpay/platform/issues/1858))
* **dapi:** replace `getMnListDiff` with a streaming endpoint ([#1859](https://github.com/dashpay/platform/issues/1859))
* **dapi:** use cached core height in streaming endpoints ([#1865](https://github.com/dashpay/platform/issues/1865))
* **drive:** verify instant lock signatures with Drive ([#1875](https://github.com/dashpay/platform/issues/1875))


### Miscellaneous Chores

* **dapi:** disable unnecessary for v1 endpoints ([#1857](https://github.com/dashpay/platform/issues/1857))
* mute NPM audit warnings ([#1879](https://github.com/dashpay/platform/issues/1879))
* update Karma to recent version ([#1901](https://github.com/dashpay/platform/issues/1901))
* update websocket client ([#1895](https://github.com/dashpay/platform/issues/1895))


### Code Refactoring

* **dpp:** change String and ByteArray DocumentPropertyType sizes to structs ([#1874](https://github.com/dashpay/platform/issues/1874))
* **drive:** encapsulate chain lock validation quorum logic ([#1868](https://github.com/dashpay/platform/issues/1868))

## [1.0.0-dev.15](https://github.com/dashpay/platform/compare/v1.0.0-dev.14...v1.0.0-dev.15) (2024-05-22)


### Miscellaneous Chores

* **drive:** state transition observability ([#1846](https://github.com/dashpay/platform/issues/1846))

## [1.0.0-dev.14](https://github.com/dashpay/platform/compare/v1.0.0-dev.13...v1.0.0-dev.14) (2024-05-17)


### ⚠ BREAKING CHANGES

* Data Contract Create and Update transitions validation logic is changed so previously created block chain data might not be valid anymore (#1835)

### Features

* **dashmate:** check for DKG before stopping node ([#1683](https://github.com/dashpay/platform/issues/1683))


### Bug Fixes

* data contract transition validation issues ([#1835](https://github.com/dashpay/platform/issues/1835))


### Code Refactoring

* rename `DataContractConfig.validate_config_update` ([#1843](https://github.com/dashpay/platform/issues/1843))
* rename `validate` to `full_validation` ([#1845](https://github.com/dashpay/platform/issues/1845))

## [1.0.0-dev.13](https://github.com/dashpay/platform/compare/v1.0.0-dev.12...v1.0.0-dev.13) (2024-05-09)


### ⚠ BREAKING CHANGES

* **sdk:** don't return Arc in SdkBuilder (#1838)
* **platform:** document creation/update/deletion does not refetch contract (#1840)

### Features

* **dashmate:** handle docker pull error on images update ([#1685](https://github.com/dashpay/platform/issues/1685))
* make document tranfers public
* make start identities number u16
* make purchase document public
* make sdk document purchases public ([#1832](https://github.com/dashpay/platform/issues/1832))
* make sdk files public
* put index serialization behind feature
* serialize for indexes and change error messages to strings
* use all current identities for strategy test state transitions ([#1820](https://github.com/dashpay/platform/issues/1820))


### Bug Fixes

* **platform:** npm audit security fix ([#1836](https://github.com/dashpay/platform/issues/1836))


### Code Refactoring

* **platform:** document creation/update/deletion does not refetch contract ([#1840](https://github.com/dashpay/platform/issues/1840))
* **sdk:** don't return Arc in SdkBuilder ([#1838](https://github.com/dashpay/platform/issues/1838))


### Miscellaneous Chores

* observability and security for HTTP gateway ([#1825](https://github.com/dashpay/platform/issues/1825))

## [1.0.0-dev.12](https://github.com/dashpay/platform/compare/v1.0.0-dev.11...v1.0.0-dev.12) (2024-04-29)


### ⚠ BREAKING CHANGES

* Removed `getIdentities` and `getIdentitiesByPublicKeyHashes` endpoints in favor of `getIdentitiesContractKeys` (#1766)
* **platform:** basic nft support (#1829)
* **dapi:** `getStatus` is removed in favor of `getMasternodeStatus` and `getBlockchainStatus` (#1812)
* **platform:** documents serialization format is changed that makes previously created block chain data invalid (#1826)

### Features

* **dapi:** split getStatus into two endpoints ([#1812](https://github.com/dashpay/platform/issues/1812))
* **drive-abci:** configure dir to store rejected txs ([#1823](https://github.com/dashpay/platform/issues/1823))
* getIdentitiesContractKeys endpoint ([#1766](https://github.com/dashpay/platform/issues/1766))
* **platform:** ability to transfer documents ([#1826](https://github.com/dashpay/platform/issues/1826))
* **platform:** basic nft support ([#1829](https://github.com/dashpay/platform/issues/1829))
* **sdk:** add query for data contract history ([#1787](https://github.com/dashpay/platform/issues/1787))
* **wallet-lib:** optional sync of the account ([#1830](https://github.com/dashpay/platform/issues/1830))


### Bug Fixes

* add tls-webpki-roots to support tls on mobile (Android, iOS) ([#1828](https://github.com/dashpay/platform/issues/1828))


### Miscellaneous Chores

* **dapi:** update autogenerated clients ([#1827](https://github.com/dashpay/platform/issues/1827))
* **dashmate:** limit concurrent state transition checks ([#1824](https://github.com/dashpay/platform/issues/1824))

## [1.0.0-dev.10](https://github.com/dashpay/platform/compare/v1.0.0-dev.9...v1.0.0-dev.10) (2024-04-04)


### ⚠ BREAKING CHANGES

There are multiple breaking changes that make previously created state invalid:
* **drive:** addition key-value in epoch trees (#1778)
* **platform:** processing costs were updated for some state transitions (#1805, #1800)
* **drive:** now we count and persist a version proposal vote on the epoch change (#1769)
* **drive:** protocol version for the first block of an epoch might be different (#1769)
* **platform:** ST validation was changed, as well as some constants (#1796, #1795)
* **dpp:** document type name must be 1 to 64 alphanumeric chars and "_", or "-" (#1798)
* **platform:** max state transition is 20 kB (#1792)

### Features

* **dpp:** validate document type name ([#1798](https://github.com/dashpay/platform/issues/1798))
* **drive-abci:** better processing costs of state transitions (no schema processing improvements) ([#1800](https://github.com/dashpay/platform/issues/1800))
* **drive:** provide protocol version in epoch info query ([#1778](https://github.com/dashpay/platform/issues/1778))
* pass asset lock vector rather than callback in strategies
* **platform:** improved state processing fees ([#1805](https://github.com/dashpay/platform/issues/1805))
* **platform:** mitigate issues of asset lock based transitions ([#1796](https://github.com/dashpay/platform/issues/1796))
* **platform:** various document validation improvements ([#1795](https://github.com/dashpay/platform/issues/1795))
* **strategy-tests:** add extra_keys field for StartIdentities and use random identities for transfers ([#1794](https://github.com/dashpay/platform/issues/1794))


### Bug Fixes

* **drive:** no longer build full grovedb when using verify feature ([#1804](https://github.com/dashpay/platform/issues/1804))
* **drive:** versioning issues on epoch change ([#1769](https://github.com/dashpay/platform/issues/1769))
* **platform:** max state transition size ([#1792](https://github.com/dashpay/platform/issues/1792))
* **sdk:** not bumping nonce on contract creation ([#1801](https://github.com/dashpay/platform/issues/1801))
* state transition already in chain error on low credit transfer amount ([#1797](https://github.com/dashpay/platform/issues/1797))
* **strategy-tests:** default identity nonce and document op contract id ([#1777](https://github.com/dashpay/platform/issues/1777))


### Performance Improvements

* **platform:** use inline on versioned functions ([#1793](https://github.com/dashpay/platform/issues/1793))


### Tests

* added a test registering many random contracts in strategy tests ([#1791](https://github.com/dashpay/platform/issues/1791))


### Miscellaneous Chores

* **sdk:** export various libraries in rs-sdk ([#1802](https://github.com/dashpay/platform/issues/1802))

## [1.0.0-dev.9](https://github.com/dashpay/platform/compare/v1.0.0-dev.8...v1.0.0-dev.9) (2024-03-19)


### ⚠ BREAKING CHANGES

* **sdk:** don't allow duplicate mock expectations (#1788)
* created_at and updated_at from block time (#1780)
* created_at_block_height and variants (#1784)


### Features

* created_at and updated_at from block time ([#1780](https://github.com/dashpay/platform/issues/1780))
* created_at_block_height and variants ([#1784](https://github.com/dashpay/platform/issues/1784))


### Bug Fixes

* **drive:** internal error on querying proofs ([#1747](https://github.com/dashpay/platform/issues/1747))
* identity add keys in strategy tests ([#1727](https://github.com/dashpay/platform/issues/1727))
* **sdk:** don't allow duplicate mock expectations ([#1788](https://github.com/dashpay/platform/issues/1788))
* query retry on race condition ([#1776](https://github.com/dashpay/platform/issues/1776))
* identity state transition validation fixes ([#1786](https://github.com/dashpay/platform/issues/1786))


### Code Refactoring

* make strategy start identities a new struct ([#1764](https://github.com/dashpay/platform/issues/1764))
* updated descriptions and function names in strategy tests plus readme file ([#1785](https://github.com/dashpay/platform/issues/1785))


### Miscellaneous Chores

* **dashmate:** readme fixes ([#1624](https://github.com/dashpay/platform/issues/1624))
* fix npm audit for follow-redirects package ([#1781](https://github.com/dashpay/platform/issues/1781))
* **dapi:** use broadcast_tx instead of deprecated broadcast_tx_sync ([#1775](https://github.com/dashpay/platform/issues/1775))


### Build System

* rs-tenderdash-abci 0.14.0-dev.9 ([#1782](https://github.com/dashpay/platform/issues/1782))


### Continuous Integration

* enforce warnings as errors ([#1783](https://github.com/dashpay/platform/issues/1783))
* update doc build branch in action config ([#1748](https://github.com/dashpay/platform/issues/1748))

## [1.0.0-dev.8](https://github.com/dashpay/platform/compare/v1.0.0-dev.7...v1.0.0-dev.8) (2024-03-14)


### ⚠ BREAKING CHANGES

* **platform:** identity update can not disable a key it is also adding (#1772)
* **platform:** key disabled at based on state transition block time (#1771)
* **platform:** data contract validation improvements (#1768)
* update tenderdash to 0.14-dev.4 (#1770)
* **platform:** advanced data contract structure validation position (#1763)

### Features

* **platform:** identity update can not disable a key it is also adding ([#1772](https://github.com/dashpay/platform/issues/1772))
* **platform:** key disabled at based on state transition block time ([#1771](https://github.com/dashpay/platform/issues/1771))


### Bug Fixes

* **platform:** advanced data contract structure validation position ([#1763](https://github.com/dashpay/platform/issues/1763))
* **platform:** data contract validation improvements ([#1768](https://github.com/dashpay/platform/issues/1768))
* **platform:** wrong state used to get current validator set ([#1773](https://github.com/dashpay/platform/issues/1773))
* remove unnecessary clone
* update strategy test document transitions with initial contract ids


### Code Refactoring

* **drive:** relax versioning of calls with fees ([#1762](https://github.com/dashpay/platform/issues/1762))
* drop unused includes; use calculate_sign_hash ([#1767](https://github.com/dashpay/platform/issues/1767))
* resolve various warnings during build or by clippy ([#1761](https://github.com/dashpay/platform/issues/1761))
* strategy test start identities ([#1749](https://github.com/dashpay/platform/issues/1749))


### Miscellaneous Chores

* **dashmate:** upgrade to Core 20.1 ([#1760](https://github.com/dashpay/platform/issues/1760))
* update tenderdash to 0.14-dev.4 ([#1770](https://github.com/dashpay/platform/issues/1770))

## [1.0.0-dev.7](https://github.com/dashpay/platform/compare/v1.0.0-dev.6...v1.0.0-dev.7) (2024-03-08)


### ⚠ BREAKING CHANGES

* **platform:** addded fee increase field to state transitions (#1750)

### Features

* enable random contract creation in strategies ([#1729](https://github.com/dashpay/platform/issues/1729))
* **platform:** state transition fee increase and priorities ([#1750](https://github.com/dashpay/platform/issues/1750))


### Bug Fixes

* **drive:** inconsistent platform state and version during ABCI calls ([#1733](https://github.com/dashpay/platform/issues/1733))
* **drive:** internal error on querying specific identity keys ([#1728](https://github.com/dashpay/platform/issues/1728))
* resolve strategy-tests test failures ([#1743](https://github.com/dashpay/platform/issues/1743))


### Documentation

* update and expand mkdocs redirects ([#1740](https://github.com/dashpay/platform/issues/1740))


### Code Refactoring

* **drive:** expose more groveDB internals ([#1739](https://github.com/dashpay/platform/issues/1739))
* reduce cargo clippy warnings ([#1738](https://github.com/dashpay/platform/issues/1738))
* reduce cargo clippy warnings ([#1741](https://github.com/dashpay/platform/issues/1741))
* reduce cargo clippy warnings in rs-dpp ([#1742](https://github.com/dashpay/platform/issues/1742))
* resolve a few clippy warnings in dapi-grpc, rs-drive-proof-verifier, rs-platform-serialization, rs-platform-serialization-derive, rs-platform-value, rs-sdk, strategy-tests ([#1756](https://github.com/dashpay/platform/issues/1756))
* resolve a few clippy warnings in rs-platform-serializaation and rs-platform-value ([#1744](https://github.com/dashpay/platform/issues/1744))
* resolve clippy warnings in rs-dpp ([#1754](https://github.com/dashpay/platform/issues/1754))
* resolve clippy warnings in rs-drive ([#1752](https://github.com/dashpay/platform/issues/1752))
* resolve clippy warnings in rs-drive-abci ([#1755](https://github.com/dashpay/platform/issues/1755))
* resolve clippy warnings in wasm-dpp ([#1753](https://github.com/dashpay/platform/issues/1753))


### Miscellaneous Chores

* fmt ([#1751](https://github.com/dashpay/platform/issues/1751))
* update testnet genesis and core nightly ([#1758](https://github.com/dashpay/platform/issues/1758))

## [1.0.0-dev.6](https://github.com/dashpay/platform/compare/v1.0.0-dev.5...v1.0.0-dev.6) (2024-03-05)


### ⚠ BREAKING CHANGES

* **platform:** identity nonce for Data Contract Create (#1724)

### Features

* add ContractUpdate to used_contract_ids function
* **platform:** identity nonce for Data Contract Create ([#1724](https://github.com/dashpay/platform/issues/1724))
* **sdk:** add fetch_current_with_metadata to ExtendedEpochInfo ([#1708](https://github.com/dashpay/platform/issues/1708))
* **sdk:** fetch with metadata ([#1707](https://github.com/dashpay/platform/issues/1707))
* **sdk:** re-fetch nonce on interval ([#1706](https://github.com/dashpay/platform/issues/1706))


### Bug Fixes

* **drive-abci:** reject reward shares operations ([#1722](https://github.com/dashpay/platform/issues/1722))
* make strategy tests compatible with all networks and platform tui ([#1705](https://github.com/dashpay/platform/issues/1705))
* **sdk:** nonce manager caching bug ([#1711](https://github.com/dashpay/platform/issues/1711))
* **test-suite:** masternode identities ([#1709](https://github.com/dashpay/platform/issues/1709))


### Performance Improvements

* query and check tx parallel processing ([#1694](https://github.com/dashpay/platform/issues/1694))


### Miscellaneous Chores

* fix npm audit warning ([#1723](https://github.com/dashpay/platform/issues/1723))


### Tests

* **test-suite:** restore dpns tests ([#1725](https://github.com/dashpay/platform/issues/1725))
* **test-suite:** withdrawals identityRecent index ([#1716](https://github.com/dashpay/platform/issues/1716))

## [1.0.0-dev.5](https://github.com/dashpay/platform/compare/v1.0.0-dev.4...v1.0.0-dev.5) (2024-02-20)

### ⚠ BREAKING CHANGES

* Identity nonce and identity contract nonces to prevent replay attacks (#1681)
* Improved check tx verification (#1681)
* Do not allow creating data contracts without documents (#1675)

### Features

* State transitions support in rust sdk (#1596)
* Mempool uniqueness by state transition identifiers (#1681)
* Remove ability to verify proofs from drive-abci in order to have a smaller package size and to reduce potential issues (#1699)
* Tenderdash mempool cache size config option (#1702)

### Bug Fixes

* Remove min core fee per byte check (#1690)
* Fix proof balance and revision proofs for IdentityTopUp (#1678)
* NPM IP package vulnerable to SSRF attacks (#1703)
* Fix for contract proofs (#1699)

### Miscellaneous Chores
* Autogenerated clippy refactoring and fixes for rust version 1.76 (#1691)
* Bump protoc to 25.2 (#1692)

## [1.0.0-dev.4](https://github.com/dashpay/platform/compare/v1.0.0-dev.3...v1.0.0-dev.4) (2024-02-07)


### ⚠ BREAKING CHANGES

* The state now contains information about chain lock quorums (#1621)
* Minimal asset lock amount is introduced that makes previous data invalid (#1667)
* The initial state is changed (#1601)


### Features

* chainlock optimized verification ([#1621](https://github.com/dashpay/platform/issues/1621))
* **drive:** validate asset lock proof minimal value ([#1667](https://github.com/dashpay/platform/issues/1667))
* **drive:** withdrawals finalization ([#1601](https://github.com/dashpay/platform/issues/1601))


### Bug Fixes

* **dashmate:** service status when tenderdash is syncing ([#1682](https://github.com/dashpay/platform/issues/1682))
* **drive:** invalid protocol version is using to deserialize state ([#1679](https://github.com/dashpay/platform/issues/1679))


### Miscellaneous Chores

* **dashmate:** update to core v20.1.0-devpr5806.a1814ce2 ([#1665](https://github.com/dashpay/platform/issues/1665))
* system data contracts versioning ([#1676](https://github.com/dashpay/platform/issues/1676))
* update rs-tenderdash-abci to v0.14.0-dev.6 and tenderdash to v0.14.0-dev.2 ([#1686](https://github.com/dashpay/platform/issues/1686))

## [1.0.0-dev.3](https://github.com/dashpay/platform/compare/v1.0.0-dev.2...v1.0.0-dev.3) (2024-01-16)


### Bug Fixes

* **dapi:** internal errors if broadcasting failed ([#1673](https://github.com/dashpay/platform/issues/1673))

## [1.0.0-dev.2](https://github.com/dashpay/platform/compare/v1.0.0-dev.1...v1.0.0-dev.2) (2024-01-12)


### ⚠ BREAKING CHANGES

* **dashmate:** adjust consensus params and enable re-check (#1669)
* **drive-abci:** internal error if vote extension block is already committed (#1663)

### Bug Fixes

* broadcasting already invalidated transitions ([#1668](https://github.com/dashpay/platform/issues/1668))
* **dashmate:** dapi kills host machine on container stop ([#1670](https://github.com/dashpay/platform/issues/1670))
* **drive-abci:** internal error if vote extension block is already committed ([#1663](https://github.com/dashpay/platform/issues/1663))


### Miscellaneous Chores

* **dashmate:** adjust consensus params and enable re-check ([#1669](https://github.com/dashpay/platform/issues/1669))
* **drive-abci:** fix state transition logging ([#1664](https://github.com/dashpay/platform/issues/1664))
* various logging improvements ([#1666](https://github.com/dashpay/platform/issues/1666))

## [1.0.0-dev.1](https://github.com/dashpay/platform/compare/v0.25.21...v1.0.0-dev.1) (2024-01-11)


### ⚠ BREAKING CHANGES

* invalid state transitions now included into blocks that makes previous chain data invalid. (#1657)
* credit transfer state transition requires revision. (#1634)
* calculated fee amounts are changed (#1656)

### Features

* **drive-abci:** include invalid state transitions into block ([#1657](https://github.com/dashpay/platform/issues/1657))
* **drive-abci:** processing fees for signature verification ([#1656](https://github.com/dashpay/platform/issues/1656))
* **drive-abci:** remove unnecessary validation from check tx and re-check ([#1647](https://github.com/dashpay/platform/issues/1647))
* **sdk:** rs-sdk fetch current epoch ([#1604](https://github.com/dashpay/platform/issues/1604))


### Bug Fixes

* **platform:** credit transfer replay attack ([#1634](https://github.com/dashpay/platform/issues/1634))
* **dapi**: internal error when mempool is full ([#1661](https://github.com/dashpay/platform/issues/1661))


### Miscellaneous Chores

* automatic clippy fixes ([#1528](https://github.com/dashpay/platform/issues/1528), [#1602](https://github.com/dashpay/platform/issues/1602))


### [0.25.22](https://github.com/dashpay/platform/compare/v0.25.21...v0.25.22) (2024-01-19)


### Bug Fixes

* **dashmate:** dapi kills host machine on container stop ([#1670](https://github.com/dashpay/platform/issues/1670))

### [0.25.21](https://github.com/dashpay/platform/compare/v0.25.20...v0.25.21) (2023-12-28)


### Bug Fixes

* corrupted credits not balanced error ([#1650](https://github.com/dashpay/platform/issues/1650))

### [0.25.20](https://github.com/dashpay/platform/compare/v0.25.19...v0.25.20) (2023-12-21)


### Features

* **dashmate:** more config options for envoy and tenderdash ([#1643](https://github.com/dashpay/platform/issues/1643))


### Bug Fixes

* **drive-abci:** rare process proposal when we prepared tx change ([#1645](https://github.com/dashpay/platform/issues/1645))

### [0.25.19](https://github.com/dashpay/platform/compare/v0.25.18...v0.25.19) (2023-12-19)


### Features

* **dashmate:** add config render command ([#1637](https://github.com/dashpay/platform/issues/1637))


### Bug Fixes

* **drive**: filtering of failed and exceeding limit transactions ([#1639](https://github.com/dashpay/platform/issues/1639))
* runtime error: index out of range 28 with length 28

### Reverts

* **drive:** temporary disable credit transfer transitions ([#1642](https://github.com/dashpay/platform/issues/1642))


### [0.25.18](https://github.com/dashpay/platform/compare/v0.25.17...v0.25.18) (2023-12-12)


### Bug Fixes

* **drive:** temporary disable credit transfer transitions ([#1635](https://github.com/dashpay/platform/issues/1635))

### [0.25.17](https://github.com/dashpay/platform/compare/v0.25.16...v0.25.17) (2023-12-11)

Increment version to overcome already published Dash SDK 3.25.16

### [0.25.16](https://github.com/dashpay/platform/compare/v0.25.15...v0.25.16) (2023-12-06)

### ⚠ BREAKING CHANGES

* **dashmate:** removed `docker.network.bindIp`, please use specific host configuration option (#1630)
* data contracts require position to be defined for object properties
* previously created state is not compatible
* upgrade to Node.JS v20 (#1562)


### Features

* **platform:** document serialization across versions ([#1594](https://github.com/dashpay/platform/issues/1594))
* **dashmate:** configure service listening interfaces ([#1630](https://github.com/dashpay/platform/issues/1630))
* **dashmate:** allow dashmate to update minor core versions ([#1619](https://github.com/dashpay/platform/issues/1619))
* **dashmate:** insight, core block explorer ([#1581](https://github.com/dashpay/platform/issues/1581))
* **dashmate:** update core version to 20.0.1 ([#1588](https://github.com/dashpay/platform/issues/1588))
* **dashmate:** upgrade dashmate to ESM syntax ([#1562](https://github.com/dashpay/platform/issues/1562))
* **package:** bump rust packages
* **sdk:** rs-sdk support for ExtendedEpochInfo::fetch()/fetch_many() ([#1576](https://github.com/dashpay/platform/issues/1576))


### Bug Fixes

* **dashmate:** http API calls fail with `command not found error` ([#1600](https://github.com/dashpay/platform/issues/1600))
* **dapi:** tonik streams hang until first message sent ([#1605](https://github.com/dashpay/platform/issues/1605))
* **dashmate:** missing choices enumerator ([#1595](https://github.com/dashpay/platform/issues/1595))
* **dashmate:** incorrect imports  ([#1591](https://github.com/dashpay/platform/issues/1591))
* drive produces wrong app hash after restart ([#1586](https://github.com/dashpay/platform/issues/1586))
* **dapi:** failure if bloom filter is not set ([#1572](https://github.com/dashpay/platform/issues/1572))
* **dashmate:** incorrect block height color on null remote block height ([#1587](https://github.com/dashpay/platform/issues/1587))

### Performance Improvements

* **dapi:** run a cluster of processes ([#1627](https://github.com/dashpay/platform/issues/1627))


### Build System

* use two faucets for parallel test suite run ([#1615](https://github.com/dashpay/platform/issues/1615))


### Miscellaneous Chores

* **dapi:** logging of the wait for state transition result logic ([#1628](https://github.com/dashpay/platform/issues/1628))
* **dashmate:** update tenderdash to 0.13.4 ([#1631](https://github.com/dashpay/platform/issues/1631))
* remove unused dependencies of rust crates ([#1578](https://github.com/dashpay/platform/issues/1578))
* upgrade to Yarn v4 ([#1562](https://github.com/dashpay/platform/issues/1562))
* upgrade to Node.JS v20 ([#1562](https://github.com/dashpay/platform/issues/1562))
* **dashmate:** remove `platform feature-flag` command ([#1616](https://github.com/dashpay/platform/issues/1616))
* **dashmate:** remove unnecessary WASM DPP ([#1607](https://github.com/dashpay/platform/issues/1607))
* **dashmate:** add platform testnet-37 support ([#1603](https://github.com/dashpay/platform/issues/1603))


### Tests

* **dashmate:** add `dashmate update` unit test ([#1609](https://github.com/dashpay/platform/issues/1609))


### Continuous Integration

* arch dependant yarn unplugged cache ([#1580](https://github.com/dashpay/platform/issues/1580))
* rs-sdk, feature and unnecessary deps testing for rust packages ([#1575](https://github.com/dashpay/platform/issues/1575))


### [0.25.16-rc.6](https://github.com/dashpay/platform/compare/v0.25.16-rc.5...v0.25.16-rc.6) (2023-12-06)


### Features

* **dashmate:** allow dashmate to update minor core versions ([#1619](https://github.com/dashpay/platform/issues/1619))

### Bug Fixes

* **dashmate:** http API calls fail with `command not found error` ([#1600](https://github.com/dashpay/platform/issues/1600))

### Tests

* **dashmate:** add `dashmate update` unit test ([#1609](https://github.com/dashpay/platform/issues/1609))

### Miscellaneous Chores

* **dashmate:** remove `platform feature-flag` command ([#1616](https://github.com/dashpay/platform/issues/1616))
* **dashmate:** remove unnecessary WASM DPP ([#1607](https://github.com/dashpay/platform/issues/1607))

### [0.25.16-rc.5](https://github.com/dashpay/platform/compare/v0.25.16-rc.4...v0.25.16-rc.5) (2023-11-29)


### Bug Fixes

* **dapi:** tonik streams hang until first message sent ([#1605](https://github.com/dashpay/platform/issues/1605))


### Miscellaneous Chores

* **dashmate:** add platform testnet-37 support ([#1603](https://github.com/dashpay/platform/issues/1603))


### [0.25.16-rc.4](https://github.com/dashpay/platform/compare/v0.25.16-rc.3...v0.25.16-rc.4) (2023-11-24)


### ⚠ BREAKING CHANGES

* data contracts require position to be defined for object properties
* previously created state is not compatible

### Features

* **platform:** document serialization across versions ([#1594](https://github.com/dashpay/platform/issues/1594))


### Bug Fixes

* drive produces wrong app hash after restart ([#1586](https://github.com/dashpay/platform/issues/1586))


### [0.25.16-rc.3](https://github.com/dashpay/platform/compare/v0.25.16-rc.2...v0.25.16-rc.3) (2023-11-23)

### Bug Fixes

* **dashmate:** missing choices enumerator ([#1595](https://github.com/dashpay/platform/issues/1595))

### [0.25.16-rc.2](https://github.com/dashpay/platform/compare/v0.25.16-rc.1...v0.25.16-rc.2) (2023-11-22)

### Bug Fixes

* **dashmate:** incorrect imports  ([#1591](https://github.com/dashpay/platform/issues/1591))

### [0.25.16-rc.1](https://github.com/dashpay/platform/compare/v0.25.15...v0.25.16-rc.1) (2023-11-21)

### ⚠ BREAKING CHANGES

* upgrade to Node.JS v20 (#1562)

### Features

* **dashmate:** insight, core block explorer ([#1581](https://github.com/dashpay/platform/issues/1581))
* **dashmate:** update core version to 20.0.1 ([#1588](https://github.com/dashpay/platform/issues/1588))
* **dashmate:** upgrade dashmate to ESM syntax ([#1562](https://github.com/dashpay/platform/issues/1562))
* **package:** bump rust packages
* **sdk:** rs-sdk support for ExtendedEpochInfo::fetch()/fetch_many() ([#1576](https://github.com/dashpay/platform/issues/1576))

### Bug Fixes

* **dapi:** failure if bloom filter is not set ([#1572](https://github.com/dashpay/platform/issues/1572))
* **dashmate:** incorrect block height color on null remote block height ([#1587](https://github.com/dashpay/platform/issues/1587))


### Continuous Integration

* arch dependant yarn unplugged cache ([#1580](https://github.com/dashpay/platform/issues/1580))
* rs-sdk, feature and unnecessary deps testing for rust packages ([#1575](https://github.com/dashpay/platform/issues/1575))

### Miscellaneous Chores

* remove unused dependencies of rust crates ([#1578](https://github.com/dashpay/platform/issues/1578))
* upgrade to Yarn v4 ([#1562](https://github.com/dashpay/platform/issues/1562))
* upgrade to Node.JS v20 ([#1562](https://github.com/dashpay/platform/issues/1562))

### [0.25.15](https://github.com/dashpay/platform/compare/v0.25.13...v0.25.15) (2023-11-05)

### ⚠ BREAKING CHANGES

* dpp: allow only one document transition ([#1555](https://github.com/dashpay/platform/issues/1555))

### Continuous Integration
* remove unused and vulnerable github-api package ([#1571](https://github.com/dashpay/platform/issues/1571))
* bump rust packages versions in the release script by ([#1573](https://github.com/dashpay/platform/issues/1573))
* speed up workflows and reduce costs ([#1545](https://github.com/dashpay/platform/issues/1545))

### Features
* rust software development kit for Dash Platform ([#1475](https://github.com/dashpay/platform/issues/1475))

### [0.25.13](https://github.com/dashpay/platform/compare/v0.25.12...v0.25.13) (2023-11-05)


### Bug Fixes

* **dashmate:** config is not persisted after migration ([#1561](https://github.com/dashpay/platform/issues/1561))

### [0.25.12](https://github.com/dashpay/platform/compare/v0.25.11...v0.25.12) (2023-11-03)


### Code Refactoring

* rename protocol version endpoints ([#1552](https://github.com/dashpay/platform/issues/1552))


### Tests

* **dashmate:** fix migrations test ([#1547](https://github.com/dashpay/platform/issues/1547))


### Miscellaneous Chores

* **dashmate:** testnet-35 support ([#1559](https://github.com/dashpay/platform/issues/1559))
* **dashmate:** update tenderdash to v0.13.3

### [0.25.11](https://github.com/dashpay/platform/compare/v0.25.10...v0.25.11) (2023-11-02)


### Bug Fixes

* **dashmate:** config commands output ([#1556](https://github.com/dashpay/platform/issues/1556))

### [0.25.10](https://github.com/dashpay/platform/compare/v0.25.9...v0.25.10) (2023-11-02)

### Bug Fixes

* **dashmate:** invalid SSL path in the obtain command ([#1553](https://github.com/dashpay/platform/issues/1553))



### [0.25.9](https://github.com/dashpay/platform/compare/v0.25.8...v0.25.9) (2023-11-02)


### ⚠ BREAKING CHANGES

* DAPI proto messages are changed (#1542)
* Consensus rules for Identity Create and TopUp are changed. (#1510)
* Asset Lock Proof structure is changed  (#1510)
* Identity Create Transition balance validation requires correct amount (twice more) (#1510)
* **dashmate:** `enabledCount` is replaced with `masternodeEnabled` in the masternode status output (#1505)
* **dashmate:** SSL keys are now stored in the node's config subdirectory (#1502)

### Features

* **dashmate:** add cli command for core service ([#1501](https://github.com/dashpay/platform/issues/1501))
* **dashmate:** configure dashd command arguments ([#1520](https://github.com/dashpay/platform/issues/1520))
* **dashmate:** docker build command ([#1546](https://github.com/dashpay/platform/issues/1546))
* **dashmate:** docker build command ([#1546](https://github.com/dashpay/platform/issues/1546))
* **dashmate:** move ssl dir ([#1502](https://github.com/dashpay/platform/issues/1502))
* identity funding with asset lock special transactions ([#1510](https://github.com/dashpay/platform/issues/1510))
* **platform:** proto message and query versioning ([#1522](https://github.com/dashpay/platform/issues/1522))
* **platform:** query version upgrade (both votes and status) and epoch info ([#1542](https://github.com/dashpay/platform/issues/1542))


### Bug Fixes

* **dashmate:** payment queue and extend enabled count ([#1505](https://github.com/dashpay/platform/issues/1505))
* **dashmate:** reset command doesn't reset configs ([#1541](https://github.com/dashpay/platform/issues/1541))
* **drive:** mishandling internal errors as validation ones ([#1492](https://github.com/dashpay/platform/issues/1492))
* security advisories in browserify-sign and crypto-js ([#1548](https://github.com/dashpay/platform/issues/1548))


### Performance Improvements

* **dashmate:** disable config auto render ([#1499](https://github.com/dashpay/platform/issues/1499))
* **dashmate:** speedup container cleanup ([#1518](https://github.com/dashpay/platform/issues/1518))


### Continuous Integration

* fix s3 layer cache access forbidden on 8x runners ([#1521](https://github.com/dashpay/platform/issues/1521))


### Tests

* **dashmate:** successful migration test ([#1500](https://github.com/dashpay/platform/issues/1500))
* **test-suite:** add optional bail ([#1488](https://github.com/dashpay/platform/issues/1488))


### Build System

* update rust to 1.73 ([#1529](https://github.com/dashpay/platform/issues/1529))


### Code Refactoring

* remove js-dpp ([#1517](https://github.com/dashpay/platform/issues/1517))


### Documentation

* Update README.md to give information about correctly adding to correct Shell $PATH ([#1550](https://github.com/dashpay/platform/issues/1550))


### Miscellaneous Chores

* adds missing crypto js ([#1538](https://github.com/dashpay/platform/issues/1538))
* **dashmate:** set tenderdash logging level to info ([#1540](https://github.com/dashpay/platform/issues/1540))
* **dpp:** version `InstantAssetLockProof.validate_structure` ([#1549](https://github.com/dashpay/platform/issues/1549))

### [0.25.8](https://github.com/dashpay/platform/compare/v0.25.7...v0.25.8) (2023-10-20)


### Bug Fixes

* **drive-abci:** cached protocol versions ([#1516](https://github.com/dashpay/platform/issues/1516))


### Miscellaneous Chores

* **drive:** remove runtime state logging ([#1511](https://github.com/dashpay/platform/issues/1511))
* **drive:** set correct target for grove logs ([#1512](https://github.com/dashpay/platform/issues/1512))


### Continuous Integration

* c6id.8xlarge runners ([#1514](https://github.com/dashpay/platform/issues/1514))

### [0.25.7](https://github.com/dashpay/platform/compare/v0.25.6...v0.25.7) (2023-10-18)


### Miscellaneous Chores

* **drive:** add more logs ([#1506](https://github.com/dashpay/platform/issues/1506))

### [0.25.6](https://github.com/dashpay/platform/compare/v0.25.5...v0.25.6) (2023-10-18)


### ⚠ BREAKING CHANGES

* **drive:** remove extended quorum info from platform state (#1496)

### Miscellaneous Chores

* **drive:** fix logging levels ([#1495](https://github.com/dashpay/platform/issues/1495))
* **drive:** remove extended quorum info from platform state ([#1496](https://github.com/dashpay/platform/issues/1496))
* logging hex strings ([#1497](https://github.com/dashpay/platform/issues/1497))

### [0.25.3](https://github.com/dashpay/platform/compare/v0.25.2...v0.25.3) (2023-10-12)


### Bug Fixes

* **dashmate:** a testnet node fails to sync ([#1485](https://github.com/dashpay/platform/issues/1485))


### Documentation

* cleanup changelog

### [0.25.2](https://github.com/dashpay/platform/compare/v0.25.1...v0.25.2) (2023-10-11)


### Features

* **dashmate:** force start ([#1481](https://github.com/dashpay/platform/issues/1481))

### [0.25.1](https://github.com/dashpay/platform/compare/v0.25.0...v0.25.1) (2023-10-11)


### Bug Fixes

* **dashmate:** log permissions on linux system ([#1479](https://github.com/dashpay/platform/issues/1479))

## [0.25.0](https://github.com/dashpay/platform/compare/v0.25.0-dev.33...v0.25.0) (2023-10-10)


### ⚠ BREAKING CHANGES

* block results might be different while blockchain replying (#1464)
* **platform:** the default epoch length is changed (#1467)
* **drive-abci:** masternode operator keys are no longer added to the unique tree lookup (#1459)
* **drive:** contracts with arrays won't be valid anymore (#1457)
* **drive-abci:** small differences in serialization of Null value (#1456)
* the DPNS system data contract is changed so the previously created state won't be valid (#1454)
* drive volume is changed so volumes and containers must be recreated. It means platform file must be wiped (#1406)

### Features

* **dashmate:** add epochTime to abci config ([#1468](https://github.com/dashpay/platform/issues/1468))
* **dashmate:** tenderdash log file ([#1396](https://github.com/dashpay/platform/issues/1396))
* mitigate homograph attack in DPNS ([#1454](https://github.com/dashpay/platform/issues/1454))
* **platform:** configurable epoch time (default down to 9.125 days) ([#1467](https://github.com/dashpay/platform/issues/1467))


### Bug Fixes

* consensus error is missing in tx results ([#1458](https://github.com/dashpay/platform/issues/1458))
* **dashmate:** drive logs configuration ([#1406](https://github.com/dashpay/platform/issues/1406))
* **drive-abci:** double state transition with document same unique index ([#1456](https://github.com/dashpay/platform/issues/1456))
* **drive-abci:** masternode identities unique keys ([#1459](https://github.com/dashpay/platform/issues/1459))
* **drive:** deadlock in tenderdash abci client ([#1463](https://github.com/dashpay/platform/issues/1463))
* **drive:** document type doesn't match array value ([#1457](https://github.com/dashpay/platform/issues/1457))
* incorrect invalidation of data contract cache ([#1464](https://github.com/dashpay/platform/issues/1464))


### Documentation

* **dashmate:** typos in README ([#1256](https://github.com/dashpay/platform/issues/1256))


### Continuous Integration

* add missing Drive workflow ([#1461](https://github.com/dashpay/platform/issues/1461))
* disable runs on draft prs ([#1380](https://github.com/dashpay/platform/issues/1380))
* update action dependency versions ([#1449](https://github.com/dashpay/platform/issues/1449))


### Miscellaneous Chores

* add a breaking changes checkbox to the PR template ([#1455](https://github.com/dashpay/platform/issues/1455))
* **dashmate:** update config for testnet-26 ([#1470](https://github.com/dashpay/platform/issues/1470))
* **dashmate:** update core image ([#1469](https://github.com/dashpay/platform/issues/1469))
* **drive:** improve quorum info update logs ([#1444](https://github.com/dashpay/platform/issues/1444))
* **drive:** log grovedb operations ([#1446](https://github.com/dashpay/platform/issues/1446))
* increase scopes for ci ([#1460](https://github.com/dashpay/platform/issues/1460))
* removed old code ([#1471](https://github.com/dashpay/platform/issues/1471))


## [0.25.0-dev.33](https://github.com/dashpay/platform/compare/v0.25.0-dev.32...v0.25.0-dev.33) (2023-10-05)


### Features

* **dashmate:** upgrade core to v20.0.0-beta.2 ([#1436](https://github.com/dashpay/platform/issues/1436))
* **drive:** handlers error codes ([#1394](https://github.com/dashpay/platform/issues/1394))
* **sdk:** add logger to dapi client ([#1420](https://github.com/dashpay/platform/issues/1420))


### Continuous Integration

* **test-suite:** run sdk and wallet functional tests from all packages workflow ([#1438](https://github.com/dashpay/platform/issues/1438))

## [0.25.0-dev.32](https://github.com/dashpay/platform/compare/v0.25.0-dev.31...v0.25.0-dev.32) (2023-09-29)


### Performance Improvements

* **drive:** clear instead of delete for previous masternode version voting ([#1437](https://github.com/dashpay/platform/issues/1437))


### Miscellaneous Chores

* update config and migrations for testnet-25 ([#1435](https://github.com/dashpay/platform/issues/1435))

## [0.25.0-dev.31](https://github.com/dashpay/platform/compare/v0.25.0-dev.30...v0.25.0-dev.31) (2023-09-28)


### ⚠ BREAKING CHANGES

* **drive:** a unique key with that hash already exists (#1429)

### Features

* **drive:** drive-abci verify grovedb CLI ([#1427](https://github.com/dashpay/platform/issues/1427))


### Bug Fixes

* **drive:** a unique key with that hash already exists ([#1429](https://github.com/dashpay/platform/issues/1429))

## [0.25.0-dev.30](https://github.com/dashpay/platform/compare/v0.25.0-dev.29...v0.25.0-dev.30) (2023-09-28)

### Bug Fixes

* **drive:** update grovedb after just in time fix ([#1426](https://github.com/dashpay/platform/issues/1426))
* **drive:** non-deterministic extended quorum info ([#1425](https://github.com/dashpay/platform/issues/1425))

### Security Threats

* **drive:** chaijs/get-func-name vulnerable to ReDoS ([#1431](https://github.com/dashpay/platform/issues/1431))

### Miscellaneous Chores

* update seed ip addresses ([#1424](https://github.com/dashpay/platform/issues/1424))
* update s3 bucket ([#1430](https://github.com/dashpay/platform/issues/1430))


## [0.25.0-dev.29](https://github.com/dashpay/platform/compare/v0.25.0-dev.28...v0.25.0-dev.29) (2023-09-22)
### Features

* **dashmate:** expose tenderdash metics ([#1419](https://github.com/dashpay/platform/issues/1419))


### Bug Fixes

* **dapi:** getTransaction google-protobuf AssertionError ([#1416](https://github.com/dashpay/platform/issues/1416))
* **dashmate:** platform reset failure ([#1415](https://github.com/dashpay/platform/issues/1415))


### Miscellaneous Chores

* **dashmate:** add images migrations for v25 ([#1377](https://github.com/dashpay/platform/issues/1377))
* update testnet genesis config for testnet-24 ([#1413](https://github.com/dashpay/platform/issues/1413))


### Continuous Integration

* make self-hosted actions-cache steps access S3 from correct region ([#1407](https://github.com/dashpay/platform/issues/1407))
* update cache target bucket ([#1418](https://github.com/dashpay/platform/issues/1418))

## [0.25.0-dev.28](https://github.com/dashpay/platform/compare/v0.25.0-dev.27...v0.25.0-dev.28) (2023-09-19)


### Bug Fixes

* **dashmate:** config format is not conventional ([#1410](https://github.com/dashpay/platform/issues/1410))
* **dashmate:** ssl verification container already exists ([#1409](https://github.com/dashpay/platform/issues/1409))
* **drive:** withdrawal transactions query ([#1402](https://github.com/dashpay/platform/issues/1402))


### Styles

* **dpp:** remove unused cbor import ([#1384](https://github.com/dashpay/platform/issues/1384))

## [0.25.0-dev.27](https://github.com/dashpay/platform/compare/v0.25.0-dev.26...v0.25.0-dev.27) (2023-09-18)


### Bug Fixes

* **dashmate:** obtain new certificates with helper failure ([#1403](https://github.com/dashpay/platform/issues/1403))

## [0.25.0-dev.26](https://github.com/dashpay/platform/compare/v0.25.0-dev.25...v0.25.0-dev.26) (2023-09-16)

### Reverted

* drive file logs ([#1400]((https://github.com/dashpay/platform/pull/1400)))


## [0.25.0-dev.25](https://github.com/dashpay/platform/compare/v0.25.0-dev.24...v0.25.0-dev.25) (2023-09-15)

### Reverted

* make actions-cache steps access S3 using the correct region ([#1398](https://github.com/dashpay/platform/pull/1398))


## [0.25.0-dev.24](https://github.com/dashpay/platform/compare/v0.25.0-dev.23...v0.25.0-dev.24) (2023-09-15)


### Bug Fixes

* drive file logs ([#1395](https://github.com/dashpay/platform/issues/1395))


### Continuous Integration

* disable apt install step on macos release job ([#1390](https://github.com/dashpay/platform/issues/1390))
* fix cache mtime ([#1385](https://github.com/dashpay/platform/issues/1385))
* make actions-cache steps access S3 using the correct region ([#1391](https://github.com/dashpay/platform/issues/1391))


### Miscellaneous Chores

* **dashmate:** support new docker version ([#1393](https://github.com/dashpay/platform/issues/1393))
* update tenderdash to v0.13.1 ([#1392](https://github.com/dashpay/platform/issues/1392))

## [0.25.0-dev.23](https://github.com/dashpay/platform/compare/v0.25.0-dev.22...v0.25.0-dev.23) (2023-09-11)


### Bug Fixes

* **drive:** vote extensions are allowed only for the current block and round ([#1387](https://github.com/dashpay/platform/issues/1387))


### Continuous Integration

* macos dashmate build broken due to bad qemu install ([#1374](https://github.com/dashpay/platform/issues/1374))

## [0.25.0-dev.22](https://github.com/dashpay/platform/compare/v0.25.0-dev.21...v0.25.0-dev.22) (2023-09-08)


### Features

* **drive:** improve ABCI logging ([#1382](https://github.com/dashpay/platform/issues/1382))
* support a new dashcore version ([#1368](https://github.com/dashpay/platform/issues/1368))


### Bug Fixes

* **drive:** -32603 error code on broadcast ([#1381](https://github.com/dashpay/platform/issues/1381))
* **drive:** query fix for 1 value and no where clause ([#1378](https://github.com/dashpay/platform/issues/1378))

## [0.25.0-dev.21](https://github.com/dashpay/platform/compare/v0.25.0-dev.20...v0.25.0-dev.21) (2023-09-07)

## [0.25.0-dev.20](https://github.com/dashpay/platform/compare/v0.25.0-dev.19...v0.25.0-dev.20) (2023-09-06)


### Features

* **drive:** better error handling on check_tx ([#1372](https://github.com/dashpay/platform/issues/1372))


### Bug Fixes

* **drive:** deserialization and identity not existing on check_tx ([#1371](https://github.com/dashpay/platform/issues/1371))

## [0.25.0-dev.19](https://github.com/dashpay/platform/compare/v0.25.0-dev.18...v0.25.0-dev.19) (2023-09-06)


### Bug Fixes

* **dpp:** some contract options are updatable ([#1364](https://github.com/dashpay/platform/issues/1364))
* **drive:** invalid mn operator reward type ([#1366](https://github.com/dashpay/platform/issues/1366))

## [0.25.0-dev.18](https://github.com/dashpay/platform/compare/v0.25.0-dev.17...v0.25.0-dev.18) (2023-09-04)


### Features

* contract specified encryption/decryption keys (stage 1 contract bounds) ([#1358](https://github.com/dashpay/platform/issues/1358))


### Bug Fixes

* **dashmate:** helper container is restarting ([#1362](https://github.com/dashpay/platform/issues/1362))


### Miscellaneous Chores

* **dashmate:** bump core to 20.0.0-alpha.6 ([#1361](https://github.com/dashpay/platform/issues/1361))
* **dashmate:** remove sentinel service ([#1354](https://github.com/dashpay/platform/issues/1354))

## [0.25.0-dev.17](https://github.com/dashpay/platform/compare/v0.25.0-dev.16...v0.25.0-dev.17) (2023-08-31)


### ⚠ BREAKING CHANGES

* Some of the WASM DPP methods are disabled
* DataContract methods are renamed
* Raw data contract structure is changed


### Features

* add better JsonSchemaError messages ([#1341](https://github.com/dashpay/platform/issues/1341))
* validate with document type ([#1334](https://github.com/dashpay/platform/issues/1334))
* code versioning ([#1327](https://github.com/dashpay/platform/issues/1327))


### Bug Fixes

* dpp and drive-abci fail to build without default-features ([#1345](https://github.com/dashpay/platform/issues/1345))
* **dpp:** data contract facade and state transition bindings ([#1342](https://github.com/dashpay/platform/issues/1342))
* **dpp:** fixing identity transition bindings ([#1315](https://github.com/dashpay/platform/issues/1315))
* **dpp:** identity constructor ([#1336](https://github.com/dashpay/platform/issues/1336))
* **dpp:** identity facade ([#1329](https://github.com/dashpay/platform/issues/1329))
* **dpp:** wasm binding for DataContract ([#1333](https://github.com/dashpay/platform/issues/1333))
* network start for querying ([#1335](https://github.com/dashpay/platform/issues/1335))
* various fixes and todos for [#1334](https://github.com/dashpay/platform/issues/1334) ([#1337](https://github.com/dashpay/platform/issues/1337))
* wasm DPP binding and other ([#1352](https://github.com/dashpay/platform/issues/1352))


### Continuous Integration

* fix credentials could not be loaded error ([#1320](https://github.com/dashpay/platform/issues/1320))


### Code Refactoring

* rename serialize ([#1338](https://github.com/dashpay/platform/issues/1338))


### Tests

* **dpp:** temporary skip wasm-dpp tests ([#1328](https://github.com/dashpay/platform/issues/1328))
* enable data contract tests ([#1346](https://github.com/dashpay/platform/issues/1346))
* restore identity create ([#1339](https://github.com/dashpay/platform/issues/1339))


### Miscellaneous Chores

* npm audit fix ([#1321](https://github.com/dashpay/platform/issues/1321))
* **sdk:** temporary disable data contracts and documents ([#1331](https://github.com/dashpay/platform/issues/1331))
* update workflow_dispatch
* upgrade to Node.JS v18 LTS ([#1280](https://github.com/dashpay/platform/issues/1280))
* support Core v0.20.0-alpha.4 ([#1357](https://github.com/dashpay/platform/pull/1357))


### [0.24.23](https://github.com/dashpay/platform/compare/v0.24.22...v0.24.23) (2023-08-18)


### Miscellaneous Chores

* npm audit fix ([#1321](https://github.com/dashpay/platform/issues/1321))

### Continuous Integration

* ci: fix credentials could not be loaded error ([#1321](https://github.com/dashpay/platform/issues/1320))

### [0.24.22](https://github.com/dashevo/platform/compare/v0.24.21...v0.24.22) (2023-08-15)


### Features

* **dashmate:** render tenderdash node mode in the service config ([#1311](https://github.com/dashevo/platform/issues/1311))

### [0.24.21](https://github.com/dashpay/platform/compare/v0.24.20...v0.24.21) (2023-08-09)


### Bug Fixes

* **js-drive** tests after upgrade of wasm-bindgen to version 0.2.86 ([#1306](https://github.com/dashpay/platform/issues/1306))
* **release:** upgrade buildbase with wasm-bindgen 0.2.86 ([#1304](https://github.com/dashpay/platform/issues/1304))

### [0.24.20](https://github.com/dashpay/platform/compare/v0.24.19...v0.24.20) (2023-08-07)


### Bug Fixes

* **dashmate:** `--platform` flag is ignored ([#1287](https://github.com/dashpay/platform/issues/1287))
* **dashmate:** load external ip for evo fullnodes ([#1288](https://github.com/dashpay/platform/issues/1288))
* **dashmate:** missing default values in IP and ports form ([#1276](https://github.com/dashpay/platform/issues/1276))
* **dashmate:** some status errors is visible without DEBUG env ([#1299](https://github.com/dashpay/platform/issues/1299))
* **dashmate:** various status output issues ([#1274](https://github.com/dashpay/platform/issues/1274), [#1293](https://github.com/dashpay/platform/issues/1293))
* **dashmate:** invalid migration version ([#1285](https://github.com/dashpay/platform/issues/1285))


### Continuous Integration

* update self-hosted runner tags ([#1271](https://github.com/dashpay/platform/issues/1271))


### Build System

* update `wasm-bindgen-cli` to 0.2.86 ([#1289](https://github.com/dashpay/platform/issues/1289))


### Miscellaneous Chores

* **dashmate:** update Core to v19.3.0 ([#1284](https://github.com/dashpay/platform/issues/1284))


## [0.25.0-dev.16](https://github.com/dashpay/platform/compare/v0.25.0-dev.15...v0.25.0-dev.16) (2023-08-02)


### ⚠ BREAKING CHANGES

* Platform state is modified so previous data won't be valid

### Features

* abci versioning ([#1172](https://github.com/dashpay/platform/issues/1172))
* adapt GroveDB's API changes ([#1099](https://github.com/dashpay/platform/issues/1099))
* build dapi-gprc rust client ([#1182](https://github.com/dashpay/platform/issues/1182))
* **drive:** add block_id_hash, quorum_type and chain_id required to verify proofs to GRPC responses ([#1207](https://github.com/dashpay/platform/issues/1207))
* **drive:** core block reward distribution ([#1135](https://github.com/dashpay/platform/issues/1135))
* **drive:** ensure that chain lock height valid ([#1157](https://github.com/dashpay/platform/issues/1157))
* **drive:** fetch contract history and contract.is_readonly ([#1120](https://github.com/dashpay/platform/issues/1120))
* **drive:** graceful shutdown ([#1154](https://github.com/dashpay/platform/issues/1154))
* **drive:** verify chain lock Core RPC ([#1146](https://github.com/dashpay/platform/issues/1146))
* **drive:** verify instant lock Core RPC ([#1142](https://github.com/dashpay/platform/issues/1142))
* fetch data contract history endpoint ([#1149](https://github.com/dashpay/platform/issues/1149))
* identity credit transfer state transition ([#1138](https://github.com/dashpay/platform/issues/1138))
* remove bad masternodes from validator sets ([#1160](https://github.com/dashpay/platform/issues/1160))
* **sdk:** retry policy for newly created platform entities ([#1143](https://github.com/dashpay/platform/issues/1143))


### Bug Fixes

* **dapi:** invalid json response body ([#1150](https://github.com/dashpay/platform/issues/1150))
* double process proposal from Tenderdash restart ([#1165](https://github.com/dashpay/platform/issues/1165))
* **drive:** core RPC retry all errors ([#1140](https://github.com/dashpay/platform/issues/1140))
* **drive:** do not return an error for non existence contract when verifying ([#1241](https://github.com/dashpay/platform/issues/1241))
* **drive:** wait for core to sync ([#1153](https://github.com/dashpay/platform/issues/1153))
* update abci test state root ([#1144](https://github.com/dashpay/platform/issues/1144))


### Code Refactoring

* misc changes during v0.25 review ([#1121](https://github.com/dashpay/platform/issues/1121))
* update dapi proto file to use either proofs or result ([#1148](https://github.com/dashpay/platform/issues/1148))


### Tests

* **drive:** verify proof signatures in strategy tests ([#1147](https://github.com/dashpay/platform/issues/1147))


### Documentation

* backport changelog from v0.24
* better drive verify docs ([#1171](https://github.com/dashpay/platform/issues/1171))


### Build System

* fix dash sdk ts config
* remove unnecessary yarn installation


### Miscellaneous Chores

* add QuantumExplorer as Code Owner
* backport deps from master
* temp fix rust-dashcore-dependency
* update grovedb version
* update lock file
* upgrade to tenderdash v0.13 ([#1236](https://github.com/dashpay/platform/issues/1236))


### Continuous Integration

* runs not cancelled when PR is closed or merged ([#1234](https://github.com/dashpay/platform/issues/1234))
* s3 cache ([#1167](https://github.com/dashpay/platform/issues/1167))
* select workflow_id to cancel based on head_ref ([#1247](https://github.com/dashpay/platform/issues/1247))
* specify docker mount cache bucket name as variable ([#1252](https://github.com/dashpay/platform/issues/1252))
* switch to multi-runner stack ([#1268](https://github.com/dashpay/platform/issues/1268))

### [0.24.19](https://github.com/dashpay/platform/compare/v0.24.18...v0.24.19) (2023-07-28)


### Bug Fixes

* **dashmate:** `baseImage/build/context` invalid json schema ([#1269](https://github.com/dashpay/platform/issues/1269))

### [0.24.18](https://github.com/dashpay/platform/compare/v0.24.17...v0.24.18) (2023-07-26)


### Bug Fixes

* **dashmate:** the update command expects helper image option ([#1264](https://github.com/dashpay/platform/issues/1264))

### [0.24.17](https://github.com/dashpay/platform/compare/v0.24.16...v0.24.17) (2023-07-26)


### Features

* **dashmate:** pre-build image ([#1259](https://github.com/dashpay/platform/issues/1259))


### Bug Fixes

* **dashmate:** undefined createIpAndPortsForm factory ([#1258](https://github.com/dashpay/platform/issues/1258))
* **dashmate:** version color is red ([#1255](https://github.com/dashpay/platform/issues/1255))


### Code Refactoring

* **dashmate:** default configuration ([#1257](https://github.com/dashpay/platform/issues/1257))

### [0.24.16](https://github.com/dashpay/platform/compare/v0.24.15...v0.24.16) (2023-07-25)


### ⚠ BREAKING CHANGES

* **dashmate:** Removed `dashmate.helper.docker.image` configuration option (#1231)

### Features

* **dashmate:** allow separation of a service build ([#1206](https://github.com/dashpay/platform/issues/1206))
* **dashmate:** reindex reworked ([#1212](https://github.com/dashpay/platform/issues/1212))


### Bug Fixes

* **dashmate:** invalid testnet chain id ([#1233](https://github.com/dashpay/platform/issues/1233))
* **dashmate:** runs invalid helper version ([#1231](https://github.com/dashpay/platform/issues/1231))
* word wrap audit fail ([#1235](https://github.com/dashpay/platform/issues/1235))


### Miscellaneous Chores

* remove envoy build from dashmate and release ([#1232](https://github.com/dashpay/platform/issues/1232))


### Code Refactoring

* **dashmate:** dynamic home dir path ([#1237](https://github.com/dashpay/platform/issues/1237))
* **dashmate:** move all envs definition to `generateEnvs` ([#1246](https://github.com/dashpay/platform/issues/1246))


### Tests

* **dashmate:** increase the reliability of e2e tests ([#1204](https://github.com/dashpay/platform/issues/1204))

### [0.24.15](https://github.com/dashpay/platform/compare/v0.24.14...v0.24.15) (2023-07-10)


### Features

* **dashmate:** interface binding configuration ([#1220](https://github.com/dashpay/platform/issues/1220))

### Bug Fixes

* **dashmate:** service name variable name ([#1225](https://github.com/dashpay/platform/issues/1225))

### Miscellaneous Chores

* **dashmate:** update tenderdash genesis for testnet ([#1223](https://github.com/dashpay/platform/issues/1223))

### Continuous Integration

* temporary ignore gRPC JS vulnerability ([#1221](https://github.com/dashpay/platform/issues/1221))


### [0.24.14](https://github.com/dashpay/platform/compare/v0.24.13...v0.24.14) (2023-07-05)


### Features

* **dashmate:** report pulled images during update ([#1186](https://github.com/dashpay/platform/issues/1186), [#1213](https://github.com/dashpay/platform/issues/1213))


### Miscellaneous Chores

* **dashmate:** bump Core version to 19.2.0 ([#1211](https://github.com/dashpay/platform/issues/1211))

### [0.24.13](https://github.com/dashpay/platform/compare/v0.24.12...v0.24.13) (2023-06-29)


### Bug Fixes

* **dashmate:** invalid migration ([#1209](https://github.com/dashpay/platform/issues/1209))

### [0.24.12](https://github.com/dashpay/platform/compare/v0.24.11...v0.24.12) (2023-06-28)


### ⚠ BREAKING CHANGES

* **dashmate:** Default dashmate helper port changed from 9000 to 9100 (#1194)

### Features

* **dashmate:** configure tenderdash pprof ([#1201](https://github.com/dashpay/platform/issues/1201))
* **dashmate:** setup masternode with DMT ([#1203](https://github.com/dashpay/platform/issues/1203))


### Bug Fixes

* **dashmate:** helper default port was bound to Windows print port ([#1194](https://github.com/dashpay/platform/issues/1194))


### Code Refactoring

* unwanted usage of x11-hash-js ([#1191](https://github.com/dashpay/platform/issues/1191))


### Miscellaneous Chores

* **drive:** downgrade grovedb to supported version ([#1202](https://github.com/dashpay/platform/issues/1202))


### Tests

* **dashmate:** e2e tests ([#1152](https://github.com/dashpay/platform/issues/1152))

### [0.24.11](https://github.com/dashpay/platform/compare/v0.24.10...v0.24.11) (2023-06-23)


### Bug Fixes

* **drive:** cannot read properties of undefined (reading 'toJSON') ([#1196](https://github.com/dashpay/platform/issues/1196))

### [0.24.10](https://github.com/dashpay/platform/compare/v0.24.9...v0.24.10) (2023-06-23)


### Miscellaneous Chores

* **drive:** disable masternode identities update logic ([#1192](https://github.com/dashpay/platform/issues/1192))

### [0.24.9](https://github.com/dashpay/platform/compare/v0.24.8...v0.24.9) (2023-06-22)


### ⚠ BREAKING CHANGES

* **dashmate:** SSL certificates and other configuration files will be deleted with the hard reset command (#1188)

### Features

* **dashmate:** hard reset deletes related files ([#1188](https://github.com/dashpay/platform/issues/1188))


### Bug Fixes

* **dashmate:** download certificate retry logic ([#1187](https://github.com/dashpay/platform/issues/1187))
* merkle root from the diff doesn't match calculated merkle root ([#1189](https://github.com/dashpay/platform/issues/1189))


### Continuous Integration

* fix cancel runs for already merged PRs ([#1185](https://github.com/dashpay/platform/issues/1185))

### [0.24.8](https://github.com/dashpay/platform/compare/v0.24.7...v0.24.8) (2023-06-21)


### Features

* **dashmate:** re-use ZeroSSL private key ([#1180](https://github.com/dashpay/platform/issues/1180))


### Continuous Integration

* cancel runs for already merged PRs ([#1179](https://github.com/dashpay/platform/issues/1179))

### [0.24.7](https://github.com/dashpay/platform/compare/v0.24.6...v0.24.7) (2023-06-21)


### ⚠ BREAKING CHANGES

* **dashmate:** handle already configured certificate in obtain command (#1176)

### Features

* **dashmate:** handle already configured certificate in obtain command ([#1176](https://github.com/dashpay/platform/issues/1176))

### [0.24.6](https://github.com/dashpay/platform/compare/v0.24.5...v0.24.6) (2023-06-19)


### Features

* **dashmate:** add homedir to compose project name ([#1141](https://github.com/dashpay/platform/issues/1141))


### Bug Fixes

* **dashmate:** config/core/rpc/allowIps must be array ([#881](https://github.com/dashpay/platform/issues/881))
* **sdk:** identifier expects buffer with "in" operator in where query ([#1168](https://github.com/dashpay/platform/issues/1168))
* **sdk:** missing and invalid types ([#1156](https://github.com/dashpay/platform/issues/1156))
* **wallet-lib:** instant locks not arriving to HD wallets ([#1126](https://github.com/dashpay/platform/issues/1126))


### Miscellaneous Chores

* bump up dashd version for mainnet in dashmate ([#1132](https://github.com/dashpay/platform/issues/1132))
* update socket io parser ([#1127](https://github.com/dashpay/platform/issues/1127))

### [0.24.5](https://github.com/dashpay/platform/compare/v0.24.4...v0.24.5) (2023-05-22)


### Features

* **dashmate:** descriptions for all possible ZeroSSL errors ([#1107](https://github.com/dashpay/platform/issues/1107))


### Bug Fixes

* **dashmate:** platform should not be enabled on mainnet ([#1112](https://github.com/dashpay/platform/issues/1112))


### Miscellaneous Chores

* **dashmate:** better port labels for mainnet evolution node setup  ([#1106](https://github.com/dashpay/platform/issues/1106))

### [0.24.4](https://github.com/dashpay/platform/compare/v0.24.3...v0.24.4) (2023-05-18)


### Bug Fixes

* **dashmate:** dashmate helper crashing ([#1072](https://github.com/dashpay/platform/issues/1072))
* **dashmate:** unable to find compatible protocol ([#1102](https://github.com/dashpay/platform/issues/1102))


### Continuous Integration

* add a name to PR linter step ([#1103](https://github.com/dashpay/platform/issues/1103))

### [0.24.3](https://github.com/dashpay/platform/compare/v0.24.2...v0.24.3) (2023-05-16)


### Features

* **dashmate:** `no-retry` flag for ssl obtain command ([#1093](https://github.com/dashpay/platform/issues/1093))


### Miscellaneous Chores

* support GA certificates for testnet ([#1092](https://github.com/dashpay/platform/issues/1092))

### [0.24.2](https://github.com/dashpay/platform/compare/v0.24.1...v0.24.2) (2023-05-16)


### Features

* **dashmate:** obtain SSL certificate command ([#1088](https://github.com/dashpay/platform/issues/1088))
* **dpp:** document `$createdAt` and `$updatedAt` validation ([#948](https://github.com/dashpay/platform/issues/948))


### Bug Fixes

* **dashmate:** status command fails with errors ([#1059](https://github.com/dashpay/platform/issues/1059))

### [0.24.1](https://github.com/dashpay/platform/compare/v0.24.0...v0.24.1) (2023-05-15)


### Features

* **dashmate:** platform flag for start, stop and restart commands ([#1063](https://github.com/dashpay/platform/issues/1063))


### Bug Fixes

* **dapi:** can't connect to testnet with default seeds ([#1084](https://github.com/dashpay/platform/issues/1084))
* **dashmate:** check core is started checks everytime ([#1071](https://github.com/dashpay/platform/issues/1071))
* **dashmate:** incorrect reset command prompt in case network is already set up ([#1064](https://github.com/dashpay/platform/issues/1064))
* **dashmate:** outdated docker images and missed migration  ([#1069](https://github.com/dashpay/platform/issues/1069))


### Miscellaneous Chores

* **dashmate:** rename high-performance nodes to evo nodes ([#1062](https://github.com/dashpay/platform/issues/1062))
* **drive:** payout fees only to single well-known Identity ([#1078](https://github.com/dashpay/platform/issues/1078))

## [0.24.0](https://github.com/dashpay/platform/compare/v0.24.0-dev.34...v0.24.0) (2023-05-10)

### Notes

The masternode identities logic is partially disabled due to incomplete Identity V2 implementation and will be enabled back in v0.25.0


### ⚠ BREAKING CHANGES

* Previous blockchain data and state is not compatible
* Previous created compose projects won't be supported. Please destroy them before update (#1055)
* The --platfrom-only flag is renamed to --platform in the reset command (#991)
* Some wasm-dpp APIs are different to js-dpp ones. The only divergencies that were addressed are the ones that were visible in tests, but some others might've been left intact (#848)
* Core version less than 19 and legacy BLS schema are not supported (#771)
* gRPC and HTTP platform ports now handling with the one single port (#752)
* State Transition fees are changed
* Document's query validation logic has minor incompatibilities with previous version

### Features

* **drive:** whitelist and filter banned nodes for validators ([#1034](https://github.com/dashpay/platform/issues/1034))
* **dashmate:** core log file and debug categories ([#913](https://github.com/dashpay/platform/issues/913))
* **dashamte:** better ZeroSSL error messages ([#950](https://github.com/dashpay/platform/issues/950))
* **dashmate:** set random core rpc usename and password on setup ([#973](https://github.com/dashpay/platform/issues/973))
* **dashmate:** verbose `connect ENOENT /var/run/docker.sock` error ([#951](https://github.com/dashpay/platform/issues/951))
* **wasm-dpp:** state_transition_fee_validator binding and tests ([#874](https://github.com/dashpay/platform/issues/874))
* **dashmate:** check system requirements before setup ([#935](https://github.com/dashpay/platform/issues/935))
* **drive:** handle quorum rotation failure ([#858](https://github.com/dashpay/platform/issues/858))
* wasm-dpp integration ([#848](https://github.com/dashpay/platform/issues/848))
* **dashmate:** build linux tarballs ([#887](https://github.com/dashpay/platform/issues/887))
* **dashmate:** build services before restart ([#894](https://github.com/dashpay/platform/issues/894))
* **dashmate:** exit status with 2 if it's not running ([#896](https://github.com/dashpay/platform/issues/896))
* **dashmate:** implement http json rpc api ([#888](https://github.com/dashpay/platform/issues/888))
* **dashmate:** tenderdash latest block time in status ([#906](https://github.com/dashpay/platform/issues/906))
* **dpp:** serialize consensus errors ([#871](https://github.com/dashpay/platform/issues/871))
* drive verification c bindings ([#860](https://github.com/dashpay/platform/issues/860))
* **dashmate:** add new quroum in dashcore config ([#862](https://github.com/dashpay/platform/issues/862))
* **dashmate:** enable platform option ([#853](https://github.com/dashpay/platform/issues/853))
* **dashmate:** generate self-signed certificates in the `setup` command ([#869](https://github.com/dashpay/platform/issues/869))
* **dashmate:** high-performance nodes registration with `setup` command ([#794](https://github.com/dashpay/platform/issues/794))
* **dashmate:** hint to setup a node on start failure ([#866](https://github.com/dashpay/platform/issues/866))
* **dpp:** add fees API  to rust wasm bindings ([#830](https://github.com/dashpay/platform/issues/830))
* **dpp:** optional execution context in rs-dpp ([#811](https://github.com/dashpay/platform/issues/811))
* **dpp:** state transition applicator ([#878](https://github.com/dashpay/platform/issues/878))
* **rs-dpp:** migrate fees from js-dpp v0.24 ([#851](https://github.com/dashpay/platform/issues/851))
* state transition conversion ([#844](https://github.com/dashpay/platform/issues/844))
* **wasm-dpp:** add tests for state transition basic validator ([#857](https://github.com/dashpay/platform/issues/857))
* **wasm-dpp:** DashPlatformProtocol tests ([#841](https://github.com/dashpay/platform/issues/841))
* **wasm-dpp:** identity transitions additional functionality ([#855](https://github.com/dashpay/platform/issues/855))
* **wasm-dpp:** implement validateStateTransitionStateFactory tests ([#861](https://github.com/dashpay/platform/issues/861))
* **wasm-dpp:** provide external entropy generator to document factory ([#845](https://github.com/dashpay/platform/issues/845))
* **wasm-dpp:** validate_state_transition_identity_signature binding and test ([#865](https://github.com/dashpay/platform/issues/865))
* better Core 19 support ([#832](https://github.com/dashpay/platform/issues/832))
* core version 19.0-beta integration ([#771](https://github.com/dashpay/platform/issues/771))
* **dashmate:** register HPMN for local network ([#796](https://github.com/dashpay/platform/issues/796))
* **dasmate:** pack release script ([#781](https://github.com/dashpay/platform/issues/781))
* **dpp:** identity facade ([#782](https://github.com/dashpay/platform/issues/782))
* **dpp:** integration tests for wasm-dpp document transitions ([#777](https://github.com/dashpay/platform/issues/777))
* **dpp:** wasm bindings for Documents related validations ([#709](https://github.com/dashpay/platform/issues/709))
* **dpp:** wasm-dpp: integration tests for document ([#762](https://github.com/dashpay/platform/issues/762))
* Identity v2 ([#705](https://github.com/dashpay/platform/issues/705))
* platform value abstraction ([#805](https://github.com/dashpay/platform/issues/805))
* proposer signaling of protocol version upgrade and fork activation ([#778](https://github.com/dashpay/platform/issues/778))
* register system data contracts in RS Drive ([#776](https://github.com/dashpay/platform/issues/776))
* **rs-dpp:**  dashpay datatrigger toUserIds better validation ([#799](https://github.com/dashpay/platform/issues/799))
* **rs-dpp:** backport of index_definitions.unique validation ([#802](https://github.com/dashpay/platform/issues/802))
* **rs-dpp:** backports of identity/stateTransition from js-dpp ([#800](https://github.com/dashpay/platform/issues/800))
* **rs-dpp:** introduce `StateTransitionFactory` ([#810](https://github.com/dashpay/platform/issues/810))
* **rs-dpp:** validate indices are backwards compatible backport ([#797](https://github.com/dashpay/platform/issues/797))
* **rs-drive:** verification feature ([#803](https://github.com/dashpay/platform/issues/803))
* **wasm dpp:** validate state transition key signature ([#806](https://github.com/dashpay/platform/issues/806))
* **wasm-dpp:**  wasm bindings for Document Transitions  ([#707](https://github.com/dashpay/platform/issues/707))
* **wasm-dpp:** async state repository ([#766](https://github.com/dashpay/platform/issues/766))
* **wasm-dpp:** data contract facade ([#716](https://github.com/dashpay/platform/issues/716))
* **wasm-dpp:** Fix metadata, metadata tests and backport v23 matedata changes into wasm-dpp ([#819](https://github.com/dashpay/platform/issues/819))
* **wasm-dpp:** implement identity update transition ([#748](https://github.com/dashpay/platform/issues/748))
* **wasm-dpp:** integration tests validate data contract update transition ([#812](https://github.com/dashpay/platform/issues/812))
* **wasm-dpp:** protocol version validator tests ([#823](https://github.com/dashpay/platform/issues/823))
* **wasm-dpp:** remove unused documents factory tests ([#828](https://github.com/dashpay/platform/issues/828))
* **wasm-dpp:** state transition facade ([#814](https://github.com/dashpay/platform/issues/814))
* withdrawals status sync ([#679](https://github.com/dashpay/platform/issues/679))
* allow to get drive status from dashmate helper ([#749](https://github.com/dashpay/platform/issues/749))
* allow to get drive's status from dashmate helper ([#755](https://github.com/dashpay/platform/issues/755))
* **dapi:** use single envoy port for all connections ([#752](https://github.com/dashpay/platform/issues/752))
* **dashmate:** update Core to 18.2.0 ([#735](https://github.com/dashpay/platform/issues/735))
* **drive:** ABCI context logger ([#693](https://github.com/dashpay/platform/issues/693))
* **drive:** log contractId in deliverTx handler ([#730](https://github.com/dashpay/platform/issues/730))
* **drive:** log number of refunded epochs ([#729](https://github.com/dashpay/platform/issues/729))
* integrate wasm Document into JS tests ([#644](https://github.com/dashpay/platform/issues/644))
* varint protocol version ([#758](https://github.com/dashpay/platform/issues/758))
* **wasm-dpp:** implement function to produce generics from JsValue ([#712](https://github.com/dashpay/platform/issues/712))
* **wasm-dpp:** implement identity create transition ([#697](https://github.com/dashpay/platform/issues/697))
* **wasm-dpp:** implement identity topup transition ([#745](https://github.com/dashpay/platform/issues/745))
* **wasm-dpp:** Wasm dpp integration tests validate data contract factory ([#751](https://github.com/dashpay/platform/issues/751))
* credit refunds ([#662](https://github.com/dashpay/platform/issues/662))
* **dashmate:** additional dashd options ([#692](https://github.com/dashpay/platform/issues/692))
* **dashmate:** pass ZeroSSL as command line parameter ([#651](https://github.com/dashpay/platform/issues/651))
* **dashmate:** remove axios from zerossl requests
* **dashmate:** remove axios from zerossl requests
* **dpp:** AbstractConsensusError tests and extensions ([#670](https://github.com/dashpay/platform/issues/670))
* **dpp:** Data Contract Update Transition wasm binding ([#696](https://github.com/dashpay/platform/issues/696))
* **drive:** do not switch to validator quorum which will be removed soon ([#616](https://github.com/dashpay/platform/issues/616))
* multiple documents changes per batch and support for GroveDB 0.9 ([#699](https://github.com/dashpay/platform/issues/699))
* Consensus Errors and ValidationResult bindings ([#643](https://github.com/dashpay/platform/issues/643))
* average estimated processing fees ([#642](https://github.com/dashpay/platform/issues/642))
* **dpp:** bls adapter for WASM DPP ([#633](https://github.com/dashpay/platform/issues/633))
* **drive:** add time and protocolVersion fields to query metadata response ([#611](https://github.com/dashpay/platform/issues/611))
* **drive:** precise fees (dashpay/rs-platform[#170](https://github.com/dashpay/platform/issues/170)), closes [dashpay/rs-platform#181](https://github.com/dashpay/rs-platform/issues/181)
* **drive:** provide latest core chain lock on init chain ([#659](https://github.com/dashpay/platform/issues/659))
* **drive:** support for V0.7 of groveDB ([#665](https://github.com/dashpay/platform/issues/665))
* **drive:** use proposal block execution context in state repository ([#653](https://github.com/dashpay/platform/issues/653))
* **drive:** use single block execution context ([#627](https://github.com/dashpay/platform/issues/627))
* external bls validtor (dashpay/rs-platform[#186](https://github.com/dashpay/platform/issues/186))
* insert with parents for `Document` (dashpay/rs-platform[#189](https://github.com/dashpay/platform/issues/189))
* add `withdrawals` data contract package ([#604](https://github.com/dashpay/platform/issues/604))
* **done:** changes needed for wasm-dpp integration (dashpay/rs-platform[#154](https://github.com/dashpay/platform/issues/154))
* **dpp:** [v23 port]  validate fee calculating worst case operations (dashpay/rs-platform[#160](https://github.com/dashpay/platform/issues/160))
* **dpp:** dashpay datatrigger toUserIds better validation ([#620](https://github.com/dashpay/platform/issues/620))
* **drive:** select the most vital validator set quorums ([#617](https://github.com/dashpay/platform/issues/617))
* **dpp:** initial RS DPP integration ([#483](https://github.com/dashpay/platform/issues/483))
* **drive:** same block execution ([#593](https://github.com/dashpay/platform/issues/593))
* **node:** multiple transactions (dashpay/rs-platform[#155](https://github.com/dashpay/platform/issues/155))
* **drive:** AssetUnlock transactions processing ([#530](https://github.com/dashpay/platform/issues/530))
* withdrawal request queue (dashpay/rs-platform[#149](https://github.com/dashpay/platform/issues/149))
* Public Keys Identities Proofs (dashpay/rs-platform[#151](https://github.com/dashpay/platform/issues/151))
*  [v23 port]  data contract indices validation (dashpay/rs-platform[#26](https://github.com/dashpay/platform/issues/26))
* a temporary plug for dry run (dashpay/rs-platform[#113](https://github.com/dashpay/platform/issues/113))
* ability to get elements by $id directly (dashpay/rs-platform[#61](https://github.com/dashpay/platform/issues/61))
* add `proveDocumentsQuery` to Node.JS binding (dashpay/rs-platform[#115](https://github.com/dashpay/platform/issues/115))
* add `proveQueryMany` to Node.JS binding (dashpay/rs-platform[#122](https://github.com/dashpay/platform/issues/122))
* add constructor for DocumentTransition
* add credits converter
* add Document Transition
* add GroveDB methods to JS wrapper
* add hash implementation for identifier
* add prefixes to errors (dashpay/rs-platform[#101](https://github.com/dashpay/platform/issues/101))
* add proof test to rs-drive query tests (dashpay/rs-platform[#109](https://github.com/dashpay/platform/issues/109))
* allow one character property names
* batch support (dashpay/rs-platform[#111](https://github.com/dashpay/platform/issues/111))
* binding for Node.JS
* **dashmate:** update tenderdash to 0.9.0-dev.1 ([#525](https://github.com/dashpay/platform/issues/525))
* **dashmate:** zeroSSL certificate renewal helper ([#554](https://github.com/dashpay/platform/issues/554))
* **dpp:**  [v23 port] add withdraw puprose to identity public key  (dashpay/rs-platform[#134](https://github.com/dashpay/platform/issues/134))
* **dpp:** [v23 port] Identity Update Transition (dashpay/rs-platform[#138](https://github.com/dashpay/platform/issues/138))
* **dpp:** [v23 port] limit the number of shares for masternode by 16 (dashpay/rs-platform[#141](https://github.com/dashpay/platform/issues/141))
* **dpp:** add wasm-dpp template package ([#529](https://github.com/dashpay/platform/issues/529))
* **dpp:** basic validtion for state transition (dashpay/rs-platform[#133](https://github.com/dashpay/platform/issues/133))
* fees distribution (dashpay/rs-platform[#105](https://github.com/dashpay/platform/issues/105))
* identity create state transition (dashpay/rs-platform[#9](https://github.com/dashpay/platform/issues/9))
* identity credit withdrawal transition (dashpay/rs-platform[#25](https://github.com/dashpay/platform/issues/25))
* identity from_buffer and from_object
* immutibility and contracts that allow document history (dashpay/rs-platform[#79](https://github.com/dashpay/platform/issues/79))
* implement `deleteDocument`
* implement `updateDocument`
* implement applyContact and createDocument
* implement grovedb "bindings" in rs-drive
* implement masternode voting identities ([#467](https://github.com/dashpay/platform/issues/467))
* implement queryDocuments + some fixes
* implementation of Document validator
* Include the DPP into Drive (dashpay/rs-platform[#126](https://github.com/dashpay/platform/issues/126))
* insert identities (dashpay/rs-platform[#99](https://github.com/dashpay/platform/issues/99))
* migrate to ABCI++ ([#464](https://github.com/dashpay/platform/issues/464))
* **node:** introduce GroveDB#proveQuery (dashpay/rs-platform[#112](https://github.com/dashpay/platform/issues/112))
* populate stack for binding errors (dashpay/rs-platform[#39](https://github.com/dashpay/platform/issues/39))
* query drive with sql  (dashpay/rs-platform[#31](https://github.com/dashpay/platform/issues/31)), closes [dashpay/rs-platform#42](https://github.com/dashpay/rs-platform/issues/42)
* **query:** allow query with `$id` (dashpay/rs-platform[#53](https://github.com/dashpay/platform/issues/53))
* recursive conditional subqueries (dashpay/rs-platform[#106](https://github.com/dashpay/platform/issues/106))
* return processing cost for `queryDocuments` (dashpay/rs-platform[#100](https://github.com/dashpay/platform/issues/100))
* sql in clause (dashpay/rs-platform[#52](https://github.com/dashpay/platform/issues/52))
* SSL certificate for DAPI ([#519](https://github.com/dashpay/platform/issues/519))
* storage run time fees, worst case scenario fees and support for contract definition references (dashpay/rs-platform[#95](https://github.com/dashpay/platform/issues/95)), closes [dashpay/rs-platform#87](https://github.com/dashpay/rs-platform/issues/87) [dashpay/rs-platform#93](https://github.com/dashpay/rs-platform/issues/93) [dashpay/rs-platform#92](https://github.com/dashpay/rs-platform/issues/92)
* **tests:** add `reference` js test case (dashpay/rs-platform[#43](https://github.com/dashpay/platform/issues/43))
* update to latest grovedb and some optimization around inserts (dashpay/rs-platform[#120](https://github.com/dashpay/platform/issues/120)), closes [dashpay/rs-platform#119](https://github.com/dashpay/rs-platform/issues/119)
* verbose `startAt` or `startAfter` not found error (dashpay/rs-platform[#76](https://github.com/dashpay/platform/issues/76))


### Bug Fixes

* **dashmate:** external IP detection hangs sometimes ([#1053](https://github.com/dashpay/platform/issues/1053))
* **dapi:** invalid addresses in the whitelist ([#1044](https://github.com/dashpay/platform/issues/1044))
* **dashmate:** reset platform commands hangs ([#1038](https://github.com/dashpay/platform/issues/1038))
* **dashmate:** set permissions for dashcore log file ([#1037](https://github.com/dashpay/platform/issues/1037))
* **drive:** cannot destructure property ‘quorumHash’ of ‘instantLock... ([#1046](https://github.com/dashpay/platform/issues/1046))
* **drive:** cannot read properties of undefined (reading 'toString') ([#1045](https://github.com/dashpay/platform/issues/1045))
* **dashmate:** waitForQuorumConnections deadline of 300000 exceeded ([#1015](https://github.com/dashpay/platform/issues/1015))
* **dashmate:** wrong volume removal retry logic ([#1016](https://github.com/dashpay/platform/issues/1016))
* expect platformNodeID to be a hex string ([#1013](https://github.com/dashpay/platform/issues/1013))
* **dashmate:** "volume is in use" and "no such volume" ([#1005](https://github.com/dashpay/platform/issues/1005))
* **dashmate:** reset platform affects core ([#1001](https://github.com/dashpay/platform/issues/1001))
* **drive:** double init chain leads to side bugs ([#1002](https://github.com/dashpay/platform/issues/1002))
* **dashmate:** BLS private key validate accepts whitespaces ([#998](https://github.com/dashpay/platform/issues/998))
* **dashmate:** can't remove volumes if they not exist ([#997](https://github.com/dashpay/platform/issues/997))
* **dashmate:** show masternode state while it is not synced ([#999](https://github.com/dashpay/platform/issues/999))
* **dashmate:** wrap register masternode command in small terminals ([#996](https://github.com/dashpay/platform/issues/996))
* **dashmate:** multiple issues in the reset command ([#991](https://github.com/dashpay/platform/issues/991))
* **drive:** non-unique masternode voting keys ([#986](https://github.com/dashpay/platform/issues/986))
* **dashmate:** ability to work with non-default docker socket path ([#967](https://github.com/dashpay/platform/issues/967))
* **dashmate:** already configured preset is ignored ([#974](https://github.com/dashpay/platform/issues/974))
* **dashmate:** empty masternode status while syncing ([#970](https://github.com/dashpay/platform/issues/970))
* **dashmate:** form accepts invalid BLS key ([#961](https://github.com/dashpay/platform/issues/961))
* **dashmate:** invalid mn register command in output of dashmate setup ([#959](https://github.com/dashpay/platform/issues/959))
* **dashmate:** reward shares can be negative during HP masternode registration ([#960](https://github.com/dashpay/platform/issues/960))
* **dashmate:** select number of masternodes during local setup freezes ([#957](https://github.com/dashpay/platform/issues/957))
* **dashmate:** yaml package security vulnerability ([#975](https://github.com/dashpay/platform/issues/975))
* **test-suite:** expects identities for invalid mns ([#968](https://github.com/dashpay/platform/issues/968))
* **dashamte:** wrong envoy config path on windows ([#949](https://github.com/dashpay/platform/issues/949))
* **drive:** updating a masternode identity with invalid entry from SML ([#965](https://github.com/dashpay/platform/issues/965))
* **dashamte:** Can't find begoo/index with `yarn dashmate setup` ([#933](https://github.com/dashpay/platform/issues/933))
* DAPI still expected on normal masternodes ([#904](https://github.com/dashpay/platform/issues/904))
* **dapi-client:** platform port is ignored from SML ([#903](https://github.com/dashpay/platform/issues/903))
* **dashmate:** api binds to all interfaces ([#893](https://github.com/dashpay/platform/issues/893))
* **dashmate:** dashmate helper is running under root user ([#895](https://github.com/dashpay/platform/issues/895))
* **dashmate:** dashmate logic doesn't recognize it's ran from helper ([#902](https://github.com/dashpay/platform/issues/902))
* **dashmate:** missing rawblock zmq message in core config ([#770](https://github.com/dashpay/platform/issues/770))
* **dashmate:** undefined wallet for dash-cli ([#786](https://github.com/dashpay/platform/issues/786))
* **dpp:** various fixes in DPP and system contracts ([#907](https://github.com/dashpay/platform/issues/907))
* **drive:** non-deterministic run of mn identities sync ([#910](https://github.com/dashpay/platform/issues/910))
* **drive:** total HPMNs contains all masternodes ([#911](https://github.com/dashpay/platform/issues/911))
* identifier deserialization doesn't work for bincode ([#885](https://github.com/dashpay/platform/issues/885))
* llmqType must be equal to one of the allowed values ([#884](https://github.com/dashpay/platform/issues/884))
* possible overflow issues ([#877](https://github.com/dashpay/platform/issues/877))
* **dashmate:** ambiguous validation errors for file certificates ([#870](https://github.com/dashpay/platform/issues/870))
* **dashmate:** config.isPlatformEnabled is not a function ([#872](https://github.com/dashpay/platform/issues/872))
* **dpp:** incorrect public key validator schema in ST Facade ([#854](https://github.com/dashpay/platform/issues/854))
* **scripts:** update configure_test_network for hpmn ([#863](https://github.com/dashpay/platform/issues/863))
* **wasm-dpp:** fix decoding protocol version varint error to match previous implementation ([#849](https://github.com/dashpay/platform/issues/849))
* **ci:** fix release workflow syntax error ([#808](https://github.com/dashpay/platform/issues/808))
* **dashmate:** make dashmate helper run commands as host user ([#765](https://github.com/dashpay/platform/issues/765))
* **dashmate:** visual fixes for dashmate status ([#787](https://github.com/dashpay/platform/issues/787))
* **dpp:** update jsonschema-rs and enable tests ([#780](https://github.com/dashpay/platform/issues/780))
* **rs-dpp:** fetch latest core chain locked height misuse ([#789](https://github.com/dashpay/platform/issues/789))
* update webpack to resolve npm audit error ([#822](https://github.com/dashpay/platform/issues/822))
* **wasm-dpp:** Identifier and its tests ([#821](https://github.com/dashpay/platform/issues/821))
* **dashmate:** Dash Core container is unable to restart properly under WSL ([#736](https://github.com/dashpay/platform/issues/736))
* **dashmate:** fix migration for configs without platform ([#738](https://github.com/dashpay/platform/issues/738))
* **dashmate:** migrations fixes ([#759](https://github.com/dashpay/platform/issues/759))
* **dpp:** existing property in a new index ([#694](https://github.com/dashpay/platform/issues/694))
* ua-parser-js vulnerability  ([#756](https://github.com/dashpay/platform/issues/756))
* **dashmate:** ZeroSSL certificate cannot be downloaded ([#718](https://github.com/dashpay/platform/issues/718))
* **dpp:** can’t create fingerprint from a document transition ([#723](https://github.com/dashpay/platform/issues/723))
* **drive:** merk caching in contract caching (irony) ([#710](https://github.com/dashpay/platform/issues/710))
* find_duplicates_by_id.rs not compiling ([#702](https://github.com/dashpay/platform/issues/702))
* Starcounter-Jack JSON-Patch Prototype Pollution vulnerability ([#708](https://github.com/dashpay/platform/issues/708))
* **dashmate:** setDay is not a function ([#677](https://github.com/dashpay/platform/issues/677))
* **dashmate:** ZeroSSL certificates are not saved in WSL ([#676](https://github.com/dashpay/platform/issues/676))
* **drive:** initChain handler is not idempotent ([#675](https://github.com/dashpay/platform/issues/675))
* **dashmate:** SSL domain verification config could not be generated in WSL ([#673](https://github.com/dashpay/platform/issues/673))
* build not working because of deprecated wasm-bindgen feature ([#639](https://github.com/dashpay/platform/issues/639))
* **dapi:** fail to reconnect to tenderdash in case of ENOTFOUND ([#621](https://github.com/dashpay/platform/issues/621))
* **dashmate:** broken helper docker image ([#630](https://github.com/dashpay/platform/issues/630))
* **dashmate:** outdated Drive and DAPI images ([#668](https://github.com/dashpay/platform/issues/668))
* **dashmate:** ZeroSSL certificate renewal ([#624](https://github.com/dashpay/platform/issues/624))
* **drive:** invalid create name
* **drive:** multi transactions doesn't work properly ([#636](https://github.com/dashpay/platform/issues/636))
* **drive:** remove ambiguous use
* DataContract.spec.js in wasm-dpp ([#618](https://github.com/dashpay/platform/issues/618))
* **dpp:**  [v23 port] cannot read properties of null (reading 'getBalance') (dashpay/rs-platform[#163](https://github.com/dashpay/platform/issues/163))
* **dpp:** [v23 port] non-deterministic fees due to data contract cache (dashpay/rs-platform[#161](https://github.com/dashpay/platform/issues/161))
* **dpp:** [v23 port] repeated disabling of already disabled identity key (dashpay/rs-platform[#162](https://github.com/dashpay/platform/issues/162))
* renamed method from rs-dpp ([#623](https://github.com/dashpay/platform/issues/623))
* `featureFlags` test was awaiting blocks that have not been produced ([#602](https://github.com/dashpay/platform/issues/602))
* **dapi:** `getConsensusParamsHandler` was handling wrong Tendermint error ([#601](https://github.com/dashpay/platform/issues/601))
* **dashmate:** invalid testnet TenderDash genesis ([#608](https://github.com/dashpay/platform/issues/608))
* **dashmate:** SSL verification server cannot be started ([#606](https://github.com/dashpay/platform/issues/606))
* typo `dash-amte` to `dashmate` ([#599](https://github.com/dashpay/platform/issues/599))
* **dapi-client:** temporary use http protocol by default ([#573](https://github.com/dashpay/platform/issues/573))
* using `ProtocolError ` in `cbor_value_to_json_value ` could lead to a stackoverflow error (dashpay/rs-platform[#164](https://github.com/dashpay/platform/issues/164))
* "number" property type is not implemented (dashpay/rs-platform[#47](https://github.com/dashpay/platform/issues/47))
* `Identity.balance` was of type `i64` but should be `u64` (dashpay/rs-platform[#23](https://github.com/dashpay/platform/issues/23))
* appendStack is not present in NPM package (dashpay/rs-platform[#41](https://github.com/dashpay/platform/issues/41))
* **ci:** support alpha prereleases ([#560](https://github.com/dashpay/platform/issues/560))
* comply with newest grovedb (dashpay/rs-platform[#121](https://github.com/dashpay/platform/issues/121))
* contract parsing errors
* create and update document in different transactions (dashpay/rs-platform[#68](https://github.com/dashpay/platform/issues/68))
* create two documents in different transactions (dashpay/rs-platform[#69](https://github.com/dashpay/platform/issues/69))
* delete empty trees (dashpay/rs-platform[#49](https://github.com/dashpay/platform/issues/49))
* **dpp:** [v23 port]  change allowed security level for withdrawal purpose to critical (dashpay/rs-platform[#140](https://github.com/dashpay/platform/issues/140))
* duplicate batched storage fee update (dashpay/rs-platform[#150](https://github.com/dashpay/platform/issues/150))
* fix build when using grovedb master
* fix paths
* fix some tests
* handle key not found error from grovedb (dashpay/rs-platform[#33](https://github.com/dashpay/platform/issues/33))
* index already exists on update document (dashpay/rs-platform[#64](https://github.com/dashpay/platform/issues/64))
* many insert commit fail (dashpay/rs-platform[#45](https://github.com/dashpay/platform/issues/45))
* merging equal path queries (dashpay/rs-platform[#128](https://github.com/dashpay/platform/issues/128))
* merging required properties in Data Cotnract
* neon security vulnerability (dashpay/rs-platform[#110](https://github.com/dashpay/platform/issues/110))
* non-deterministic apply contract (dashpay/rs-platform[#46](https://github.com/dashpay/platform/issues/46))
* non-present optional fields shouldn't be indexed
* order by on non equal fields (dashpay/rs-platform[#37](https://github.com/dashpay/platform/issues/37))
* owner id and additional tests (dashpay/rs-platform[#59](https://github.com/dashpay/platform/issues/59))
* path_queries can only refer to items and references (dashpay/rs-platform[#88](https://github.com/dashpay/platform/issues/88))
* primary key tree is not present (dashpay/rs-platform[#74](https://github.com/dashpay/platform/issues/74))
* query empty contract fails (dashpay/rs-platform[#65](https://github.com/dashpay/platform/issues/65))
* query validation logic (dashpay/rs-platform[#104](https://github.com/dashpay/platform/issues/104))
* remove prebuilds at pretest stage to prevent random mocha error process killed
* strange error on deletion of specific data set (dashpay/rs-platform[#90](https://github.com/dashpay/platform/issues/90))
* unable to decode contract
* update contract (dashpay/rs-platform[#72](https://github.com/dashpay/platform/issues/72))
* update to latest grovedb and added some tests. (dashpay/rs-platform[#123](https://github.com/dashpay/platform/issues/123))
* use binary zero instead of ascii
* use correct linker
* use slices instead of binary strings to represent the values we intend


### Continuous Integration

* increase release timeouts ([#1032](https://github.com/dashpay/platform/issues/1032))
* add PR linter ([#1025](https://github.com/dashpay/platform/issues/1025))
* add timeouts to self-hosted ci runs ([#1026](https://github.com/dashpay/platform/issues/1026))
* remove drive node.js binding release
* sign MacOs Dashmate release ([#890](https://github.com/dashpay/platform/issues/890))
* dashmate release script fix ([#846](https://github.com/dashpay/platform/issues/846), [#836](https://github.com/dashpay/platform/issues/836))
* increase cache-max-size ([#704](https://github.com/dashpay/platform/issues/704))
* add rust toolchain with wasm target to release workflow
* integrate rust and js build process
* add toolchain to setup rust actions
* build package before lining
* checks for JS and Rust packages
* code scanning ([#626](https://github.com/dashpay/platform/issues/626))
* do not build everything everytime
* fix concurrency for js checks
* fix env syntax in release workflow ([#664](https://github.com/dashpay/platform/issues/664))
* fix rs-checks
* fix workflow names
* increase JS linting timeout
* increase timeouts
* move concurrency to package workflows
* move out workflows from include
* set concurrency based on workflow
* shorten workflow
* release.yml contained wrong indentation ([#597](https://github.com/dashpay/platform/issues/597))
* publish envoy and dashmate-helper docker images after release ([#595](https://github.com/dashpay/platform/issues/595))
* update workflows to use stable toolchain (dashpay/rs-platform[#167](https://github.com/dashpay/platform/issues/167))
* `musl` binaries were not built correctly (dashpay/rs-platform[#58](https://github.com/dashpay/platform/issues/58))
* add deps security check
* add Node.JS workflow
* add release workflow
* add rust-toolchain.toml
* fix aarch64 musl build (dashpay/rs-platform[#36](https://github.com/dashpay/platform/issues/36))
* formatter check
* implement initial ci
* prevent PRs to master (dashpay/rs-platform[#28](https://github.com/dashpay/platform/issues/28))
* rename workflow
* run GitHub action for PR on dev branches (dashpay/rs-platform[#38](https://github.com/dashpay/platform/issues/38))


### Tests

* **dpp:** remove old dpp fixtures from tests ([#971](https://github.com/dashpay/platform/issues/971))
* **rs-drive:** fix path to EntropyGenerator ([#856](https://github.com/dashpay/platform/issues/856))
* **wasm-dpp:** decodeProtocolEntity test ([#834](https://github.com/dashpay/platform/issues/834))
* **drive:** synchronizeMasternodeIdentitiesFactory ([#586](https://github.com/dashpay/platform/issues/586))
* add test with fetching non exist document
* added test for in with only some elements (dashpay/rs-platform[#81](https://github.com/dashpay/platform/issues/81))
* dpns tests (dashpay/rs-platform[#89](https://github.com/dashpay/platform/issues/89))
* encoding (added negative and positive infinity)
* refactor and implementation of document's validator tests


### Build System

* **dashmate:** better versioning ([#988](https://github.com/dashpay/platform/issues/988))
* **dashmate:** prepare MacOs build for notarization ([#985](https://github.com/dashpay/platform/issues/985))
* **dashmate:** remove MacOs malicious software warning ([#977](https://github.com/dashpay/platform/issues/977))
* **dashmate:** fix removing unnecessary packages
* **dashamte:** build arm64 deb file instead of armel ([#943](https://github.com/dashpay/platform/issues/943))
* **dashamte:** remove deb release docker dependencies ([#934](https://github.com/dashpay/platform/issues/934))
* **dashmate:** build only linux tarballs ([#936](https://github.com/dashpay/platform/issues/936))
* **dashmate:** fix deb package release  ([#864](https://github.com/dashpay/platform/issues/864))
* operations in configure.sh script were in a wrong order ([#876](https://github.com/dashpay/platform/issues/876))
* add missing deps ([#824](https://github.com/dashpay/platform/issues/824))
* add bash to Drive's Dockerfile
* continuation of build profile fixing
* fix cargo build profile option
* use debug profile for development
* release script could not find previous tag in some cases ([#558](https://github.com/dashpay/platform/issues/558))


### Code Refactoring

* **dashmate:** always keep platform config ([#868](https://github.com/dashpay/platform/issues/868))
* **dashmate:** move core devnet options to subsection ([#867](https://github.com/dashpay/platform/issues/867))
* change dpp to be based on platform value ([#809](https://github.com/dashpay/platform/issues/809))
* remove various Clippy warnings ([#793](https://github.com/dashpay/platform/issues/793))
* **rs-dpp:** bring error type to one format ([#804](https://github.com/dashpay/platform/issues/804))
* initial cleanup of Drive and DPP document types ([#750](https://github.com/dashpay/platform/issues/750))
* use a trait to remove the need for some functions ([#747](https://github.com/dashpay/platform/issues/747))
* **dashmate:** status command ([#660](https://github.com/dashpay/platform/issues/660))
* **drive:** remove redundant genesis time key ([#722](https://github.com/dashpay/platform/issues/722))
* use FeeResult to collect block fees ([#652](https://github.com/dashpay/platform/issues/652))
* accept DPP entities
* adjust how left_to_right is assigned
* documents refactoring, with batching and fee work (dashpay/rs-platform[#118](https://github.com/dashpay/platform/issues/118))
* error types to handle user errors (dashpay/rs-platform[#77](https://github.com/dashpay/platform/issues/77))
* fix numerous unused include warnings
* query (dashpay/rs-platform[#96](https://github.com/dashpay/platform/issues/96))
* query `$id` property (dashpay/rs-platform[#56](https://github.com/dashpay/platform/issues/56))
* remove a few unneeded `mut` keywords, fixes warnings
* remove unused variable
* replace unused mut variable with todo for calculating cost
* return array type in case of non byteArray
* split GroveDB and Drive
* split query_documents into two methods
* update grovedb (dashpay/rs-platform[#91](https://github.com/dashpay/platform/issues/91))
* use 32 byte identity fields


### Performance Improvements

* **drive:** do not call process proposal after prepare ([#656](https://github.com/dashpay/platform/issues/656))


### Miscellaneous Chores

* bump sentinel to 1.7.3 ([#1057](https://github.com/dashpay/platform/issues/1057))
* **dashmate:** rename compose project name ([#1055](https://github.com/dashpay/platform/issues/1055))
* **dashmate:** update testnet preset ([#1054](https://github.com/dashpay/platform/issues/1054))
* increase client side timeouts ([#1050](https://github.com/dashpay/platform/issues/1050))
* **dapi-client:** upate testnet masternode addresses whitelist ([#1023](https://github.com/dashpay/platform/issues/1023))
* disable protocol version signaling ([#1029](https://github.com/dashpay/platform/issues/1029))
* **dashamte:** remove deprecation warning on start ([#925](https://github.com/dashpay/platform/issues/925))
* **dashmate:** update production dashcore versions for mainnet and testnet ([#840](https://github.com/dashpay/platform/issues/840))
* **sdk:** add eslint ([#829](https://github.com/dashpay/platform/issues/829))
* `rs-dpp` and `wasm-dpp` updates for integration ([#875](https://github.com/dashpay/platform/issues/875))
* **wasm-dpp:** proper identifier buffer inheritance ([#879](https://github.com/dashpay/platform/issues/879))
* **dashmate:** upgrade dashcore version for local networks ([#843](https://github.com/dashpay/platform/issues/843))
* **drive:** temporary disable payout script ([#835](https://github.com/dashpay/platform/issues/835))
* **drive:** temporary disable unstable withdrawal logic ([#831](https://github.com/dashpay/platform/issues/831))
* **sdk:** add eslint ([#827](https://github.com/dashpay/platform/issues/827))
* **dashmate:** sync Tenderdash config with Tenderdash v0.10.0-dev.8 ([#746](https://github.com/dashpay/platform/issues/746))
* update Tenderdash to 0.10.0-dev.8 ([#741](https://github.com/dashpay/platform/issues/741))
* **dpp:** wasm dpp data contract test error types ([#684](https://github.com/dashpay/platform/issues/684))
* **drive:** log synchronize identities properly ([#686](https://github.com/dashpay/platform/issues/686))
* **dashmate:** update tenderdash up to 0.10.0-dev.6 ([#674](https://github.com/dashpay/platform/issues/674))
* **drive:** remove txs from logs ([#683](https://github.com/dashpay/platform/issues/683))
* **dashmate:** update tenderdash up to 0.10.0-dev.6 ([#674](https://github.com/dashpay/platform/issues/674))
* **drive:** remove txs from logs ([#683](https://github.com/dashpay/platform/issues/683))
* add yarn cache
* **dpp:** switch dpp to working revision
* **drive:** comprehensive logging for same block execution ([#657](https://github.com/dashpay/platform/issues/657))
* ensure consistent Rust dependencies ([#658](https://github.com/dashpay/platform/issues/658))
* fix NPM security vulnerabilities
* ignore target dir for docker
* **release:** bump package version
* **release:** update changelog and bump version to 0.24.0-dev.9 ([#667](https://github.com/dashpay/platform/issues/667))
* remove js-abci from workspaces
* remove lodash per-method deps ([#661](https://github.com/dashpay/platform/issues/661))
* remove package-lock.json
* update to yarn 3.3.0
* update bls-signatures to fix dependencies conflict (dashpay/rs-platform[#184](https://github.com/dashpay/platform/issues/184))
* **dpp:** re-enable limited array support for data contracts (dashpay/rs-platform[#165](https://github.com/dashpay/platform/issues/165))
* **dpp:** [v23 port] allow only asc order for indices (dashpay/rs-platform[#135](https://github.com/dashpay/platform/issues/135))
* update error prefix (dashpay/rs-platform[#67](https://github.com/dashpay/platform/issues/67))
* update to new GroveDB (dashpay/rs-platform[#108](https://github.com/dashpay/platform/issues/108)), closes [dashpay/rs-platform#107](https://github.com/dashpay/rs-platform/issues/107)


### Styles

* fix formatting
* extract field_type
* format json
* remove commented code
* remove todo
* run cargo fmt
* sam's *** naming
* use explicit xor (dashpay/rs-platform[#57](https://github.com/dashpay/platform/issues/57))


## [0.24.0-dev.34](https://github.com/dashpay/platform/compare/v0.24.0-dev.33...v0.24.0-dev.34) (2023-05-08)


### ⚠ BREAKING CHANGES

* Validator rotation logic is changed. Previous blockchain data won't be compatible (#1034)

### Features

* **drive:** whitelist and filter banned nodes for validators ([#1034](https://github.com/dashpay/platform/issues/1034))


### Bug Fixes

* **dapi:** invalid addresses in the whitelist ([#1044](https://github.com/dashpay/platform/issues/1044))
* **dashmate:** reset platform commands hangs ([#1038](https://github.com/dashpay/platform/issues/1038))
* **dashmate:** set permissions for dashcore log file ([#1037](https://github.com/dashpay/platform/issues/1037))
* **drive:** cannot destructure property ‘quorumHash’ of ‘instantLock... ([#1046](https://github.com/dashpay/platform/issues/1046))
* **drive:** cannot read properties of undefined (reading 'toString') ([#1045](https://github.com/dashpay/platform/issues/1045))

## [0.24.0-dev.33](https://github.com/dashpay/platform/compare/v0.24.0-dev.32...v0.24.0-dev.33) (2023-05-05)


### Continuous Integration

* increase release timeouts ([#1032](https://github.com/dashpay/platform/issues/1032))

## [0.24.0-dev.32](https://github.com/dashpay/platform/compare/v0.24.0-dev.31...v0.24.0-dev.32) (2023-05-04)

### ⚠ BREAKING CHANGES

* Previous state won't be valid (#1029)


### Features

* **dashmate:** core log file and debug categories ([#913](https://github.com/dashpay/platform/issues/913))


### Bug Fixes

* **dashmate:** waitForQuorumConnections deadline of 300000 exceeded ([#1015](https://github.com/dashpay/platform/issues/1015))
* **dashmate:** wrong volume removal retry logic ([#1016](https://github.com/dashpay/platform/issues/1016))


### Continuous Integration

* add PR linter ([#1025](https://github.com/dashpay/platform/issues/1025))
* add timeouts to self-hosted ci runs ([#1026](https://github.com/dashpay/platform/issues/1026))


### Miscellaneous Chores

* **dapi-client:** upate testnet masternode addresses whitelist ([#1023](https://github.com/dashpay/platform/issues/1023))
* **drive:** disable protocol version signaling ([#1029](https://github.com/dashpay/platform/issues/1029))



## [0.25.0-dev.2](https://github.com/dashpay/platform/compare/v0.25.0-dev.1...v0.25.0-dev.2) (2023-05-01)

### Miscellaneous Chores

* backports from v0.24


## [0.25.0-dev.1](https://github.com/dashpay/platform/compare/v0.24.0-dev.16...v0.25.0-dev.1) (2023-05-01)

### Features

* abci propose validators ([#954](https://github.com/dashpay/platform/issues/954))
* **dpp:** state transition applicator ([#878](https://github.com/dashpay/platform/issues/878))
* **wasm-dpp:** state_transition_fee_validator binding and tests ([#874](https://github.com/dashpay/platform/issues/874))
* **wasm-dpp:** validate_state_transition_identity_signature binding and test ([#865](https://github.com/dashpay/platform/issues/865))


### Bug Fixes


* fixes for deployment and clean up ([#1004](https://github.com/dashpay/platform/issues/1004))
* identifier deserialization doesn't work for bincode ([#885](https://github.com/dashpay/platform/issues/885))
* inconsistencies after merge
* init chain core chain lock failure ([#976](https://github.com/dashpay/platform/issues/976))
* **rs-dpp:** json query wrapper incorrectly serializes identifier
* **rs-drive-abci:** Get rs-drive-abci to commit block 1 ([#981](https://github.com/dashpay/platform/issues/981))
* **rs-drive-abci:** start rs-drive-abci using dashmate and pass init_chain ([#941](https://github.com/dashpay/platform/issues/941))
* same block core chain lock height
* **sdk:** provide StateTransitionExecutionContext to validateBasic
* small wasm fix
* **wasm-dpp:** failing build and tests ([#947](https://github.com/dashpay/platform/issues/947))


### Code Refactoring

* **rs-dpp:** use common wrapper for new signature errors
* **rs-drive-abci:** new error wrappers


### Tests

* chainlock quorum rotation test ([#952](https://github.com/dashpay/platform/issues/952))
* **dapi-client:** fix broken SimplifiedMasternodeListDAPIAddressProvider test ([#916](https://github.com/dashpay/platform/issues/916))
* **rs-dpp:** fix tests
* **rs-drive-abci:** fix optional pose_revived_height
* **rs-drive-abci:** set pose_revived_height to None


### Continuous Integration

* remove drive node.js binding release


### Build System

* add missing rust packages to Dockerfiles
* operations in configure.sh script were in a wrong order ([#876](https://github.com/dashpay/platform/issues/876))
* remove js-drive and rs-drive-nodejs from PR CI
* return proper Rust setup


### Miscellaneous Chores

* `rs-dpp` and `wasm-dpp` updates for integration ([#875](https://github.com/dashpay/platform/issues/875))
* bump base.js (local networks only) dashd version to 20.0.0-alpha.assetlocks.2
* remove unused deps ([#987](https://github.com/dashpay/platform/issues/987))
* remove unused js-drive and rs-drive code
* resolve todos
* **sdk:** add eslint ([#829](https://github.com/dashpay/platform/issues/829))
* use master dash core rpc
* **wasm-dpp:** proper identifier buffer inheritance ([#879](https://github.com/dashpay/platform/issues/879))

## [0.24.0-dev.31](https://github.com/dashpay/platform/compare/v0.24.0-dev.30...v0.24.0-dev.31) (2023-05-01)


### Bug Fixes

* expect platformNodeID to be a hex string ([#1013](https://github.com/dashpay/platform/issues/1013))

## [0.24.0-dev.30](https://github.com/dashpay/platform/compare/v0.24.0-dev.29...v0.24.0-dev.30) (2023-04-30)


### Bug Fixes

* **dashmate:** "volume is in use" and "no such volume" ([#1005](https://github.com/dashpay/platform/issues/1005))


## [0.24.0-dev.29](https://github.com/dashpay/platform/compare/v0.24.0-dev.28...v0.24.0-dev.29) (2023-04-29)


### Bug Fixes

* **dashmate:** reset platform affects core ([#1001](https://github.com/dashpay/platform/issues/1001))
* **drive:** double init chain leads to side bugs ([#1002](https://github.com/dashpay/platform/issues/1002))

## [0.24.0-dev.28](https://github.com/dashpay/platform/compare/v0.24.0-dev.27...v0.24.0-dev.28) (2023-04-28)


### Bug Fixes

* **dashmate:** BLS private key validate accepts whitespaces ([#998](https://github.com/dashpay/platform/issues/998))
* **dashmate:** can't remove volumes if they not exist ([#997](https://github.com/dashpay/platform/issues/997))
* **dashmate:** show masternode state while it is not synced ([#999](https://github.com/dashpay/platform/issues/999))
* **dashmate:** wrap register masternode command in small terminals ([#996](https://github.com/dashpay/platform/issues/996))

## [0.24.0-dev.27](https://github.com/dashpay/platform/compare/v0.24.0-dev.26...v0.24.0-dev.27) (2023-04-28)


### ⚠ BREAKING CHANGES

* The --platfrom-only flag is renamed to --platform (#991)

### Bug Fixes

* **dashmate:** multiple issues in the reset command ([#991](https://github.com/dashpay/platform/issues/991))

## [0.24.0-dev.26](https://github.com/dashpay/platform/compare/v0.24.0-dev.25...v0.24.0-dev.26) (2023-04-27)


### ⚠ BREAKING CHANGES

* Previous state won't be valid due to changes in the sync identities logic

### Bug Fixes

* **drive:** non-unique masternode voting keys ([#986](https://github.com/dashpay/platform/issues/986))


### Build System

* **dashmate:** better versioning ([#988](https://github.com/dashpay/platform/issues/988))
* **dashmate:** prepare MacOs build for notarization ([#985](https://github.com/dashpay/platform/issues/985))

## [0.24.0-dev.25](https://github.com/dashpay/platform/compare/v0.24.0-dev.24...v0.24.0-dev.25) (2023-04-26)


### Features

* **dashmate:** better ZeroSSL error messages ([#950](https://github.com/dashpay/platform/issues/950))
* **dashmate:** set random core rpc username and password on setup ([#973](https://github.com/dashpay/platform/issues/973))
* **dashmate:** verbose `connect ENOENT /var/run/docker.sock` error ([#951](https://github.com/dashpay/platform/issues/951))


### Bug Fixes

* **dashmate:** ability to work with non-default docker socket path ([#967](https://github.com/dashpay/platform/issues/967))
* **dashmate:** empty masternode status while syncing ([#970](https://github.com/dashpay/platform/issues/970))
* **dashmate:** form accepts invalid BLS key ([#961](https://github.com/dashpay/platform/issues/961))
* **dashmate:** invalid mn register command in output of dashmate setup ([#959](https://github.com/dashpay/platform/issues/959))
* **dashmate:** reward shares can be negative during HP masternode registration ([#960](https://github.com/dashpay/platform/issues/960))
* **dashmate:** select number of masternodes during local setup freezes ([#957](https://github.com/dashpay/platform/issues/957))
* **dashmate:** yaml package security vulnerability ([#975](https://github.com/dashpay/platform/issues/975))
* **dashmate:** already configured preset is ignored ([#974](https://github.com/dashpay/platform/issues/974))
* **test-suite:** expects identities for invalid mns ([#968](https://github.com/dashpay/platform/issues/968))


### Build System

* **dashmate:** remove MacOs malicious software warning ([#977](https://github.com/dashpay/platform/issues/977))

## [0.24.0-dev.24](https://github.com/dashpay/platform/compare/v0.24.0-dev.23...v0.24.0-dev.24) (2023-04-24)


### ⚠ BREAKING CHANGES

* Previous state might be invalid since to new sync mn identities logic (#965)

### Features

* **wasm-dpp:** state_transition_fee_validator binding and tests ([#874](https://github.com/dashpay/platform/issues/874))


### Bug Fixes

* **dashmate:** wrong envoy config path on windows ([#949](https://github.com/dashpay/platform/issues/949))
* **drive:** updating a masternode identity with invalid entry from SML ([#965](https://github.com/dashpay/platform/issues/965))

## [0.24.0-dev.23](https://github.com/dashpay/platform/compare/v0.24.0-dev.22...v0.24.0-dev.23) (2023-04-20)


### Build System

* **dashmate:** fix removing unnecessary packages

## [0.24.0-dev.22](https://github.com/dashpay/platform/compare/v0.24.0-dev.21...v0.24.0-dev.22) (2023-04-20)


### Continuous Integration

* remove drive node.js binding release


### Build System

* **dashmate:** build arm64 deb file instead of armel ([#943](https://github.com/dashpay/platform/issues/943))

## [0.24.0-dev.21](https://github.com/dashpay/platform/compare/v0.24.0-dev.20...v0.24.0-dev.21) (2023-04-19)


### Features

* **dashmate:** check system requirements before setup ([#935](https://github.com/dashpay/platform/issues/935))


### Bug Fixes

* **dashmate:** сan't find begoo/index with `yarn dashmate setup` ([#933](https://github.com/dashpay/platform/issues/933))


### Miscellaneous Chores

* **dashmate:** remove deprecation warning on start ([#925](https://github.com/dashpay/platform/issues/925))


### Continuous Integration

* fix release workflow


### Build System

* **dashmate:** remove deb release docker dependencies ([#934](https://github.com/dashpay/platform/issues/934))
* **dashmate:** build only linux tarballs ([#936](https://github.com/dashpay/platform/issues/936))


## [0.24.0-dev.20](https://github.com/dashpay/platform/compare/v0.24.0-dev.19...v0.24.0-dev.20) (2023-04-18)


### ⚠ BREAKING CHANGES

* Some wasm-dpp APIs are different to js-dpp ones. The only visible divergencies were addressed, but many others might've been left unnoticed (#848)

### Features

* **drive:** handle quorum rotation failure ([#858](https://github.com/dashpay/platform/issues/858))
* wasm-dpp integration ([#848](https://github.com/dashpay/platform/issues/848))

## [0.24.0-dev.19](https://github.com/dashpay/platform/compare/v0.24.0-dev.18...v0.24.0-dev.19) (2023-04-17)

### Continuous Integration

* test release workflow

## [0.24.0-dev.18](https://github.com/dashpay/platform/compare/v0.24.0-dev.17...v0.24.0-dev.18) (2023-04-14)


### Features

* **dashmate:** build linux tarballs ([#887](https://github.com/dashpay/platform/issues/887))
* **dashmate:** build services before restart ([#894](https://github.com/dashpay/platform/issues/894))
* **dashmate:** exit status with 2 if it's not running ([#896](https://github.com/dashpay/platform/issues/896))
* **dashmate:** implement http json rpc api ([#888](https://github.com/dashpay/platform/issues/888))
* **dashmate:** tenderdash latest block time in status ([#906](https://github.com/dashpay/platform/issues/906))
* **dpp:** serialize consensus errors ([#871](https://github.com/dashpay/platform/issues/871))
* drive verification c bindings ([#860](https://github.com/dashpay/platform/issues/860))


### Bug Fixes

* DAPI still expected on normal masternodes ([#904](https://github.com/dashpay/platform/issues/904))
* **dapi-client:** platform port is ignored from SML ([#903](https://github.com/dashpay/platform/issues/903))
* **dashmate:** api binds to all interfaces ([#893](https://github.com/dashpay/platform/issues/893))
* **dashmate:** dashmate helper is running under root user ([#895](https://github.com/dashpay/platform/issues/895))
* **dashmate:** dashmate logic doesn't recognize it's ran from helper ([#902](https://github.com/dashpay/platform/issues/902))
* **dashmate:** missing rawblock zmq message in core config ([#770](https://github.com/dashpay/platform/issues/770))
* **dashmate:** undefined wallet for dash-cli ([#786](https://github.com/dashpay/platform/issues/786))
* **dpp:** various fixes in DPP and system contracts ([#907](https://github.com/dashpay/platform/issues/907))
* **drive:** non-deterministic run of mn identities sync ([#910](https://github.com/dashpay/platform/issues/910))
* **drive:** total HPMNs contains all masternodes ([#911](https://github.com/dashpay/platform/issues/911))
* identifier deserialization doesn't work for bincode ([#885](https://github.com/dashpay/platform/issues/885))
* llmqType must be equal to one of the allowed values ([#884](https://github.com/dashpay/platform/issues/884))
* possible overflow issues ([#877](https://github.com/dashpay/platform/issues/877))


### Miscellaneous Chores

* **dashmate:** update production dashcore versions for mainnet and testnet ([#840](https://github.com/dashpay/platform/issues/840))
* **sdk:** add eslint ([#829](https://github.com/dashpay/platform/issues/829))


### Continuous Integration

* sign MacOs Dashmate release ([#890](https://github.com/dashpay/platform/issues/890))

## [0.24.0-dev.17](https://github.com/dashpay/platform/compare/v0.24.0-dev.16...v0.24.0-dev.17) (2023-04-04)


### Features

* **dashmate:** add new quroum in dashcore config ([#862](https://github.com/dashpay/platform/issues/862))
* **dashmate:** enable platform option ([#853](https://github.com/dashpay/platform/issues/853))
* **dashmate:** generate self-signed certificates in the `setup` command ([#869](https://github.com/dashpay/platform/issues/869))
* **dashmate:** high-performance nodes registration with `setup` command ([#794](https://github.com/dashpay/platform/issues/794))
* **dashmate:** hint to setup a node on start failure ([#866](https://github.com/dashpay/platform/issues/866))
* **dpp:** add fees API  to rust wasm bindings ([#830](https://github.com/dashpay/platform/issues/830))
* **dpp:** optional execution context in rs-dpp ([#811](https://github.com/dashpay/platform/issues/811))
* **dpp:** state transition applicator ([#878](https://github.com/dashpay/platform/issues/878))
* **rs-dpp:** migrate fees from js-dpp v0.24 ([#851](https://github.com/dashpay/platform/issues/851))
* state transition conversion ([#844](https://github.com/dashpay/platform/issues/844))
* **wasm-dpp:** add tests for state transition basic validator ([#857](https://github.com/dashpay/platform/issues/857))
* **wasm-dpp:** DashPlatformProtocol tests ([#841](https://github.com/dashpay/platform/issues/841))
* **wasm-dpp:** identity transitions additional functionality ([#855](https://github.com/dashpay/platform/issues/855))
* **wasm-dpp:** implement validateStateTransitionStateFactory tests ([#861](https://github.com/dashpay/platform/issues/861))
* **wasm-dpp:** provide external entropy generator to document factory ([#845](https://github.com/dashpay/platform/issues/845))
* **wasm-dpp:** validate_state_transition_identity_signature binding and test ([#865](https://github.com/dashpay/platform/issues/865))


### Bug Fixes

* **dashmate:** ambiguous validation errors for file certificates ([#870](https://github.com/dashpay/platform/issues/870))
* **dashmate:** config.isPlatformEnabled is not a function ([#872](https://github.com/dashpay/platform/issues/872))
* **dpp:** incorrect public key validator schema in ST Facade ([#854](https://github.com/dashpay/platform/issues/854))
* **scripts:** update configure_test_network for hpmn ([#863](https://github.com/dashpay/platform/issues/863))
* **wasm-dpp:** fix decoding protocol version varint error to match previous implementation ([#849](https://github.com/dashpay/platform/issues/849))


### Tests

* **rs-drive:** fix path to EntropyGenerator ([#856](https://github.com/dashpay/platform/issues/856))


### Code Refactoring

* **dashmate:** always keep platform config ([#868](https://github.com/dashpay/platform/issues/868))
* **dashmate:** move core devnet options to subsection ([#867](https://github.com/dashpay/platform/issues/867))


### Build System

* **dashmate:** fix deb package release  ([#864](https://github.com/dashpay/platform/issues/864))
* operations in configure.sh script were in a wrong order ([#876](https://github.com/dashpay/platform/issues/876))


### Miscellaneous Chores

* `rs-dpp` and `wasm-dpp` updates for integration ([#875](https://github.com/dashpay/platform/issues/875))
* **wasm-dpp:** proper identifier buffer inheritance ([#879](https://github.com/dashpay/platform/issues/879))

## [0.24.0-dev.16](https://github.com/dashpay/platform/compare/v0.24.0-dev.15...v0.24.0-dev.16) (2023-03-22)


### Features

* **wasm-dpp:** decodeProtocolEntity test ([#834](https://github.com/dashpay/platform/issues/834))


### Code Refactoring

* change dpp to be based on platform value ([#809](https://github.com/dashpay/platform/issues/809))


### Miscellaneous Chores

* **dashmate:** upgrade dashcore version for local networks ([#843](https://github.com/dashpay/platform/issues/843))


### Continuous Integration

* dashmate release script fix ([#846](https://github.com/dashpay/platform/issues/846))

## [0.24.0-dev.15](https://github.com/dashpay/platform/compare/v0.24.0-dev.14...v0.24.0-dev.15) (2023-03-21)


### Miscellaneous Chores

* **drive:** temporary disable payout script ([#835](https://github.com/dashpay/platform/issues/835))


### Continuous Integration

* dashmate release script fix ([#836](https://github.com/dashpay/platform/issues/836))

## [0.24.0-dev.14](https://github.com/dashpay/platform/compare/v0.24.0-dev.13...v0.24.0-dev.14) (2023-03-20)


### ⚠ BREAKING CHANGES

* core version 19.0-beta integration (#771)

### Features

* better Core 19 support ([#832](https://github.com/dashpay/platform/issues/832))
* core version 19.0-beta integration ([#771](https://github.com/dashpay/platform/issues/771))
* **dashmate:** register HPMN for local network ([#796](https://github.com/dashpay/platform/issues/796))
* **dasmate:** pack release script ([#781](https://github.com/dashpay/platform/issues/781))
* **dpp:** identity facade ([#782](https://github.com/dashpay/platform/issues/782))
* **dpp:** integration tests for wasm-dpp document transitions ([#777](https://github.com/dashpay/platform/issues/777))
* **dpp:** wasm bindings for Documents related validations ([#709](https://github.com/dashpay/platform/issues/709))
* **dpp:** wasm-dpp: integration tests for document ([#762](https://github.com/dashpay/platform/issues/762))
* Identity v2 ([#705](https://github.com/dashpay/platform/issues/705))
* platform value abstraction ([#805](https://github.com/dashpay/platform/issues/805))
* proposer signaling of protocol version upgrade and fork activation ([#778](https://github.com/dashpay/platform/issues/778))
* register system data contracts in RS Drive ([#776](https://github.com/dashpay/platform/issues/776))
* **rs-dpp:**  dashpay datatrigger toUserIds better validation ([#799](https://github.com/dashpay/platform/issues/799))
* **rs-dpp:** backport of index_definitions.unique validation ([#802](https://github.com/dashpay/platform/issues/802))
* **rs-dpp:** backports of identity/stateTransition from js-dpp ([#800](https://github.com/dashpay/platform/issues/800))
* **rs-dpp:** introduce `StateTransitionFactory` ([#810](https://github.com/dashpay/platform/issues/810))
* **rs-dpp:** validate indices are backwards compatible backport ([#797](https://github.com/dashpay/platform/issues/797))
* **rs-drive:** verification feature ([#803](https://github.com/dashpay/platform/issues/803))
* **wasm dpp:** validate state transition key signature ([#806](https://github.com/dashpay/platform/issues/806))
* **wasm-dpp:**  wasm bindings for Document Transitions  ([#707](https://github.com/dashpay/platform/issues/707))
* **wasm-dpp:** async state repository ([#766](https://github.com/dashpay/platform/issues/766))
* **wasm-dpp:** data contract facade ([#716](https://github.com/dashpay/platform/issues/716))
* **wasm-dpp:** Fix metadata, metadata tests and backport v23 matedata changes into wasm-dpp ([#819](https://github.com/dashpay/platform/issues/819))
* **wasm-dpp:** implement identity update transition ([#748](https://github.com/dashpay/platform/issues/748))
* **wasm-dpp:** integration tests validate data contract update transition ([#812](https://github.com/dashpay/platform/issues/812))
* **wasm-dpp:** protocol version validator tests ([#823](https://github.com/dashpay/platform/issues/823))
* **wasm-dpp:** remove unused documents factory tests ([#828](https://github.com/dashpay/platform/issues/828))
* **wasm-dpp:** state transition facade ([#814](https://github.com/dashpay/platform/issues/814))
* withdrawals status sync ([#679](https://github.com/dashpay/platform/issues/679))


### Bug Fixes

* **ci:** fix release workflow syntax error ([#808](https://github.com/dashpay/platform/issues/808))
* **dashmate:** make dashmate helper run commands as host user ([#765](https://github.com/dashpay/platform/issues/765))
* **dashmate:** visual fixes for dashmate status ([#787](https://github.com/dashpay/platform/issues/787))
* **dpp:** update jsonschema-rs and enable tests ([#780](https://github.com/dashpay/platform/issues/780))
* **rs-dpp:** fetch latest core chain locked height misuse ([#789](https://github.com/dashpay/platform/issues/789))
* update webpack to resolve npm audit error ([#822](https://github.com/dashpay/platform/issues/822))
* **wasm-dpp:** Identifier and its tests ([#821](https://github.com/dashpay/platform/issues/821))


### Code Refactoring

* remove various Clippy warnings ([#793](https://github.com/dashpay/platform/issues/793))
* **rs-dpp:** bring error type to one format ([#804](https://github.com/dashpay/platform/issues/804))


### Build System

* add missing deps ([#824](https://github.com/dashpay/platform/issues/824))


### Documentation

* supported Node.JS version and dashmate command description ([#825](https://github.com/dashpay/platform/issues/825))


### Miscellaneous Chores

* **drive:** temporary disable unstable withdrawal logic ([#831](https://github.com/dashpay/platform/issues/831))
* **sdk:** add eslint ([#827](https://github.com/dashpay/platform/issues/827))

## [0.24.0-dev.13](https://github.com/dashpay/platform/compare/v0.24.0-dev.12...v0.24.0-dev.13) (2023-01-30)


### ⚠ BREAKING CHANGES

* **dapi:** use single envoy port for all connections (#752)

### Features

* allow to get drive status from dashmate helper ([#749](https://github.com/dashpay/platform/issues/749))
* allow to get drive's status from dashmate helper ([#755](https://github.com/dashpay/platform/issues/755))
* **dapi:** use single envoy port for all connections ([#752](https://github.com/dashpay/platform/issues/752))
* **dashmate:** update Core to 18.2.0 ([#735](https://github.com/dashpay/platform/issues/735))
* **drive:** ABCI context logger ([#693](https://github.com/dashpay/platform/issues/693))
* **drive:** log contractId in deliverTx handler ([#730](https://github.com/dashpay/platform/issues/730))
* **drive:** log number of refunded epochs ([#729](https://github.com/dashpay/platform/issues/729))
* integrate wasm Document into JS tests ([#644](https://github.com/dashpay/platform/issues/644))
* varint protocol version ([#758](https://github.com/dashpay/platform/issues/758))
* **wasm-dpp:** implement function to produce generics from JsValue ([#712](https://github.com/dashpay/platform/issues/712))
* **wasm-dpp:** implement identity create transition ([#697](https://github.com/dashpay/platform/issues/697))
* **wasm-dpp:** Wasm dpp integration tests validate data contract factory ([#751](https://github.com/dashpay/platform/issues/751))


### Bug Fixes

* **dashmate:** Dash Core container is unable to restart properly under WSL ([#736](https://github.com/dashpay/platform/issues/736))
* **dashmate:** fix migration for configs without platform ([#738](https://github.com/dashpay/platform/issues/738))
* **dashmate:** migrations fixes ([#759](https://github.com/dashpay/platform/issues/759))
* **dpp:** existing property in a new index ([#694](https://github.com/dashpay/platform/issues/694))
* ua-parser-js vulnerability  ([#756](https://github.com/dashpay/platform/issues/756))


### Miscellaneous Chores

* **dashmate:** sync Tenderdash config with Tenderdash v0.10.0-dev.8 ([#746](https://github.com/dashpay/platform/issues/746))
* update Tenderdash to 0.10.0-dev.8 ([#741](https://github.com/dashpay/platform/issues/741))


### Code Refactoring

* initial cleanup of Drive and DPP document types ([#750](https://github.com/dashpay/platform/issues/750))
* use a trait to remove the need for some functions ([#747](https://github.com/dashpay/platform/issues/747))

### [0.23.2](https://github.com/dashpay/platform/compare/v0.23.0...v0.23.2) (2023-01-19)


### Bug Fixes

* **dapi-client:** missing fetch-polyfill in npm installation ([#743](https://github.com/dashpay/platform/issues/743))
* **dapi-grpc:** unsafe-eval errors in protobuf js files ([#713](https://github.com/dashpay/platform/issues/713))


### [0.23.1](https://github.com/dashpay/platform/compare/v0.23.0...v0.23.1) (2023-01-13)


### Features

* update Platform to Core v18.2.0-rc.4 ([#701](https://github.com/dashpay/platform/issues/701))


### Continuous Integration

* fix incorrect github actions templating syntax ([#689](https://github.com/dashpay/platform/issues/689))
* release dashmate packages ([#669](https://github.com/dashpay/platform/issues/669))


### Miscellaneous Chores

* **dapi-client:** replace axios with fetch ([#690](https://github.com/dashpay/platform/issues/690))
* **dashmate:** update base config to core 18.2.0 ([#706](https://github.com/dashpay/platform/issues/706))
* replace grpc-web with @improbable-eng/grpc-web ([#628](https://github.com/dashpay/platform/issues/628))
* set core in base config to v18.1.1 ([#720](https://github.com/dashpay/platform/issues/720))
* update platform chain id ([#703](https://github.com/dashpay/platform/issues/703))
* use core 18.2.0 on testnet ([#725](https://github.com/dashpay/platform/issues/725))



## [0.24.0-dev.12](https://github.com/dashpay/platform/compare/v0.24.0-dev.11...v0.24.0-dev.12) (2023-01-11)


### ⚠ BREAKING CHANGES

* AbstractStateTransition#calculateFees removed
* State transition fees are calculated differently so previous block data is not valid anymore
* Added new tree to initial structure so previous state is not valid anymore

### Features

* credit refunds ([#662](https://github.com/dashpay/platform/issues/662))
* **dashmate:** additional dashd options ([#692](https://github.com/dashpay/platform/issues/692))
* **dashmate:** pass ZeroSSL as command line parameter ([#651](https://github.com/dashpay/platform/issues/651))
* **dashmate:** remove axios from zerossl requests
* **dashmate:** remove axios from zerossl requests
* **dpp:** AbstractConsensusError tests and extensions ([#670](https://github.com/dashpay/platform/issues/670))
* **dpp:** Data Contract Update Transition wasm binding ([#696](https://github.com/dashpay/platform/issues/696))
* **drive:** do not switch to validator quorum which will be removed soon ([#616](https://github.com/dashpay/platform/issues/616))
* multiple documents changes per batch and support for GroveDB 0.9 ([#699](https://github.com/dashpay/platform/issues/699))
* update Platform to Core v18.2.0-rc.4 ([#701](https://github.com/dashpay/platform/issues/701))


### Bug Fixes

* **dashmate:** ZeroSSL certificate cannot be downloaded ([#718](https://github.com/dashpay/platform/issues/718))
* **drive:** merk caching in contract caching (irony) ([#710](https://github.com/dashpay/platform/issues/710))
* find_duplicates_by_id.rs not compiling ([#702](https://github.com/dashpay/platform/issues/702))
* Starcounter-Jack JSON-Patch Prototype Pollution vulnerability ([#708](https://github.com/dashpay/platform/issues/708))


### Code Refactoring

* **dashmate:** status command ([#660](https://github.com/dashpay/platform/issues/660))


### Continuous Integration

* fix incorrect github actions templating syntax ([#689](https://github.com/dashpay/platform/issues/689))
* increase cache-max-size ([#704](https://github.com/dashpay/platform/issues/704))


### Miscellaneous Chores

* **dapi-client:** replace axios with fetch ([#690](https://github.com/dashpay/platform/issues/690))
* **dashmate:** update base config to core 18.2.0 ([#706](https://github.com/dashpay/platform/issues/706))
* **dpp:** wasm dpp data contract test error types ([#684](https://github.com/dashpay/platform/issues/684))
* **drive:** log synchronize identities properly ([#686](https://github.com/dashpay/platform/issues/686))
* set core in base config to v18.1.1 ([#720](https://github.com/dashpay/platform/issues/720))

## [0.24.0-dev.11](https://github.com/dashpay/platform/compare/v0.24.0-dev.10...v0.24.0-dev.11) (2022-12-20)


### Bug Fixes

* **dashmate:** setDay is not a function ([#677](https://github.com/dashpay/platform/issues/677))
* **dashmate:** ZeroSSL certificates are not saved in WSL ([#676](https://github.com/dashpay/platform/issues/676))
* **drive:** initChain handler is not idempotent ([#675](https://github.com/dashpay/platform/issues/675))


### Continuous Integration

* bump action versions ([#678](https://github.com/dashpay/platform/issues/678))
* release dashmate packages ([#669](https://github.com/dashpay/platform/issues/669))


### Miscellaneous Chores

* **dashmate:** update tenderdash up to 0.10.0-dev.6 ([#674](https://github.com/dashpay/platform/issues/674))

## [0.24.0-dev.10](https://github.com/dashpay/platform/compare/v0.24.0-dev.9...v0.24.0-dev.10) (2022-12-15)


### Features

* Consensus Errors and ValidationResult bindings ([#643](https://github.com/dashpay/platform/issues/643))


### Bug Fixes

* **dashmate**: outdated Drive and DAPI images ([#668](https://github.com/dashpay/platform/issues/668))


### Continuous Integration

* fix entrypoint path in release workflow ([#671](https://github.com/dashpay/platform/issues/671))



## [0.24.0-dev.9](https://github.com/dashpay/platform/compare/v0.23.0...v0.24.0-dev.9) (2022-12-14)


### ⚠ BREAKING CHANGES

* Previous data is not compatible with new Drive

### Features

* average estimated processing fees ([#642](https://github.com/dashpay/platform/issues/642))
* **dpp:** bls adapter for WASM DPP ([#633](https://github.com/dashpay/platform/issues/633))
* **drive:** add time and protocolVersion fields to query metadata response ([#611](https://github.com/dashpay/platform/issues/611))
* **drive:** provide latest core chain lock on init chain ([#659](https://github.com/dashpay/platform/issues/659))
* **drive:** support for V0.7 of groveDB ([#665](https://github.com/dashpay/platform/issues/665))
* **drive:** use proposal block execution context in state repository ([#653](https://github.com/dashpay/platform/issues/653))
* **drive:** use single block execution context ([#627](https://github.com/dashpay/platform/issues/627))


### Bug Fixes

* build not working because of deprecated wasm-bindgen feature ([#639](https://github.com/dashpay/platform/issues/639))
* **dapi:** fail to reconnect to tenderdash in case of ENOTFOUND ([#621](https://github.com/dashpay/platform/issues/621))
* **dashmate:** broken helper docker image ([#630](https://github.com/dashpay/platform/issues/630))
* **dashmate:** ZeroSSL certificate renewal ([#624](https://github.com/dashpay/platform/issues/624))
* **drive:** multi transactions doesn't work properly ([#636](https://github.com/dashpay/platform/issues/636))


### Performance Improvements

* **drive:** do not call process proposal after prepare ([#656](https://github.com/dashpay/platform/issues/656))


### Code Refactoring

* use FeeResult to collect block fees ([#652](https://github.com/dashpay/platform/issues/652))


### Build System

* support Rust and JS packages

### Continuous Integration

* code scanning ([#626](https://github.com/dashpay/platform/issues/626))
* fix env syntax in release workflow ([#664](https://github.com/dashpay/platform/issues/664))
* support Rust and JS packages


### Miscellaneous Chores

* **dpp:** switch dpp to working revision
* **drive:** comprehensive logging for same block execution ([#657](https://github.com/dashpay/platform/issues/657))
* ensure consistent Rust dependencies ([#658](https://github.com/dashpay/platform/issues/658))
* remove lodash per-method deps ([#661](https://github.com/dashpay/platform/issues/661))
* replace grpc-web with @improbable-eng/grpc-web ([#628](https://github.com/dashpay/platform/issues/628))
* merged rs-platform monorepo


## [0.23.0](https://github.com/dashpay/platform/compare/v0.22.16...v0.23.0) (2022-12-05)

### ⚠ BREAKING CHANGES

* Dash Core version lower than 18.1.0 is not supported
* Blockchain data and state structure are changed and incompatible with previous versions
* `getIdentityIdsByPublicKeyHash` endpoint is removed. `getIdentitiesByPublicKeyHash` now responds with an array of identities, instead of an array of cbored arrays of identities. (#437)
* Data Contract indices must have 'asc' order (#435)
* Document query logic can behave differently in some cases (#398)
* Identity master key can be used only to update identity
* Use plain proRegTx for masternode identifier
* Incompatible wallet-lib storage format


### Features

* implement headers first synchronization ([#428](https://github.com/dashpay/platform/issues/428))
* precise storage fees ([#619](https://github.com/dashpay/platform/issues/619))
* **dashmate:** add `core reindex` command ([#533](https://github.com/dashevo/platform/issues/533)), closes [#538](https://github.com/dashevo/platform/issues/538)
* update Core to 18.1.0-rc.1 ([#544](https://github.com/dashevo/platform/issues/544), [#526](https://github.com/dashevo/platform/issues/526), [#511](https://github.com/dashevo/platform/issues/511))
* **drive:** fees distribution ([#458](https://github.com/dashevo/platform/issues/458), [#474](https://github.com/dashevo/platform/issues/474), [#484](https://github.com/dashevo/platform/issues/484))
* bench suite ([#335](https://github.com/dashevo/platform/issues/335))
* **bench-suite:** add fees to documents benchmark ([#379](https://github.com/dashevo/platform/issues/379))
* **bench-suite:** function benchmark and other improvements ([#344](https://github.com/dashevo/platform/issues/344))
* **bench:** state transition benchmark ([#418](https://github.com/dashevo/platform/issues/418))
* **dashmate:** add --force flag to stop command ([#434](https://github.com/dashevo/platform/issues/434))
* **dashmate:** upgrade docker compose to v2 ([#441](https://github.com/dashevo/platform/issues/441))
* **dpp:** allow 1 char document type and 1 char property name ([#445](https://github.com/dashevo/platform/issues/445))
* integrate dash-spv into monorepo
* limit the number of shares for masternode by 16 ([#432](https://github.com/dashevo/platform/issues/432))
* re-enable proof responses ([#440](https://github.com/dashevo/platform/issues/440))
* create withdrawal keys for masternode identities ([#320](https://github.com/dashevo/platform/issues/320))
* **dpp:** BIP13_SCRIPT_HASH identity public key type ([#353](https://github.com/dashevo/platform/issues/353))
* Identity master key can be used only to update identity ([#384](https://github.com/dashevo/platform/issues/384))
* identity public key proofs ([#349](https://github.com/dashevo/platform/issues/349))
* Integrate with Tenderdash ABCI++ ([#314](https://github.com/dashevo/platform/issues/314))
* **dpp:** add `withdraw` purpose for `IdentityPublicKey` ([#317](https://github.com/dashevo/platform/issues/317))
* update identity ([#292](https://github.com/dashevo/platform/issues/292), [#477](https://github.com/dashevo/platform/issues/477), [#421](https://github.com/dashevo/platform/issues/421))
* **wallet-lib:** rework storage for multiple key-chains ([#231](https://github.com/dashevo/platform/issues/231))
* **wallet-lib:** satoshisBalanceImpact in transaction history ([#319](https://github.com/dashevo/platform/issues/319))
* calculate state transition fees using operations ([#376](https://github.com/dashevo/platform/issues/376), [#387](https://github.com/dashevo/platform/issues/387), [#369](https://github.com/dashevo/platform/issues/369), [#370](https://github.com/dashevo/platform/issues/370), [#392](https://github.com/dashevo/platform/issues/392), [#444](https://github.com/dashevo/platform/issues/444))
* **dashmate:** add hardcoded stop grace time before killing services ([#536](https://github.com/dashevo/platform/issues/536))


### Bug Fixes

* **dapi:** Cannot read properties of undefined (reading 'PoSePenalty') ([#612](https://github.com/dashpay/platform/issues/612))
* **test-suite:** `featureFlags` test was awaiting blocks that have not been produced ([#602](https://github.com/dashpay/platform/issues/602))
* **dapi:** caching of headers prone to reorgs ([#578](https://github.com/dashpay/platform/issues/578))
* 0.23-dev.10 version installed instead of alpha ([#581](https://github.com/dashpay/platform/issues/581))
* **dashmate:** Core RPC is not responding ([#575](https://github.com/dashevo/platform/issues/575))
* **drive:** db is in readonly mode due to the active transaction ([#567](https://github.com/dashevo/platform/issues/567))
* **sdk:** `platform.initialize` is not a function ([#555](https://github.com/dashevo/platform/issues/555))
* **dapi-client:** metadata.get is not a function ([#492](https://github.com/dashevo/platform/issues/492), [#531](https://github.com/dashevo/platform/issues/531))
* **dashmate:** homedir fs.exists check ([#537](https://github.com/dashevo/platform/issues/537))
* **drive:** masternode identities sync from beginning after restart ([#542](https://github.com/dashevo/platform/issues/542))
* **dashmate:** DockerComposeError undefined ([#480](https://github.com/dashevo/platform/issues/480), [#513](https://github.com/dashevo/platform/issues/513))
* security vulnerability in elliptic library ([#501](https://github.com/dashevo/platform/issues/501))
* test suite image and environment problems in ci ([#505](https://github.com/dashevo/platform/issues/505))
* **test-suite:** failing assertions due to replication lag ([#502](https://github.com/dashevo/platform/issues/502))
* **dapi:** getStatus errored if masternode is banned ([#496](https://github.com/dashevo/platform/issues/496))
* Drive and DAPI expect data available on H+1 block ([#497](https://github.com/dashevo/platform/issues/497))
* **test-suite:** failing tests due to replication latency ([#500](https://github.com/dashevo/platform/issues/500))
* DAPI client requests one local single node only ([#479](https://github.com/dashevo/platform/issues/479))
* **dapi-client:** node can be marked as banned on retriable error ([#482](https://github.com/dashevo/platform/issues/482))
* **dashmate:** gRPC stream timeout ([#481](https://github.com/dashevo/platform/issues/481))
* **sdk:** cannot read properties of null (reading 'getMetadata') ([#488](https://github.com/dashevo/platform/issues/488))
* **test-suite:** invalid transaction: Missing inputs ([#494](https://github.com/dashevo/platform/issues/494))
* **drive:** invalid previous block time ([#475](https://github.com/dashevo/platform/issues/475))
* **drive:** various fixes in synchronize masternode identities logic and logging ([#461](https://github.com/dashevo/platform/issues/461))
* **dpp:** data contract index update validation ([#427](https://github.com/dashevo/platform/issues/427))
* **drive:** change transaction is started check ([#451](https://github.com/dashevo/platform/issues/451))
* **wallet-lib:** separate persistent storage by walletId ([#407](https://github.com/dashevo/platform/issues/407))
* change allowed security level for withdrawal purpose to critical ([#352](https://github.com/dashevo/platform/issues/352))
* **dapi-grpc:** outdated autogenerated code ([#331](https://github.com/dashevo/platform/issues/331))
* **wallet-lib:** hook tx chain broadcast on mempool response ([#388](https://github.com/dashevo/platform/issues/388))
* **dashmate:** config/core/miner must have required property 'interval' ([#311](https://github.com/dashevo/platform/issues/311))
* do not hash proRegTx for masternode identifier ([#318](https://github.com/dashevo/platform/issues/318))
* **dpp:** cannot read properties of null (reading 'getBalance') ([#549](https://github.com/dashevo/platform/issues/549))
* **dashmate**: can't use local seed as a wallet ([#538](https://github.com/dashevo/platform/issues/538))


### Miscellaneous Chores

* add yarn cache ([#637](https://github.com/dashpay/platform/issues/637))
* **drive:** update RS Drive to 0.23.0-dev.9 ([#588](https://github.com/dashpay/platform/issues/588))
* **dashmate:** update testnet credentials ([#571](https://github.com/dashevo/platform/issues/571))
* **dapi-client:** update dapi addresses white list ([#574](https://github.com/dashevo/platform/issues/574))
* **dashmate:** use latest DAPI and Drive 0.23.0-alpha ([#568](https://github.com/dashevo/platform/issues/568))
* **drive:** correct typo in debug message ([#535](https://github.com/dashevo/platform/issues/535))
* **test-suite:** remove unused merk dependency ([#547](https://github.com/dashevo/platform/issues/547))
* clean up dependencies ([#534](https://github.com/dashevo/platform/issues/534))
* **scripts:** remove comment from env key ([#532](https://github.com/dashevo/platform/issues/532))
* **test-suite:** move wallet storage persistence in the outer folder ([#416](https://github.com/dashevo/platform/issues/416))
* various linter fixes across packages ([#465](https://github.com/dashevo/platform/issues/465))
* **dpp:** allow only `asc` order for indices ([#435](https://github.com/dashevo/platform/issues/435))
* **drive:** log synchronize masternode identities ([#449](https://github.com/dashevo/platform/issues/449))
* **drive:** add more block execution timers ([#329](https://github.com/dashevo/platform/issues/329))
* **scripts**: remove dashmate update ([#550](https://github.com/dashevo/platform/issues/550))


### Performance Improvements

* **dapi:** cache block headers and chainlocks ([#235](https://github.com/dashevo/platform/issues/235), [#296](https://github.com/dashevo/platform/issues/296))
* **dapi:** remove unnecessary Core RPC calls for core streams ([#194](https://github.com/dashevo/platform/issues/194))


### Code Refactoring

* **drive:** use RS Drive query validation logic ([#398](https://github.com/dashevo/platform/issues/398))
* simplified public key to identity structure ([#437](https://github.com/dashevo/platform/issues/437))
* **wallet-lib:** storage layer refactoring ([#232](https://github.com/dashevo/platform/issues/232))


### Tests

* **dapi:** fix broken subscribeToNewBlockHeaders test ([#508](https://github.com/dashevo/platform/issues/508))
* **dapi:** rename test files to mach the naming convention ([#509](https://github.com/dashevo/platform/issues/509))
* **dpp:** fix invalid findIndexDuplicates test in DPP ([#448](https://github.com/dashevo/platform/issues/448))
* **wallet-lib:** fixes wallet.spec.js


### Continuous Integration

* update deps ([#591](https://github.com/dashpay/platform/issues/591))
* add dashmate and test suite ([#551](https://github.com/dashevo/platform/issues/551), [#576](https://github.com/dashevo/platform/issues/576))
* add `latest-dev` docker tag ([#382](https://github.com/dashevo/platform/issues/382))
* enable multiarch builds ([#316](https://github.com/dashevo/platform/issues/316))
* docker images incorrectly tagged with v ([#413](https://github.com/dashevo/platform/issues/413))


### Documentation

* GitHub org change updates ([#590](https://github.com/dashpay/platform/issues/590))
* update URL from dashevo to dashpay ([#579](https://github.com/dashpay/platform/issues/579))
* **dashmate:** add troubleshooting section ([#431](https://github.com/dashevo/platform/issues/431))
* **dashmate:** update dashmate documentation ([#459](https://github.com/dashevo/platform/issues/459))
* **sdk:** update outdated documentation ([#463](https://github.com/dashevo/platform/issues/463))
* update badges in individual package readmes ([#361](https://github.com/dashevo/platform/issues/361))


### Build System

* **dapi-grpc:** update protoc builder image ([#553](https://github.com/dashpay/platform/issues/553), [#647](https://github.com/dashpay/platform/issues/647))
* remove buildx bake workaround ([#541](https://github.com/dashpay/platform/issues/541))
* release arbitrary pre-release tags ([#552](https://github.com/dashevo/platform/issues/552), [#558](https://github.com/dashevo/platform/issues/558), [#560](https://github.com/dashevo/platform/issues/560), [#562](https://github.com/dashevo/platform/issues/562), [#566](https://github.com/dashevo/platform/issues/566))
* `yarn install` fails with Node.JS 16.17.0 ([#507](https://github.com/dashevo/platform/issues/507))
* new node alpine image breaks package builds ([#493](https://github.com/dashevo/platform/issues/493))
* **test-suite:** fix docker image build



## [0.23.0-alpha.9](https://github.com/dashpay/platform/compare/v0.23.0-alpha.8...v0.23.0-alpha.9) (2022-11-21)


### ⚠ BREAKING CHANGES

* precise storage fees (#619)

### Features

* precise storage fees ([#619](https://github.com/dashpay/platform/issues/619))


### Bug Fixes

* **dapi:** Cannot read properties of undefined (reading 'PoSePenalty') ([#612](https://github.com/dashpay/platform/issues/612))

## [0.24.0-dev.8](https://github.com/dashpay/platform/compare/v0.24.0-dev.7...v0.24.0-dev.8) (2022-11-18)


### Features

* add `withdrawals` data contract package ([#604](https://github.com/dashpay/platform/issues/604))
* **dpp:** dashpay datatrigger toUserIds better validation ([#620](https://github.com/dashpay/platform/issues/620))
* **drive:** select the most vital validator set quorums ([#617](https://github.com/dashpay/platform/issues/617))


### Bug Fixes

* DataContract.spec.js in wasm-dpp ([#618](https://github.com/dashpay/platform/issues/618))
* renamed method from rs-dpp ([#623](https://github.com/dashpay/platform/issues/623))

## [0.24.0-dev.7](https://github.com/dashpay/platform/compare/v0.23.0-dev.6...v0.24.0-dev.7) (2022-11-07)


### ⚠ BREAKING CHANGES

* **drive:** same block execution (#593)

### Features

* **dpp:** initial RS DPP integration ([#483](https://github.com/dashpay/platform/issues/483))
* **drive:** same block execution ([#593](https://github.com/dashpay/platform/issues/593))


### Bug Fixes

* `featureFlags` test was awaiting blocks that have not been produced ([#602](https://github.com/dashpay/platform/issues/602))
* **dapi:** `getConsensusParamsHandler` was handling wrong Tendermint error ([#601](https://github.com/dashpay/platform/issues/601))
* **dashmate:** invalid testnet TenderDash genesis ([#608](https://github.com/dashpay/platform/issues/608))
* **dashmate:** SSL verification server cannot be started ([#606](https://github.com/dashpay/platform/issues/606))

## [0.23.0-alpha.8](https://github.com/dashpay/platform/compare/v0.23.0-alpha.7...v0.23.0-alpha.8) (2022-11-04)


### Bug Fixes

* **test-suite:** `featureFlags` test was awaiting blocks that have not been produced ([#602](https://github.com/dashpay/platform/issues/602))


### Continuous Integration

* update deps ([#591](https://github.com/dashpay/platform/issues/591))


### Documentation

* GitHub org change updates ([#590](https://github.com/dashpay/platform/issues/590))


## [0.24.0-dev.6](https://github.com/dashpay/platform/compare/v0.24.0-dev.5...v0.24.0-dev.6) (2022-10-26)


### Bug Fixes

* typo `dash-amte` to `dashmate` ([#599](https://github.com/dashpay/platform/issues/599))

## [0.24.0-dev.5](https://github.com/dashpay/platform/compare/v0.24.0-dev.4...v0.24.0-dev.5) (2022-10-26)


### Bug Fixes

* release.yml contained wrong indentation ([#597](https://github.com/dashpay/platform/issues/597))

## [0.24.0-dev.4](https://github.com/dashpay/platform/compare/v0.24.0-dev.3...v0.24.0-dev.4) (2022-10-26)


### Build System

* publish envoy and dashmate-helper docker images after release ([#595](https://github.com/dashpay/platform/issues/595))

## [0.24.0-dev.3](https://github.com/dashpay/platform/compare/v0.23.0-alpha.7...v0.24.0-dev.3) (2022-10-20)


### Features

* **drive:** AssetUnlock transactions processing ([#530](https://github.com/dashpay/platform/issues/530))


## [0.23.0-alpha.7](https://github.com/dashpay/platform/compare/v0.23.0-alpha.6...v0.23.0-alpha.7) (2022-10-18)


### ⚠ BREAKING CHANGES

* Previous data is not compatible with new Drive


### Bug Fixes

* **dapi:** caching of headers prone to reorgs ([#578](https://github.com/dashpay/platform/issues/578))


### Documentation

* update URL from dashevo to dashpay ([#579](https://github.com/dashpay/platform/issues/579))


### Build System

* **dapi-grpc:** update grpc protoc image ([#553](https://github.com/dashpay/platform/issues/553))
* remove buildx bake workaround ([#541](https://github.com/dashpay/platform/issues/541))


### Miscellaneous Chores

* **drive:** update RS Drive to 0.23.0-dev.9 ([#588](https://github.com/dashpay/platform/issues/588))


## [0.24.0-dev.2](https://github.com/dashpay/platform/compare/v0.23.0-alpha.5...v0.24.0-dev.2) (2022-10-13)


### Bug Fixes

* 0.23-dev.10 version installed instead of alpha ([#581](https://github.com/dashpay/platform/issues/581))
* **dapi-client:** temporary use http protocol by default ([#573](https://github.com/dashpay/platform/issues/573))
* **dapi:** caching of headers prone to reorgs ([#578](https://github.com/dashpay/platform/issues/578))


### Documentation

* update URL from dashevo to dashpay ([#579](https://github.com/dashpay/platform/issues/579))


### Tests

* **drive:** synchronizeMasternodeIdentitiesFactory ([#586](https://github.com/dashpay/platform/issues/586))


## [0.23.0-alpha.6](https://github.com/dashpay/platform/compare/v0.23.0-alpha.5...v0.23.0-alpha.6) (2022-10-12)


### Bug Fixes

* 0.23-dev.10 version installed instead of alpha ([#581](https://github.com/dashpay/platform/issues/581))


## [0.23.0-alpha.5](https://github.com/dashevo/platform/compare/v0.23.0-alpha.4...v0.23.0-alpha.5) (2022-10-12)


### Continuous Integration

* run dashmate and test suite workflow ([#576](https://github.com/dashevo/platform/issues/576))


### Miscellaneous Chores

* **dashmate:** update testnet credentials ([#571](https://github.com/dashevo/platform/issues/571))

## [0.23.0-alpha.4](https://github.com/dashevo/platform/compare/v0.23.0-alpha.3...v0.23.0-alpha.4) (2022-10-11)


### Bug Fixes

* **dashmate:** Core RPC is not responding ([#575](https://github.com/dashevo/platform/issues/575))


### Miscellaneous Chores

* **dapi-client:** update dapi addresses white list ([#574](https://github.com/dashevo/platform/issues/574))

## [0.23.0-alpha.3](https://github.com/dashevo/platform/compare/v0.23.0-alpha.2...v0.23.0-alpha.3) (2022-10-10)


### Bug Fixes

* **drive:** db is in readonly mode due to the active transaction ([#567](https://github.com/dashevo/platform/issues/567))


### Build System

* fix release script wrong param names ([#566](https://github.com/dashevo/platform/issues/566))


### Miscellaneous Chores

* **dashmate:** use latest DAPI and Drive 0.23.0-alpha ([#568](https://github.com/dashevo/platform/issues/568))


## [0.24.0-dev.1](https://github.com/dashevo/platform/compare/v0.23.0-alpha.2...v0.24.0-dev.1) (2022-10-07)


### ⚠ BREAKING CHANGES

* migrate to ABCI++ (#464)
* Tenderdash 0.8 and lower not supported anymore

### Features

* **dashmate:** update tenderdash to 0.9.0-dev.1 ([#525](https://github.com/dashevo/platform/issues/525))
* **dashmate:** zeroSSL certificate renewal helper ([#554](https://github.com/dashevo/platform/issues/554))
* **dpp:** add wasm-dpp template package ([#529](https://github.com/dashevo/platform/issues/529))
* implement masternode voting identities ([#467](https://github.com/dashevo/platform/issues/467))
* migrate to ABCI++ ([#464](https://github.com/dashevo/platform/issues/464))
* SSL certificate for DAPI ([#519](https://github.com/dashevo/platform/issues/519))



## [0.23.0-alpha.2](https://github.com/dashevo/platform/compare/v0.23.0-alpha.1...v0.23.0-alpha.2) (2022-10-07)


### Features

* **dashmate:** add `core reindex` command ([#533](https://github.com/dashevo/platform/issues/533)), closes [#538](https://github.com/dashevo/platform/issues/538)


### Bug Fixes

* **sdk:** platform.initialize is not a function ([#555](https://github.com/dashevo/platform/issues/555))


### Continuous Integration

* add dashmate and test suite ([#551](https://github.com/dashevo/platform/issues/551))
* release arbitrary pre-release tags ([#552](https://github.com/dashevo/platform/issues/552))


### Build System

* support alpha prereleases ([#560](https://github.com/dashevo/platform/issues/560), #558](https://github.com/dashevo/platform/issues/558))
* release can be made from any branch if type is defined ([#562](https://github.com/dashevo/platform/issues/562))


## [0.23.0-alpha.1](https://github.com/dashevo/platform/compare/v0.22.16...v0.23.0-alpha.1) (2022-09-28)


### ⚠ BREAKING CHANGES

* Dash Core version lower than 18.1.0-rc.1 is not supported
* Blockchain data and state structure are changed and incompatible with previous versions
* `getIdentityIdsByPublicKeyHash` endpoint is removed. `getIdentitiesByPublicKeyHash` now responds with an array of identities, instead of an array of cbored arrays of identities. (#437)
* Data Contract indices must have 'asc' order (#435)
* Document query logic can behave differently in some cases (#398)
* Identity master key can be used only to update identity
* Use plain proRegTx for masternode identifier
* Incompatible wallet-lib storage format


### Features

* update Core to 18.1.0-rc.1 ([#544](https://github.com/dashevo/platform/issues/544), [#526](https://github.com/dashevo/platform/issues/526), [#511](https://github.com/dashevo/platform/issues/511))
* **drive:** fees distribution ([#458](https://github.com/dashevo/platform/issues/458), [#474](https://github.com/dashevo/platform/issues/474), [#484](https://github.com/dashevo/platform/issues/484))
* bench suite ([#335](https://github.com/dashevo/platform/issues/335))
* **bench-suite:** add fees to documents benchmark ([#379](https://github.com/dashevo/platform/issues/379))
* **bench-suite:** function benchmark and other improvements ([#344](https://github.com/dashevo/platform/issues/344))
* **bench:** state transition benchmark ([#418](https://github.com/dashevo/platform/issues/418))
* **dashmate:** add --force flag to stop command ([#434](https://github.com/dashevo/platform/issues/434))
* **dashmate:** upgrade docker compose to v2 ([#441](https://github.com/dashevo/platform/issues/441))
* **dpp:** allow 1 char document type and 1 char property name ([#445](https://github.com/dashevo/platform/issues/445))
* integrate dash-spv into monorepo
* limit the number of shares for masternode by 16 ([#432](https://github.com/dashevo/platform/issues/432))
* re-enable proof responses ([#440](https://github.com/dashevo/platform/issues/440))
* create withdrawal keys for masternode identities ([#320](https://github.com/dashevo/platform/issues/320))
* **dpp:** BIP13_SCRIPT_HASH identity public key type ([#353](https://github.com/dashevo/platform/issues/353))
* Identity master key can be used only to update identity ([#384](https://github.com/dashevo/platform/issues/384))
* identity public key proofs ([#349](https://github.com/dashevo/platform/issues/349))
* Integrate with Tenderdash ABCI++ ([#314](https://github.com/dashevo/platform/issues/314))
* **dpp:** add `withdraw` purpose for `IdentityPublicKey` ([#317](https://github.com/dashevo/platform/issues/317))
* update identity ([#292](https://github.com/dashevo/platform/issues/292), [#477](https://github.com/dashevo/platform/issues/477), [#421](https://github.com/dashevo/platform/issues/421))
* **wallet-lib:** rework storage for multiple key-chains ([#231](https://github.com/dashevo/platform/issues/231))
* **wallet-lib:** satoshisBalanceImpact in transaction history ([#319](https://github.com/dashevo/platform/issues/319))
* calculate state transition fees using operations ([#376](https://github.com/dashevo/platform/issues/376), [#387](https://github.com/dashevo/platform/issues/387), [#369](https://github.com/dashevo/platform/issues/369), [#370](https://github.com/dashevo/platform/issues/370), [#392](https://github.com/dashevo/platform/issues/392), [#444](https://github.com/dashevo/platform/issues/444))
* **dashmate:** add hardcoded stop grace time before killing services ([#536](https://github.com/dashevo/platform/issues/536))


### Bug Fixes

* **dapi-client:** metadata.get is not a function ([#492](https://github.com/dashevo/platform/issues/492), [#531](https://github.com/dashevo/platform/issues/531))
* **dashmate:** homedir fs.exists check ([#537](https://github.com/dashevo/platform/issues/537))
* **drive:** masternode identities sync from beginning after restart ([#542](https://github.com/dashevo/platform/issues/542))
* **dashmate:** DockerComposeError undefined ([#480](https://github.com/dashevo/platform/issues/480), [#513](https://github.com/dashevo/platform/issues/513))
* security vulnerability in elliptic library ([#501](https://github.com/dashevo/platform/issues/501))
* test suite image and environment problems in ci ([#505](https://github.com/dashevo/platform/issues/505))
* **test-suite:** failing assertions due to replication lag ([#502](https://github.com/dashevo/platform/issues/502))
* **dapi:** getStatus errored if masternode is banned ([#496](https://github.com/dashevo/platform/issues/496))
* Drive and DAPI expect data available on H+1 block ([#497](https://github.com/dashevo/platform/issues/497))
* **test-suite:** failing tests due to replication latency ([#500](https://github.com/dashevo/platform/issues/500))
* DAPI client requests one local single node only ([#479](https://github.com/dashevo/platform/issues/479))
* **dapi-client:** node can be marked as banned on retriable error ([#482](https://github.com/dashevo/platform/issues/482))
* **dashmate:** gRPC stream timeout ([#481](https://github.com/dashevo/platform/issues/481))
* **sdk:** cannot read properties of null (reading 'getMetadata') ([#488](https://github.com/dashevo/platform/issues/488))
* **test-suite:** invalid transaction: Missing inputs ([#494](https://github.com/dashevo/platform/issues/494))
* **drive:** invalid previous block time ([#475](https://github.com/dashevo/platform/issues/475))
* **drive:** various fixes in synchronize masternode identities logic and logging ([#461](https://github.com/dashevo/platform/issues/461))
* **dpp:** data contract index update validation ([#427](https://github.com/dashevo/platform/issues/427))
* **drive:** change transaction is started check ([#451](https://github.com/dashevo/platform/issues/451))
* **wallet-lib:** separate persistent storage by walletId ([#407](https://github.com/dashevo/platform/issues/407))
* change allowed security level for withdrawal purpose to critical ([#352](https://github.com/dashevo/platform/issues/352))
* **dapi-grpc:** outdated autogenerated code ([#331](https://github.com/dashevo/platform/issues/331))
* **wallet-lib:** hook tx chain broadcast on mempool response ([#388](https://github.com/dashevo/platform/issues/388))
* **dashmate:** config/core/miner must have required property 'interval' ([#311](https://github.com/dashevo/platform/issues/311))
* do not hash proRegTx for masternode identifier ([#318](https://github.com/dashevo/platform/issues/318))
* **dpp:** cannot read properties of null (reading 'getBalance') ([#549](https://github.com/dashevo/platform/issues/549))
* **dashmate**: can't use local seed as a wallet ([#538](https://github.com/dashevo/platform/issues/538))


### Performance Improvements

* **dapi:** cache block headers and chainlocks ([#235](https://github.com/dashevo/platform/issues/235), [#296](https://github.com/dashevo/platform/issues/296))
* **dapi:** remove unnecessary Core RPC calls for core streams ([#194](https://github.com/dashevo/platform/issues/194))


### Code Refactoring

* **drive:** use RS Drive query validation logic ([#398](https://github.com/dashevo/platform/issues/398))
* simplified public key to identity structure ([#437](https://github.com/dashevo/platform/issues/437))
* **wallet-lib:** storage layer refactoring ([#232](https://github.com/dashevo/platform/issues/232))


### Tests

* **dapi:** fix broken subscribeToNewBlockHeaders test ([#508](https://github.com/dashevo/platform/issues/508))
* **dapi:** rename test files to mach the naming convention ([#509](https://github.com/dashevo/platform/issues/509))
* **dpp:** fix invalid findIndexDuplicates test in DPP ([#448](https://github.com/dashevo/platform/issues/448))
* **wallet-lib:** fixes wallet.spec.js


### Build System

* `yarn install` fails with Node.JS 16.17.0 ([#507](https://github.com/dashevo/platform/issues/507))
* new node alpine image breaks package builds ([#493](https://github.com/dashevo/platform/issues/493))
* **test-suite:** fix docker image build


### Documentation

* **dashmate:** add troubleshooting section ([#431](https://github.com/dashevo/platform/issues/431))
* **dashmate:** update dashmate documentation ([#459](https://github.com/dashevo/platform/issues/459))
* **sdk:** update outdated documentation ([#463](https://github.com/dashevo/platform/issues/463))
* update badges in individual package readmes ([#361](https://github.com/dashevo/platform/issues/361))


### Continuous Integration

* add `latest-dev` docker tag ([#382](https://github.com/dashevo/platform/issues/382))
* enable multiarch builds ([#316](https://github.com/dashevo/platform/issues/316))
* docker images incorrectly tagged with v ([#413](https://github.com/dashevo/platform/issues/413))


### Miscellaneous Chores

* **drive:** correct typo in debug message ([#535](https://github.com/dashevo/platform/issues/535))
* **test-suite:** remove unused merk dependency ([#547](https://github.com/dashevo/platform/issues/547))
* clean up dependencies ([#534](https://github.com/dashevo/platform/issues/534))
* **scripts:** remove comment from env key ([#532](https://github.com/dashevo/platform/issues/532))
* **test-suite:** move wallet storage persistence in the outer folder ([#416](https://github.com/dashevo/platform/issues/416))
* various linter fixes across packages ([#465](https://github.com/dashevo/platform/issues/465))
* **dpp:** allow only `asc` order for indices ([#435](https://github.com/dashevo/platform/issues/435))
* **drive:** log synchronize masternode identities ([#449](https://github.com/dashevo/platform/issues/449))
* **drive:** add more block execution timers ([#329](https://github.com/dashevo/platform/issues/329))
* **scripts**: remove dashmate update ([#550](https://github.com/dashevo/platform/issues/550))



## [0.23.0-dev.10](https://github.com/dashevo/platform/compare/v0.22.16...v0.23.0-dev.10) (2022-09-27)


### ⚠ BREAKING CHANGES

* **drive:** masternode identities sync from begging after restart (#542)
* update Dash Core to 18.1.0-alpha.2 (#526)

### Features

* update Dash Core to 18.1.0-alpha.2 ([#526](https://github.com/dashevo/platform/issues/526))


### Bug Fixes

* **dapi-client:** metadata.get is not a function once again ([#531](https://github.com/dashevo/platform/issues/531))
* **dashmate:** homedir fs.exists check ([#537](https://github.com/dashevo/platform/issues/537))
* **drive:** masternode identities sync from begging after restart ([#542](https://github.com/dashevo/platform/issues/542))


### Miscellaneous Chores

* clean up dependencies ([#534](https://github.com/dashevo/platform/issues/534))
* **scripts:** remove comment from env key ([#532](https://github.com/dashevo/platform/issues/532))


### [0.22.16](https://github.com/dashevo/platform/compare/v0.22.15...v0.22.16) (2022-09-08)


### Features

* create test suite's `.env` from deployment tool config ([#518](https://github.com/dashevo/platform/issues/518))


### Bug Fixes

* security vulnerability in elliptic library ([#501](https://github.com/dashevo/platform/issues/501))
* test suite image and environment problems in ci ([#505](https://github.com/dashevo/platform/issues/505))



## [0.23.0-dev.9](https://github.com/dashevo/platform/compare/v0.22.15...v0.23.0-dev.9) (2022-09-05)


### Bug Fixes

* **dashmate:** DockerComposeError undefined ([#513](https://github.com/dashevo/platform/issues/513))
* security vulnerability in elliptic library ([#501](https://github.com/dashevo/platform/issues/501))
* test suite image and environment problems in ci ([#505](https://github.com/dashevo/platform/issues/505))
* **test-suite:** failing assertions due to replication lag ([#502](https://github.com/dashevo/platform/issues/502))


### Build System

* `yarn install` fails with Node.JS 16.17.0 ([#507](https://github.com/dashevo/platform/issues/507))


### Tests

* **dapi:** fix broken subscribeToNewBlockHeaders test ([#508](https://github.com/dashevo/platform/issues/508))
* **dapi:** rename test files to mach the naming convention ([#509](https://github.com/dashevo/platform/issues/509))


### [0.22.15](https://github.com/dashevo/platform/compare/v0.22.14...v0.22.15) (2022-08-31)


### Features

* **dashmate:** update v18 for the mainnet nodes ([#514](https://github.com/dashevo/platform/issues/514))


### [0.22.14](https://github.com/dashevo/platform/compare/v0.22.13...v0.22.14) (2022-08-29)


### Features

* **dashmate:** add --force flag to stop command ([#434](https://github.com/dashevo/platform/issues/434))
* integrate dash-spv into monorepo


### Documentation

* **dashmate:** update dashmate documentation ([#459](https://github.com/dashevo/platform/issues/459))
* **sdk:** update outdated documentation ([#463](https://github.com/dashevo/platform/issues/463))


### Miscellaneous Chores

* bump dash core to v18.0.1 and sentinel to 1.7.1 ([#511](https://github.com/dashevo/platform/issues/511))
* **test-suite:** move wallet storage persistence in the outer folder ([#416](https://github.com/dashevo/platform/issues/416))


### Build System

* `yarn install` fails with Node.JS 16.17.0 ([#507](https://github.com/dashevo/platform/issues/507))
* new node alpine image breaks package builds ([#493](https://github.com/dashevo/platform/issues/493))
* **test-suite:** fix docker image build

## [0.23.0-dev.8](https://github.com/dashevo/platform/compare/v0.23.0-dev.7...v0.23.0-dev.8) (2022-08-22)


### Bug Fixes

* **dapi:** getStatus errored if masternode is banned ([#496](https://github.com/dashevo/platform/issues/496))
* Drive and DAPI expect data available on H+1 block ([#497](https://github.com/dashevo/platform/issues/497))
* **test-suite:** failing tests due to replication latency ([#500](https://github.com/dashevo/platform/issues/500))

## [0.23.0-dev.7](https://github.com/dashevo/platform/compare/v0.23.0-dev.6...v0.23.0-dev.7) (2022-08-18)


### Bug Fixes

* DAPI client requests one local single node only ([#479](https://github.com/dashevo/platform/issues/479))
* **dapi-client:** A.metadata.get is not a function ([#492](https://github.com/dashevo/platform/issues/492))
* **dapi-client:** node can bed marked as banned on retriable error ([#482](https://github.com/dashevo/platform/issues/482))
* **dashmate:** docker undefined error ([#480](https://github.com/dashevo/platform/issues/480))
* **dashmate:** gRPC stream timeout ([#481](https://github.com/dashevo/platform/issues/481))
* infinite block production due to fees distribution ([#484](https://github.com/dashevo/platform/issues/484))
* **sdk:** cannot read properties of null (reading 'getMetadata') ([#488](https://github.com/dashevo/platform/issues/488))
* **test-suite:** invalid transaction: Missing inputs ([#494](https://github.com/dashevo/platform/issues/494))


### Miscellaneous Chores

* **dashmate:** update Core to 18.0.0-rc.12


### Build System

* new node alpine image breaks package builds ([#493](https://github.com/dashevo/platform/issues/493))

## [0.23.0-dev.6](https://github.com/dashevo/platform/compare/v0.23.0-dev.5...v0.23.0-dev.6) (2022-08-12)


### ⚠ BREAKING CHANGES

* Correct cumulative fees invalidate previous blockchain data (#474)

### Bug Fixes

* **dpp:** repeated disabling of already disabled identity key ([#477](https://github.com/dashevo/platform/issues/477))
* **drive:** cumulative fees are not reset between blocks ([#474](https://github.com/dashevo/platform/issues/474))
* **drive:** invalid previous block time ([#475](https://github.com/dashevo/platform/issues/475))

## [0.23.0-dev.5](https://github.com/dashevo/platform/compare/v0.23.0-dev.4...v0.23.0-dev.5) (2022-08-05)


### ⚠ BREAKING CHANGES

* Blockchain data and state structure are changed and incompatible with previous versions

### Features

* **drive:** fees distribution ([#458](https://github.com/dashevo/platform/issues/458))


### Miscellaneous Chores

* various linter fixes across packages ([#465](https://github.com/dashevo/platform/issues/465))

## [0.23.0-dev.4](https://github.com/dashevo/platform/compare/v0.23.0-dev.3...v0.23.0-dev.4) (2022-07-12)


### Bug Fixes

* **dashmate:** replace `seeds` by `bootstrap-peers` in config.toml ([#460](https://github.com/dashevo/platform/issues/460))
* **drive:** various fixes in synchronize masternode identities logic and logging ([#461](https://github.com/dashevo/platform/issues/461))


### Build System

* **test-suite:** fix docker image build


## [0.23.0-dev.3](https://github.com/dashevo/platform/compare/v0.22.13...v0.23.0-dev.3) (2022-06-30)


### ⚠ BREAKING CHANGES

* Previous invalid data contracts in blockchain might be valid now (#445)
* `getIdentityIdsByPublicKeyHash` endpoint is removed. `getIdentitiesByPublicKeyHash` now responds with an array of identities, instead of an array of cbored arrays of identities. (#437)
* All indices must have 'asc' order (#435)
* Some state transitions in the chain could change validation result due to changes in fee logic. Previously invalid state transition in chain could become valid since BLS signing is fixed (#392)
* Previously invalidated `DataContractUpdateTransitions` with `unique` equals `false` will become valid (#427)
* Document query logic can behave differently in some cases (#398)

### Features

* **bench:** state transition benchmark ([#418](https://github.com/dashevo/platform/issues/418))
* **dashmate:** add --force flag to stop command ([#434](https://github.com/dashevo/platform/issues/434))
* **dashmate:** upgrade docker compose to v2 ([#441](https://github.com/dashevo/platform/issues/441))
* **dpp:** allow 1 char document type and 1 char property name ([#445](https://github.com/dashevo/platform/issues/445))
* integrate dash-spv into monorepo
* limit the number of shares for masternode by 16 ([#432](https://github.com/dashevo/platform/issues/432))
* move dash-spv in packages after import
* re-enable proof responses ([#440](https://github.com/dashevo/platform/issues/440))
* validate fee calculating worst case operations ([#392](https://github.com/dashevo/platform/issues/392))


### Bug Fixes

* **ci:** docker images incorrectly tagged with v ([#413](https://github.com/dashevo/platform/issues/413))
* **dpp:** data contract index update validation ([#427](https://github.com/dashevo/platform/issues/427))
* **drive:** change transaction is started check ([#451](https://github.com/dashevo/platform/issues/451))
* non-deterministic fees due to data contract cache ([#444](https://github.com/dashevo/platform/issues/444))
* **sdk:** identity update method can't sign publicKeys in some cases ([#421](https://github.com/dashevo/platform/issues/421))
* **wallet-lib:** separate persistent storage by walletId ([#407](https://github.com/dashevo/platform/issues/407))


### Documentation

* add input description


### Code Refactoring

* **drive:** use RS Drive query validation logic ([#398](https://github.com/dashevo/platform/issues/398))
* simplified public key to identity structure ([#437](https://github.com/dashevo/platform/issues/437))


### Tests

* **dpp:** fix invalid findIndexDuplicates test in DPP ([#448](https://github.com/dashevo/platform/issues/448))


### Miscellaneous Chores

* **dpp:** allow only `asc` order for indices ([#435](https://github.com/dashevo/platform/issues/435))
* **drive:** log synchronize masternode identities ([#449](https://github.com/dashevo/platform/issues/449))
* **test-suite:** move wallet storage persistence in the outer folder ([#416](https://github.com/dashevo/platform/issues/416))
* update readme


### [0.22.13](https://github.com/dashevo/platform/compare/v0.22.12...v0.22.13) (2022-06-17)


### Features

* support DIP24 devnet LLMQ type ([#438](https://github.com/dashevo/platform/issues/438))

### [0.22.12](https://github.com/dashevo/platform/compare/v0.22.11...v0.22.12) (2022-06-07)


### Bug Fixes

* **sdk:** incomplete bundle for web ([#400](https://github.com/dashevo/platform/issues/400))
* **wallet-lib:** separate persistent storage by walletId ([#407](https://github.com/dashevo/platform/issues/407))

### [0.22.11](https://github.com/dashevo/platform/compare/v0.22.10...v0.22.11) (2022-05-31)


### Bug Fixes

* incorrect image versions and variables for testnet config ([#415](https://github.com/dashevo/platform/issues/415))

### [0.22.10](https://github.com/dashevo/platform/compare/v0.22.9...v0.22.10) (2022-05-26)


### Bug Fixes

* CommitmentTxPayload#toBuffer method was using version instead of qfcVersion for serialization ([#410](https://github.com/dashevo/platform/issues/410))


### Continuous Integration

* dispatch trigger and parallelization ([#406](https://github.com/dashevo/platform/issues/406))

### [0.22.9](https://github.com/dashevo/platform/compare/v0.22.8...v0.22.9) (2022-05-24)


### Bug Fixes

* incorrect parsing of commitment payload ([#408](https://github.com/dashevo/platform/issues/408))

### [0.22.8](https://github.com/dashevo/platform/compare/v0.22.7...v0.22.8) (2022-05-23)


### Bug Fixes

* `verifyChainLock` was returning `false` instead of `ResponseQuery` ([#402](https://github.com/dashevo/platform/issues/402))
* **dashmate:** switch `drive` and `dapi` to stable versions ([#381](https://github.com/dashevo/platform/issues/381))
* **wallet-lib:** hook tx chain broadcast on mempool response ([#388](https://github.com/dashevo/platform/issues/388))


## [0.23.0-dev.2](https://github.com/dashevo/platform/compare/v0.23.0-dev.1...v0.23.0-dev.2) (2022-05-20)


### ⚠ BREAKING CHANGES

* Identity master key can be used only to update identity (#384)
* SDK's identity update method now requires correspond private keys. Identity public keys in state transitions must be signed

### Features

* bench suite ([#335](https://github.com/dashevo/platform/issues/335))
* **bench-suite:** add fees to documents benchmark ([#379](https://github.com/dashevo/platform/issues/379))
* **bench-suite:** function benchmark and other improvements ([#344](https://github.com/dashevo/platform/issues/344))
* calculate state transition fees using operations ([#376](https://github.com/dashevo/platform/issues/376))
* create withdrawal keys for masternode identities ([#320](https://github.com/dashevo/platform/issues/320))
* **dpp:** BIP13_SCRIPT_HASH identity public key type ([#353](https://github.com/dashevo/platform/issues/353))
* **dpp:** calculate signature verification costs for fees ([#387](https://github.com/dashevo/platform/issues/387))
* **dpp:** fee operations and execution context ([#369](https://github.com/dashevo/platform/issues/369))
* **drive:** collect fee operation to execution context ([#370](https://github.com/dashevo/platform/issues/370))
* Identity master key can be used only to update identity ([#384](https://github.com/dashevo/platform/issues/384))
* identity public key proofs ([#349](https://github.com/dashevo/platform/issues/349))
* integrate with Tenderdash v0.8-dev ([#314](https://github.com/dashevo/platform/issues/314))


### Bug Fixes

* change allowed security level for withdrawal purpose to critical ([#352](https://github.com/dashevo/platform/issues/352))
* **dapi-grpc:** outdated autogenerated code ([#331](https://github.com/dashevo/platform/issues/331))
* **dashmate:** switch `drive` and `dapi` to stable versions ([#381](https://github.com/dashevo/platform/issues/381))
* **wallet-lib:** hook tx chain broadcast on mempool response ([#388](https://github.com/dashevo/platform/issues/388))


### Documentation

* update badges in individual package readmes ([#361](https://github.com/dashevo/platform/issues/361))


### Continuous Integration

* add `latest-dev` docker tag ([#382](https://github.com/dashevo/platform/issues/382))


### Miscellaneous Chores

* **dashmate:** use 0.23-dev images


### [0.22.7](https://github.com/dashevo/platform/compare/v0.22.6...v0.22.7) (2022-05-02)


### Bug Fixes

* invalid version to parse `CommitmentTxPayload` ([#373](https://github.com/dashevo/platform/issues/373))

### [0.22.6](https://github.com/dashevo/platform/compare/v0.22.5...v0.22.6) (2022-05-02)


### Bug Fixes

* can't parse `CommitmentTxPayload` ([#371](https://github.com/dashevo/platform/issues/371))

### [0.22.5](https://github.com/dashevo/platform/compare/v0.22.4...v0.22.5) (2022-04-29)


### Bug Fixes

* broken QuorumEntry unserialization ([#366](https://github.com/dashevo/platform/issues/366))

### [0.22.4](https://github.com/dashevo/platform/compare/v0.22.3...v0.22.4) (2022-04-29)


### ⚠ BREAKING CHANGES

* Core v0.17 is not supported anymore

### Bug Fixes

* invalid `merkleRootQuorums` calculation ([#362](https://github.com/dashevo/platform/issues/362))

### [0.22.3](https://github.com/dashevo/platform/compare/v0.22.2...v0.22.3) (2022-04-27)


### ⚠ BREAKING CHANGES

* **wallet-lib:** storage layer refactoring (#232)

### Features

* **wallet-lib:** adds balance and metadata information from registered identity ([#337](https://github.com/dashevo/platform/issues/337))
* **wallet-lib:** provide transaction history item as a date object ([#336](https://github.com/dashevo/platform/issues/336))
* **wallet-lib:** rework storage for multiple key chains ([#231](https://github.com/dashevo/platform/issues/231))
* **wallet-lib:** satoshisBalanceImpact in transaction history ([#319](https://github.com/dashevo/platform/issues/319))
* **wallet-lib:** storage layer refactoring ([#232](https://github.com/dashevo/platform/issues/232))


### Bug Fixes

* **dashmate:** broken migrations ([#355](https://github.com/dashevo/platform/issues/355))
* **wallet-lib:** optimize storage version check ([#348](https://github.com/dashevo/platform/issues/348))
* **wallet-lib:** persistent storage regression ([#302](https://github.com/dashevo/platform/issues/302))


### [0.22.2](https://github.com/dashevo/platform/compare/v0.22.1...v0.22.2) (2022-04-21)


### Bug Fixes

* docker-test-suite missing test files


### Tests

* **dpp:** double test in identity validation ([#330](https://github.com/dashevo/platform/issues/330))
* fixes sdk timeouts in platform test suite ([#309](https://github.com/dashevo/platform/issues/309))


### Miscellaneous Chores

* update Core to v0.18.0.0-rc1 ([#351](https://github.com/dashevo/platform/issues/351))


## [0.23.0-dev.1](https://github.com/dashevo/platform/compare/v0.22.0...v0.23.0-dev.1) (2022-04-08)


### ⚠ BREAKING CHANGES

* plain proRegTx for masternode identifier (#318)
* **wallet-lib:** storage layer refactoring (#232)

### Features

* **dpp:** add `withdraw` purpose for `IdentityPublicKey` ([#317](https://github.com/dashevo/platform/issues/317))
* update identity ([#292](https://github.com/dashevo/platform/issues/292))
* **wallet-lib:** rework storage for multiple key chains ([#231](https://github.com/dashevo/platform/issues/231))
* **wallet-lib:** satoshisBalanceImpact in transaction history ([#319](https://github.com/dashevo/platform/issues/319))
* **wallet-lib:** storage layer refactoring ([#232](https://github.com/dashevo/platform/issues/232))


### Bug Fixes

* **dashmate:** config/core/miner must have required property 'interval' ([#311](https://github.com/dashevo/platform/issues/311))
* do not hash proRegTx for masternode identifier ([#318](https://github.com/dashevo/platform/issues/318))


### Performance Improvements

* **dapi:** cache block headers and chainlocks ([#235](https://github.com/dashevo/platform/issues/235), [#296](https://github.com/dashevo/platform/issues/296))
* **dapi:** remove unnecessary Core RPC calls for core streams ([#194](https://github.com/dashevo/platform/issues/194))


### Continuous Integration

* enable multiarch builds ([#316](https://github.com/dashevo/platform/issues/316))


### Miscellaneous Chores

* **drive:** add more block execution timers ([#329](https://github.com/dashevo/platform/issues/329))


### Tests

* fixes wallet.spec.js

### [0.22.1](https://github.com/dashevo/platform/compare/v0.22.0...v0.22.1) (2022-03-25)


### Bug Fixes

* **dashmate:** cannot read properties of undefined (reading 'masternodeRewardShares’) ([#310](https://github.com/dashevo/platform/issues/310))
* **dashmate:** config/core/miner must have required property 'interval' ([#311](https://github.com/dashevo/platform/issues/311))


### Tests

* fix platform-test-suite-execution in browser environment ([#289](https://github.com/dashevo/platform/issues/289))


## [0.22.0](https://github.com/dashevo/platform/compare/v0.21.8...v0.22.0) (2022-03-21)

### ⚠ BREAKING CHANGES

* `name` is required for document index definition
* `platform.contracts.broadcast` method in SDK renamed to `platform.contracts.publish`
* Identity public key requires `purpose` and `securityLevel` properties
* `$id` property can't be used in document indices
* Indexed properties now require size constraints
* `getIdentitiesByPublicKeyHashes` returns array of arrays of identities
* `getIdentityIdsByPublicKeyHashes` returns array of arrays of identity ids
* Document array properties temporarily cannot be indexed. Will be enabled in v0.23
* Range operations in document queries can be used only in the last where clause
* sorting (`orderBy`) in document queries is required for range operations
* `elementMatch`, `contains` and `includes` operations are temporarily disabled in document query. Will be enabled in v0.23
* `$ref` in data contract is temporarily disabled
* `startAt` and `startAfter` accept now only document id instead of document offset
* `in` operator can be used only in two last where clauses
* Cryptographical proofs for platform state are temporarily disabled. Will be enabled in upcoming releases
* Platform data is not compatible with previous platform versions. Please reset your node.


### Features

* identity public key purpose and security levels ([#46](https://github.com/dashevo/platform/issues/46))
* allow using non-unique Identity public keys ([#168](https://github.com/dashevo/platform/issues/168))
* distribute dashmate with NPM ([#148](https://github.com/dashevo/platform/issues/148))
* create and update masternode identities ([#160](https://github.com/dashevo/platform/issues/160), [#170](https://github.com/dashevo/platform/issues/170), [#257](https://github.com/dashevo/platform/issues/257), [#272](https://github.com/dashevo/platform/issues/272), [#279](https://github.com/dashevo/platform/issues/279), [#287](https://github.com/dashevo/platform/issues/287))
* added WalletStore ([#197](https://github.com/dashevo/platform/issues/197))
* register system contracts on `initChain` ([#182](https://github.com/dashevo/platform/issues/182), [#192](https://github.com/dashevo/platform/issues/192))
* integrate new storage (GroveDB) and secondary indices (RS Drive) ([#77](https://github.com/dashevo/platform/issues/77), [#177](https://github.com/dashevo/platform/issues/177), [#178](https://github.com/dashevo/platform/issues/178), [#199](https://github.com/dashevo/platform/issues/199), [#201](https://github.com/dashevo/platform/issues/201), [#225](https://github.com/dashevo/platform/issues/225), [#259](https://github.com/dashevo/platform/issues/259), [#280](https://github.com/dashevo/platform/issues/280), [#303](https://github.com/dashevo/platform/issues/303))
* fallback to chain asset lock proof ([#297](https://github.com/dashevo/platform/issues/297))
* add an ability to update data contract ([#52](https://github.com/dashevo/platform/issues/52), [#83](https://github.com/dashevo/platform/issues/83), [#223](https://github.com/dashevo/platform/issues/223))
* add required `name` property to index definition ([#74](https://github.com/dashevo/platform/issues/74))
* use document for `startAt` and `startAfter` in document query ([#227](https://github.com/dashevo/platform/pull/227), [#255](https://github.com/dashevo/platform/issues/255))
* **dashmate:** enable mainnet for dashmate ([#2](https://github.com/dashevo/platform/issues/2))
* **dashmate:** json output for status commands ([#31](https://github.com/dashevo/platform/issues/31), [#262](https://github.com/dashevo/platform/issues/262))
* **dashmate:** add an ability to configure node subnet mask ([#237](https://github.com/dashevo/platform/issues/237))
* **dpp:** add `readOnly` flag to `IdentityPublicKey` ([#142](https://github.com/dashevo/platform/issues/142), [#239](https://github.com/dashevo/platform/issues/239))
* **dpp:** allow using BLS key to sign state transitions ([#268](https://github.com/dashevo/platform/issues/268), [#275](https://github.com/dashevo/platform/issues/275))
* **drive:** network address in `ValidatorUpdate` ABCI ([#140](https://github.com/dashevo/platform/issues/140), [#155](https://github.com/dashevo/platform/issues/155), [#184](https://github.com/dashevo/platform/issues/184))
* **drive:** add performance timers to measure block execution ([#281](https://github.com/dashevo/platform/issues/281))
* **dapi:** `subscribeToBlockHeadersWithChainLocks` endpoint ([#153](https://github.com/dashevo/platform/issues/153))
* **wallet-lib:** ChainStore ([#196](https://github.com/dashevo/platform/issues/196))
* **dapi-client:** get and verify block headers with dash-spv ([#211](https://github.com/dashevo/platform/issues/211))
* **dapi-client:** handle asynchronous errors ([#233](https://github.com/dashevo/platform/issues/233))


### Bug Fixes

* **dashmate:** `cannot read properties of undefined (reading 'dpns')` on reset ([#47](https://github.com/dashevo/platform/issues/47))
* **drive:** missed JS ABCI yarn cache ([#156](https://github.com/dashevo/platform/issues/156))
* **build:** `zeromq` build is not working on linux ([#236](https://github.com/dashevo/platform/issues/236))
* cannot install `protobufjs` in some cases ([#266](https://github.com/dashevo/platform/issues/266), [#267](https://github.com/dashevo/platform/issues/267))
* **dashmate:** `rimraf` module could not remove config directory ([#248](https://github.com/dashevo/platform/issues/248))
* **dashmate:** logs were incorrectly mounted ([#261](https://github.com/dashevo/platform/issues/261))
* **drive:** documents have mixed owner ids ([#283](https://github.com/dashevo/platform/issues/283))
* cannot read properties of undefined (reading 'getIp') ([#285](https://github.com/dashevo/platform/issues/285))
* InstantLock waiting period for transaction... ([#293](https://github.com/dashevo/platform/issues/293))
* **dpp:** re2 memory leak ([#301](https://github.com/dashevo/platform/issues/301))
* **drive:** internal error on verify instant lock ([#295](https://github.com/dashevo/platform/issues/295))


### Documentation

* improved sidebar and usage in DAPI client ([#3](https://github.com/dashevo/platform/issues/3))
* provide getTransactionHistory ([#5](https://github.com/dashevo/platform/issues/5))
* minor Readme fixes ([#163](https://github.com/dashevo/platform/issues/163))
* add readme to docs folder ([#175](https://github.com/dashevo/platform/issues/175))
* escape literal '|' in table ([#164](https://github.com/dashevo/platform/issues/164))
* indicate which network(s) this repo supports ([#174](https://github.com/dashevo/platform/issues/174))
* ignore folder with empty docs during build ([#212](https://github.com/dashevo/platform/issues/212))


### Tests

* **wallet-lib:** enable skipped test after the fix for grpc-js lib ([#71](https://github.com/dashevo/platform/issues/71))


### Miscellaneous Chores

* fix wrong version in a release PR title ([#82](https://github.com/dashevo/platform/issues/82))
* missed merk darwin x64 pre-build binary ([#144](https://github.com/dashevo/platform/issues/144))
* undefined "-w" argument in restart script ([#85](https://github.com/dashevo/platform/issues/85))
* **drive:** send initial core chain locked height on init chain ([#180](https://github.com/dashevo/platform/issues/180))
* update to use current @oclif/core ([#154](https://github.com/dashevo/platform/issues/154))
* remove `fixCumulativeFeesBug` feature flag ([#191](https://github.com/dashevo/platform/issues/191))
* update tenderdash and core images ([#188](https://github.com/dashevo/platform/issues/188), [#252](https://github.com/dashevo/platform/issues/252), [#269](https://github.com/dashevo/platform/issues/269))
* **dpp:** temporarily disable $refs in data contract definitions ([#300](https://github.com/dashevo/platform/issues/300))
* **dpp:** size constraints for indexed properties ([#179](https://github.com/dashevo/platform/issues/179), [#273](https://github.com/dashevo/platform/issues/273))


### Build System

* **test-suite:** docker image build doesn't work ([#172](https://github.com/dashevo/platform/issues/172))
* fix configure test suite script for grep 2.5.1 ([#187](https://github.com/dashevo/platform/issues/187))


### Code Refactoring

* **dapi:** rename tx-filter-stream.js to core-streams.js ([#169](https://github.com/dashevo/platform/issues/169))


## [0.22.0-dev.16](https://github.com/dashevo/platform/compare/v0.22.0-dev.15...v0.22.0-dev.16) (2022-03-18)


### ⚠ BREAKING CHANGES

* previously created platform state might be not compatible

### Features

* **dpp:** temporarily disable $refs in data contract definitions ([#300](https://github.com/dashevo/platform/issues/300))
* fallback to chain asset lock proof ([#297](https://github.com/dashevo/platform/issues/297))


### Bug Fixes

* **dpp:** re2 memory leak ([#301](https://github.com/dashevo/platform/issues/301))
* **drive:** document query and delete issues ([#303](https://github.com/dashevo/platform/issues/303))
* **drive:** internal error on verify instant lock ([#295](https://github.com/dashevo/platform/issues/295))

## [0.22.0-dev.15](https://github.com/dashevo/platform/compare/v0.22.0-dev.14...v0.22.0-dev.15) (2022-03-11)


### Bug Fixes

* InstantLock waiting period for transaction.. ([#293](https://github.com/dashevo/platform/issues/293))

## [0.22.0-dev.14](https://github.com/dashevo/platform/compare/v0.22.0-dev.13...v0.22.0-dev.14) (2022-03-10)


### ⚠ BREAKING CHANGES

* The fixed masternode identities logic breaks compatibility with previous invalid state.

### Bug Fixes

* **drive:** non-deterministic behaviour in masternode identities logic  ([#287](https://github.com/dashevo/platform/issues/287))

## [0.22.0-dev.13](https://github.com/dashevo/platform/compare/v0.22.0-dev.12...v0.22.0-dev.13) (2022-03-09)


### Bug Fixes

* cannot read properties of undefined (reading 'getIp') ([#285](https://github.com/dashevo/platform/issues/285))

## [0.22.0-dev.12](https://github.com/dashevo/platform/compare/v0.22.0-dev.11...v0.22.0-dev.12) (2022-03-08)


### Bug Fixes

* **drive:** documents have mixed owner ids ([#283](https://github.com/dashevo/platform/issues/283))

## [0.22.0-dev.11](https://github.com/dashevo/platform/compare/v0.22.0-dev.10...v0.22.0-dev.11) (2022-03-08)


### ⚠ BREAKING CHANGES

* `in` query operator doesn't work with multiple values (#280)

### Features

* **drive:** add performance timers to measure block execution ([#281](https://github.com/dashevo/platform/issues/281))


### Bug Fixes

* `in` query operator doesn't work with multiple values ([#280](https://github.com/dashevo/platform/issues/280))
* can't find masternode raward shares data contract ([#279](https://github.com/dashevo/platform/issues/279))

## [0.22.0-dev.10](https://github.com/dashevo/platform/compare/v0.22.0-dev.9...v0.22.0-dev.10) (2022-03-07)


### Bug Fixes

* **dpp:** Invalid DER format public key ([#275](https://github.com/dashevo/platform/issues/275))

## [0.22.0-dev.9](https://github.com/dashevo/platform/compare/v0.22.0-dev.8...v0.22.0-dev.9) (2022-03-04)


### ⚠ BREAKING CHANGES

* **dpp:** lower indexed string properties constraints (#273)

### Features

* **dpp:** lower indexed string properties constraints ([#273](https://github.com/dashevo/platform/issues/273))


### Bug Fixes

* masternode reward shares ([#272](https://github.com/dashevo/platform/issues/272))

## [0.22.0-dev.8](https://github.com/dashevo/platform/compare/v0.21.8...v0.22.0-dev.8) (2022-03-01)


### ⚠ BREAKING CHANGES

* New state is not compatible with previous versions
* Document queries have limitations compared with previous versions
* Proofs are temporary disabled

### Features

* **dapi-client:** get and verify block headers with dash-spv ([#211](https://github.com/dashevo/platform/issues/211))
* **dapi-client:** handle asynchronous errors ([#233](https://github.com/dashevo/platform/issues/233))
* **dashmate:** add an ability to configure node subnet mask ([#237](https://github.com/dashevo/platform/issues/237))
* **dpp:** allow using BLS key to sign state transitions ([#268](https://github.com/dashevo/platform/issues/268))
* **dpp:** do not allow to index array properties ([#225](https://github.com/dashevo/platform/issues/225))
* **drive:** create/update identities based on SML changes ([#170](https://github.com/dashevo/platform/issues/170))
* integrate RS Drive and GroveDB ([#177](https://github.com/dashevo/platform/issues/177))


### Bug Fixes

* **dashmate:** `group:status` command was missing a `format` flag ([#262](https://github.com/dashevo/platform/issues/262))
* `startAt` and `startAfter` invalid decoding ([#255](https://github.com/dashevo/platform/issues/255))
* **build:** `zeromq` build is not working on linux ([#236](https://github.com/dashevo/platform/issues/236))
* cannot install `protobufjs` in some cases ([#266](https://github.com/dashevo/platform/issues/266))
* **dashmate:** `rimraf` module could not remove config directory ([#248](https://github.com/dashevo/platform/issues/248))
* **dashmate:** logs were incorrectly mounted ([#261](https://github.com/dashevo/platform/issues/261))
* **dpp:** Identity public key `readOnly` flag was read as `undefined` instead of `false` ([#239](https://github.com/dashevo/platform/issues/239))
* **drive:** unable to reconstruct SML ([#257](https://github.com/dashevo/platform/issues/257))
* **drive:** invalid query errors are fatal ([#259](https://github.com/dashevo/platform/issues/259))
* **sdk:** can't update cached data contract ([#223](https://github.com/dashevo/platform/issues/223))


### Documentation

* ignore folder with empty docs during build ([#212](https://github.com/dashevo/platform/issues/212))


### Build System

* `protobufjs` isn't installing from git sometimes ([#267](https://github.com/dashevo/platform/issues/267))


### Miscellaneous Chores

* **dashmate:** update Core to 0.18.0.0-beta4 ([#269](https://github.com/dashevo/platform/issues/269))
* **release:** revert version back
* update tenderdash and core images ([#252](https://github.com/dashevo/platform/issues/252))



## [0.21.8](https://github.com/dashevo/platform/compare/v0.21.7...v0.21.8) (2022-02-15)


### Bug Fixes

* sorting unconfirmed tx as oldest ([#206](https://github.com/dashevo/platform/issues/206))
* **wallet-lib:** get transaction history missing txs ([#246](https://github.com/dashevo/platform/issues/246))


### Tests

* **platform-suite:** add -b flag to abort after first error ([#222](https://github.com/dashevo/platform/issues/222))


### Miscellaneous Chores

* updates @dashevo/dashcore-lib to v0.19.30 ([#238](https://github.com/dashevo/platform/issues/238))


## [0.22.0-dev.7](https://github.com/dashevo/platform/compare/v0.21.7...v0.22.0-dev.7) (2022-01-19)


### Features

* added WalletStore ([#197](https://github.com/dashevo/platform/issues/197))
* **drive:** allow using `in` and `startsWith` only in last `where` condition ([#201](https://github.com/dashevo/platform/issues/201))
* **drive:** allow using `orderBy` for fields having `in` and `startsWith` in last `where` clause ([#199](https://github.com/dashevo/platform/issues/199))
* register system contracts on `initChain` ([#182](https://github.com/dashevo/platform/issues/182))
* **wallet-lib:** ChainStore ([#196](https://github.com/dashevo/platform/issues/196))


### Bug Fixes

* **sdk:** system contract ids were hardcoded in SDKs Client module ([#192](https://github.com/dashevo/platform/issues/192))


### Build System

* fix configure test suite script for grep 2.5.1 ([#187](https://github.com/dashevo/platform/issues/187))


### Miscellaneous Chores

* **dashmate:** update tenderdash to 0.7.0-dev ([#188](https://github.com/dashevo/platform/issues/188))
* remove `fixCumulativeFeesBug` feature flag ([#191](https://github.com/dashevo/platform/issues/191))



## [0.21.7](https://github.com/dashevo/platform/compare/v0.21.6...v0.21.7) (2022-01-17)


### ⚠ BREAKING CHANGES

* **dashmate:** `platform.drive.abci.docker.build.path' and 'platform.dapi.api.docker.build.path' are removed in favor of `platform.sourcePath'

### Features

* **dashmate:** build DAPI and Drive from monorepo path ([#145](https://github.com/dashevo/platform/issues/145))
* distribute dashmate with NPM ([#148](https://github.com/dashevo/platform/issues/148))
* support Apple Silicone ([#143](https://github.com/dashevo/platform/issues/143))


### Bug Fixes

* instantlock waiting period for transaction <hash> timed out


### Miscellaneous Chores

* fix wrong version in a release PR title ([#82](https://github.com/dashevo/platform/issues/82))
* missed merk darwin x64 pre-build binary ([#144](https://github.com/dashevo/platform/issues/144))
* undefined "-w" argument in restart script ([#85](https://github.com/dashevo/platform/issues/85))


### Documentation

* escape literal '|' in table ([#164](https://github.com/dashevo/platform/issues/164))


### Tests

* **wallet-lib:** fix hanging functional test ([#186](https://github.com/dashevo/platform/issues/186))

## [0.22.0-dev.6](https://github.com/dashevo/platform/compare/v0.22.0-dev.5...v0.22.0-dev.6) (2022-01-11)


### ⚠ BREAKING CHANGES

* **drive:** temporary restrictions for a document query (#77)

### Features

* **dapi:** `subscribeToBlockHeadersWithChainLocks` endpoint ([#153](https://github.com/dashevo/platform/issues/153))


### Bug Fixes

* **drive:** missed `nodeAddress` field on `EndBlock` ([#184](https://github.com/dashevo/platform/issues/184))


### Miscellaneous Chores

* **drive:** temporary restrictions for a document query ([#77](https://github.com/dashevo/platform/issues/77))


### Build System

* **test-suite:** docker image build doesn't work ([#172](https://github.com/dashevo/platform/issues/172))


### Code Refactoring

* **dapi:** rename tx-filter-stream.js to core-streams.js ([#169](https://github.com/dashevo/platform/issues/169))


### Documentation

* add readme to docs folder ([#175](https://github.com/dashevo/platform/issues/175))
* escape literal '|' in table ([#164](https://github.com/dashevo/platform/issues/164))
* indicate which network(s) this repo supports ([#174](https://github.com/dashevo/platform/issues/174))

## [0.22.0-dev.5](https://github.com/dashevo/platform/compare/v0.22.0-dev.4...v0.22.0-dev.5) (2022-01-07)


### ⚠ BREAKING CHANGES

* **dpp:** `$id` can't be used in secondary indices
* **dpp:** Indexed properties now require size constraints
* allow using non-unique Identity public keys (#168)
* **dashmate:** `platform.drive.abci.docker.build.path' and 'platform.dapi.api.docker.build.path' are removed in favor of `platform.sourcePath'

### Features

* allow adding non-unique indices for newly defined properties ([#83](https://github.com/dashevo/platform/issues/83))
* allow using non-unique Identity public keys ([#168](https://github.com/dashevo/platform/issues/168))
* **dashmate:** build DAPI and Drive from monorepo path ([#145](https://github.com/dashevo/platform/issues/145))
* distribute dashmate with NPM ([#148](https://github.com/dashevo/platform/issues/148))
* **dpp:** `$id` can't be used in secondary indices ([#178](https://github.com/dashevo/platform/issues/178))
* **dpp:** size constraints for indexed properties ([#179](https://github.com/dashevo/platform/issues/179))
* masternode reward shares contract ([#160](https://github.com/dashevo/platform/issues/160))


### Bug Fixes

* downgrade dash-core image to v0.17 ([#171](https://github.com/dashevo/platform/issues/171))


### Documentation

* minor Readme fixes ([#163](https://github.com/dashevo/platform/issues/163))


### Miscellaneous Chores

* **drive:** send initial core chain locked height on init chain ([#180](https://github.com/dashevo/platform/issues/180))
* update to use current @oclif/core ([#154](https://github.com/dashevo/platform/issues/154))

## [0.22.0-dev.4](https://github.com/dashevo/platform/compare/v0.22.0-dev.3...v0.22.0-dev.4) (2021-12-24)


### Bug Fixes

* **drive:** `ValidatorSetUpdate` doesn't contain `nodeAddress` ([#155](https://github.com/dashevo/platform/issues/155))
* **drive:** missed JS ABCI yarn cache ([#156](https://github.com/dashevo/platform/issues/156))

## [0.22.0-dev.3](https://github.com/dashevo/platform/compare/v0.21.6...v0.22.0-dev.3) (2021-12-21)


### ⚠ BREAKING CHANGES

* add required `name` property to index definition (#74)
* add an ability to update data contract (#52)
* Identity public key now has two more fields, purpose and securityLevel, and keys without those fields won't be valid anymore

### Features

* add an ability to update data contract ([#52](https://github.com/dashevo/platform/issues/52))
* add required `name` property to index definition ([#74](https://github.com/dashevo/platform/issues/74))
* **dashmate:** json output for status commands ([#31](https://github.com/dashevo/platform/issues/31))
* **dpp:** add `readOnly` flag to `IdentityPublicKey` ([#142](https://github.com/dashevo/platform/issues/142))
* **drive:** network address in `ValidatorUpdate` ABCI ([#140](https://github.com/dashevo/platform/issues/140))
* enable mainnet for dashmate ([#2](https://github.com/dashevo/platform/issues/2))
* identity public key purpose and security levels ([#46](https://github.com/dashevo/platform/issues/46))
* support Apple Silicone ([#143](https://github.com/dashevo/platform/issues/143))
* **wallet-lib:** do not sync transactions if mnemonic is absent
* **wallet-lib:** dump wallet storage ([#8](https://github.com/dashevo/platform/issues/8))


### Bug Fixes

* **dashmate:** `cannot read properties of undefined (reading 'dpns')` on reset ([#47](https://github.com/dashevo/platform/issues/47))


### Documentation

* improved sidebar and usage in DAPI client ([#3](https://github.com/dashevo/platform/issues/3))
* provide getTransactionHistory ([#5](https://github.com/dashevo/platform/issues/5))


### Tests

* **wallet-lib:** enable skipped test after the fix for grpc-js lib ([#71](https://github.com/dashevo/platform/issues/71))


### Miscellaneous Chores

* fix wrong version in a release PR title ([#82](https://github.com/dashevo/platform/issues/82))
* missed merk darwin x64 pre-build binary ([#144](https://github.com/dashevo/platform/issues/144))
* undefined "-w" argument in restart script ([#85](https://github.com/dashevo/platform/issues/85))


## [0.21.6](https://github.com/dashevo/platform/compare/v0.21.5...v0.21.6) (2021-12-13)


### Bug Fixes

* **dashmate:** RPC error on stopping node ([#61](https://github.com/dashevo/platform/issues/61))
* **wallet-lib:** "Failure: Type not convertible to Uint8Array" ([#60](https://github.com/dashevo/platform/issues/60))
* **wallet-lib:** eventemitter memory leak ([#56](https://github.com/dashevo/platform/issues/56))
* **wallet-lib:** invalid deserialization of persistent storage ([#76](https://github.com/dashevo/platform/issues/76))


### Documentation

* publish consolidated docs using mkdocs ([#42](https://github.com/dashevo/platform/issues/42))


### Miscellaneous Chores

* changelogs generation script ([#62](https://github.com/dashevo/platform/issues/62))
* enable yarn PnP to achieve zero installs ([#63](https://github.com/dashevo/platform/issues/63))
* exit if some env variables are empty during setup ([#75](https://github.com/dashevo/platform/issues/75))
* fix `test:drive` script ([#78](https://github.com/dashevo/platform/issues/78))
* migrate from NPM to Yarn 3 ([#50](https://github.com/dashevo/platform/issues/50))
* remove temporary reset script ([#64](https://github.com/dashevo/platform/issues/64))
* update oclif and remove pnpify ([#73](https://github.com/dashevo/platform/issues/73))


### Build System

* fix bash syntax issue in release script ([#79](https://github.com/dashevo/platform/issues/79))
* release process automation ([#67](https://github.com/dashevo/platform/issues/67))

## [0.21.5](https://github.com/dashevo/platform/compare/v0.21.4...v0.21.5) (2021-11-25)


### Bug Fixes

* new instant lock is not compatible with DashCore 0.17 ([#57](https://github.com/dashevo/platform/issues/57))
* **wallet-lib:** tx chaining mempool conflict errors ([#57](https://github.com/dashevo/platform/issues/44))


### Continuous Integration
* use correct Dockerfile in test suite release ([#58](https://github.com/dashevo/platform/issues/58))
* set correct docker tag outputs in release workflow ([#55](https://github.com/dashevo/platform/issues/55))
* enable NPM login on for release workflow ([#54](https://github.com/dashevo/platform/issues/54))


## [0.21.4](https://github.com/dashevo/platform/compare/v0.21.0...v0.21.4) (2021-11-23)


### Bug Fixes

* **dapi-client:** expect 100 but got 122 in SML provider test ([#22](https://github.com/dashevo/platform/issues/22))
* **dapi-client:** retry doesn’t work with 502 errors ([#35](https://github.com/dashevo/platform/issues/35))
* **dapi:** Identifier expects Buffer ([#28](https://github.com/dashevo/platform/issues/28))
* **dashmate:** ajv schema errors ([#14](https://github.com/dashevo/platform/issues/14))
* **dashmate:** reset command doesn't work if setup failed ([#23](https://github.com/dashevo/platform/issues/23))
* **dashmate:** cannot read properties error on group:reset ([#47](https://github.com/dashevo/platform/issues/47))
* **dashmate:** json output for status commands ([#31](https://github.com/dashevo/platform/issues/31))
* **dashmate:** enable mainnet for dashmate ([#2](https://github.com/dashevo/platform/issues/2))
* **dpp:** rename generateEntropy to entropyGenerator ([#13](https://github.com/dashevo/platform/issues/13))
* **sdk:** dpp hash function import ([#15](https://github.com/dashevo/platform/issues/15))
* **sdk:** override ts-node target for unit tests ([#21](https://github.com/dashevo/platform/issues/21))
* **sdk:** this is undefined during unit tests ([#18](https://github.com/dashevo/platform/issues/18))


### Features

* **dashmate:** force option for `group:stop` command ([#36](https://github.com/dashevo/platform/issues/36))
* **dashmate:** provide docker build logs for verbose mode ([#19](https://github.com/dashevo/platform/issues/19))
* migrate to DashCore 0.18.0.0-beta1 ([#51](https://github.com/dashevo/platform/issues/51))
* **wallet-lib:** dump wallet storage ([#8](https://github.com/dashevo/platform/issues/8))
* **wallet-lib:** do not sync transactions if mnemonic is absent ([#7](https://github.com/dashevo/platform/issues/7))


### Performance Improvements

* **test-suite:** speedup test suite up to 6 times ([#30](https://github.com/dashevo/platform/issues/30))


### Build System
* build only necessary packages ([#27](https://github.com/dashevo/platform/issues/27))
* run npm scripts in parallel ([#33](https://github.com/dashevo/platform/issues/33))
* cache native npm modules during docker build ([#20](https://github.com/dashevo/platform/issues/20))
* setup semantic pull requests ([#11](https://github.com/dashevo/platform/issues/11))
* **sdk:** upgrade to webpack 5 ([#6](https://github.com/dashevo/platform/issues/6))


### Continuous Integration
* simplify release workflow ([#48](https://github.com/dashevo/platform/issues/48))
* show docker logs on failure ([#43](https://github.com/dashevo/platform/issues/43))
* check mismatch dependencies ([#26](https://github.com/dashevo/platform/issues/26))
* run package tests in parallel ([#25](https://github.com/dashevo/platform/issues/25))


### Tests
* adjust timeouts ([#45](https://github.com/dashevo/platform/issues/45))
* **test-suite:** skipSynchronizationBeforeHeight option with new wallet ([#34](https://github.com/dashevo/platform/issues/34))
* **dpp:** fix invalid network floating error ([#32](https://github.com/dashevo/platform/issues/32))
* **dpp:** grpc common bootstrap not working ([#16](https://github.com/dashevo/platform/issues/16))


### Documentation
* markdown link fixes ([#49](https://github.com/dashevo/platform/issues/49))
* add README.md for the whole platform as a project ([#38](https://github.com/dashevo/platform/issues/38))
* add contributing.md ([#37](https://github.com/dashevo/platform/issues/37))
* **sdk:** provide getTransactionHistory ([#5](https://github.com/dashevo/platform/issues/5))
* improved sidebar and usage in DAPI client ([#3](https://github.com/dashevo/platform/issues/3))


### Styles
* fix ES linter errors ([#24](https://github.com/dashevo/platform/issues/24))


### BREAKING CHANGES

* supports only new DashCore InstantLock format https://github.com/dashpay/dips/blob/master/dip-0022.md


# Previous versions

Before 0.21.x, packages were located in separate repositories and have own changelogs:

* [DAPI Client](https://github.com/dashevo/js-dapi-client/blob/master/CHANGELOG.md)
* [DAPI gRPC](https://github.com/dashevo/dapi-grpc/blob/master/CHANGELOG.md)
* [DAPI](https://github.com/dashevo/dapi/blob/master/CHANGELOG.md)
* [Dashmate](https://github.com/dashevo/dashmate/blob/master/CHANGELOG.md)
* [DashPay contract](https://github.com/dashevo/dashpay-contract/blob/master/CHANGELOG.md)
* [Feature Flags Contract](https://github.com/dashevo/feature-flags-contract/blob/master/CHANGELOG.md)
* [Dash SDK](https://github.com/dashevo/js-dash-sdk/blob/master/CHANGELOG.md)
* [Dash Platform Protocol JS](https://github.com/dashevo/js-dpp/blob/master/CHANGELOG.md)
* [Drive](https://github.com/dashevo/js-drive/blob/master/CHANGELOG.md)
* [Dash Platform Test Suite](https://github.com/dashevo/platform-test-suite/blob/master/CHANGELOG.md)
* [Wallet Library](https://github.com/dashevo/wallet-lib/blob/master/CHANGELOG.md)
