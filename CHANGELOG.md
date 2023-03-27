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
