## [4.2.0-dev.1](https://github.com/dashpay/platform/compare/v4.1.0...v4.2.0-dev.1) (2026-08-12)


### ⚠ BREAKING CHANGES

* **drive-abci:** cross-check the contested index of a prefunded voting balance at protocol v14 (#4281)
* **kotlin-sdk:** keystore rework — policy-alias split, layered key recovery, durable repair, structured signer errors (stacked on #4191) (#4183)
* **drive:** make shared-prefix aggregate indexes insertable at protocol v14 (#4265)
* **dashmate:** record whether a platform image was chosen instead of deriving it (#4239)

### Features

* **dashmate:** record whether a platform image was chosen instead of deriving it ([#4239](https://github.com/dashpay/platform/issues/4239))
* expose Platform-to-Shielded capacity preflight ([#4360](https://github.com/dashpay/platform/issues/4360))
* **kotlin-sdk:** bind OP_RETURN, output-order and VIN0-change builder controls ([#4288](https://github.com/dashpay/platform/issues/4288))
* **kotlin-sdk:** dashpay invitations — create, claim, reclaim, persistence (DIP-13) ([#4284](https://github.com/dashpay/platform/issues/4284))
* **kotlin-sdk:** expose core_wallet_next_receive_address / next_change_address (Swift parity) ([#4260](https://github.com/dashpay/platform/issues/4260))
* **kotlin-sdk:** keystore rework — policy-alias split, layered key recovery, durable repair, structured signer errors (stacked on [#4191](https://github.com/dashpay/platform/issues/4191)) ([#4183](https://github.com/dashpay/platform/issues/4183))
* **kotlin-sdk:** split build/broadcast with reservation release for BIP70-style deferred submission ([#4308](https://github.com/dashpay/platform/issues/4308))
* **kotlin-sdk:** tx-label & asset-lock-kind DAO resolver queries ([#4251](https://github.com/dashpay/platform/issues/4251))
* **platform-wallet:** add encrypted txMetadata document support ([#4277](https://github.com/dashpay/platform/issues/4277))
* **platform-wallet:** classic Dash message signing (signMessage) over FFI, JNI, Kotlin, and Swift, closes [#4259](https://github.com/dashpay/platform/issues/4259) [#4279](https://github.com/dashpay/platform/issues/4279)
* **platform-wallet:** CoinJoin-drain asset-lock funding for the shielded pool ([#4327](https://github.com/dashpay/platform/issues/4327))
* **platform-wallet:** derive owner/voting provider keys Rust-side ([#4338](https://github.com/dashpay/platform/issues/4338))
* **platform-wallet:** expose an invitation's prospective identity id ([#4332](https://github.com/dashpay/platform/issues/4332))
* **platform-wallet:** own the DashPay startup ordering instead of each client ([#4359](https://github.com/dashpay/platform/issues/4359))
* **platform-wallet:** persist DashPay payment history through the persister callback ([#4326](https://github.com/dashpay/platform/issues/4326))
* **platform-wallet:** pool BIP44 + BIP32 + DashPay receiving funds on the asset-lock path ([#4350](https://github.com/dashpay/platform/issues/4350))
* **platform-wallet:** pool BIP44 + BIP32 + DashPay receiving funds on the send path ([#4329](https://github.com/dashpay/platform/issues/4329))
* **platform-wallet:** rebuild tracked asset locks after restore; honest scan-derived shielded history ([#4342](https://github.com/dashpay/platform/issues/4342))
* **platform-wallet:** reconstruct sent DashPay payments from tx history ([#4300](https://github.com/dashpay/platform/issues/4300))
* **platform-wallet:** registry-owned coordinator lifecycle with Rust-owned FFI callback contexts ([#4268](https://github.com/dashpay/platform/issues/4268))
* **platform-wallet:** wallet-level DPNS username marketplace with FFI and Swift wrappers ([#4348](https://github.com/dashpay/platform/issues/4348))
* **platform:** introduce protocol version 14 ([#4267](https://github.com/dashpay/platform/issues/4267))
* ranked aggregate indexes with provable top-K queries (protocol v14) ([#4266](https://github.com/dashpay/platform/issues/4266))
* **rs-sdk:** own the masternode voting-key facts ([#4340](https://github.com/dashpay/platform/issues/4340))
* **sdk:** add transport feature to dapi-grpc for types-only consumers ([#4344](https://github.com/dashpay/platform/issues/4344))
* **sdk:** expose OP_RETURN, output-order and change-to-VIN0 controls ([#4286](https://github.com/dashpay/platform/issues/4286))
* **sdk:** expose the label each contender actually requested ([#4331](https://github.com/dashpay/platform/issues/4331))
* **sdk:** masternodes-by-voting-key lookup (contested-username voting post-cutover) ([#4258](https://github.com/dashpay/platform/issues/4258))
* **swift-sdk:** expose watermark-freeze sync fault flag (syncFaultDetected) ([#4320](https://github.com/dashpay/platform/issues/4320))
* **swift-sdk:** split build/broadcast with reservation release for BIP70-style deferred submission ([#4322](https://github.com/dashpay/platform/issues/4322))
* **swift-sdk:** typed DPNS contested-name browsing for voters ([#4328](https://github.com/dashpay/platform/issues/4328))


### Bug Fixes

* **dashmate:** extend Core shutdown grace period ([#4307](https://github.com/dashpay/platform/issues/4307))
* **docs:** point parity manifest at the renamed identity-resume test, closes [#4015](https://github.com/dashpay/platform/issues/4015)
* **dpp:** derive deterministic names for unnamed document type indexes ([#4280](https://github.com/dashpay/platform/issues/4280))
* **dpp:** make document type index update validation name-order independent ([#4291](https://github.com/dashpay/platform/issues/4291))
* **dpp:** stop hard-erroring on index-order-only contract updates ([#4295](https://github.com/dashpay/platform/issues/4295))
* **drive-abci:** cross-check the contested index of a prefunded voting balance at protocol v14 ([#4281](https://github.com/dashpay/platform/issues/4281))
* **drive:** derive wasm-drive-verify platform version cap from rs-platform-version ([#4269](https://github.com/dashpay/platform/issues/4269))
* **drive:** make shared-prefix aggregate indexes insertable at protocol v14 ([#4265](https://github.com/dashpay/platform/issues/4265))
* **kotlin-sdk:** unmanaged-identity reads return absence + typed SigningKeyUnavailable (split from [#4183](https://github.com/dashpay/platform/issues/4183)) ([#4191](https://github.com/dashpay/platform/issues/4191))
* **platform-wallet:** accept legacy dashj key purposes on inbound contact requests ([#4372](https://github.com/dashpay/platform/issues/4372))
* **platform-wallet:** batch wallet-event persistence + expose sync_fault (watermark-freeze mitigations) ([#4314](https://github.com/dashpay/platform/issues/4314))
* **platform-wallet:** finalize reconstructed asset locks as RecoveredFromChain, in-session ([#4347](https://github.com/dashpay/platform/issues/4347))
* **platform-wallet:** fold fetched on-chain state into already-known identities on load ([#4374](https://github.com/dashpay/platform/issues/4374))
* **platform-wallet:** gate candidate-address re-export behind shielded feature ([#4369](https://github.com/dashpay/platform/issues/4369))
* **platform-wallet:** lossless mpsc persistence drain — root-cause fix for the sync-watermark freeze ([#4315](https://github.com/dashpay/platform/issues/4315))
* **platform-wallet:** preserve reported-consumed asset-lock recovery ([#4357](https://github.com/dashpay/platform/issues/4357))
* **platform-wallet:** raise the invitation cap to cover the contested username tier ([#4362](https://github.com/dashpay/platform/issues/4362))
* **platform-wallet:** reject trailing bytes in persisted asset-lock proof blobs ([#4346](https://github.com/dashpay/platform/issues/4346))
* **platform-wallet:** report an unanswered identity scan as incomplete, not empty ([#4352](https://github.com/dashpay/platform/issues/4352))
* **platform-wallet:** stale-islock to chainlock fallback for the L1 invite claim ([#4364](https://github.com/dashpay/platform/issues/4364))
* **platform-wallet:** stop a contact's watch-only chain from defining the persisted transaction row ([#4363](https://github.com/dashpay/platform/issues/4363))
* **platform-wallet:** survive an ambiguous re-broadcast when resuming a Built asset lock ([#4367](https://github.com/dashpay/platform/issues/4367))
* **platform-wallet:** type signer-reported missing key as MessageSigningKeyUnavailable ([#4321](https://github.com/dashpay/platform/issues/4321))
* **release:** base changelog on the immediately-preceding release ([#4229](https://github.com/dashpay/platform/issues/4229))
* **rs-sdk-ffi:** build the voter identifier from ProTxHash byte order ([#4333](https://github.com/dashpay/platform/issues/4333))
* **sdk:** poll message signing on the big-stack worker, and throw on JNI string-allocation failure
* **sdk:** restore proved current-epoch fetch with two-step explicit-start query ([#4231](https://github.com/dashpay/platform/issues/4231))
* **swift-example-app:** gate identity resumes by funding type ([#4015](https://github.com/dashpay/platform/issues/4015))
* **swift-sdk:** gate the ordered bring-up on the seed actually owning the wallet ([#4368](https://github.com/dashpay/platform/issues/4368))
* **swift-sdk:** isLocal = mine-or-tracked; promote wallet identities, fix observed-entry mislinking ([#4375](https://github.com/dashpay/platform/issues/4375))
* **test-suite:** stabilize platform e2e tests ([#4214](https://github.com/dashpay/platform/issues/4214))


### Performance Improvements

* **platform-wallet:** keep the contact fetch off the drain's repeating-rejection path ([#4373](https://github.com/dashpay/platform/issues/4373))


### Documentation

* **agents:** never commit working specs; enforce by discipline, not git plumbing
* **kotlin-sdk:** cite the Swift source in the signMessage KDoc, closes [#4259](https://github.com/dashpay/platform/issues/4259)


### Continuous Integration

* install Rust lint components for workspace tests ([#4294](https://github.com/dashpay/platform/issues/4294))
* keep coverage-instrumented build between runs ([#4296](https://github.com/dashpay/platform/issues/4296))
* lint test targets with clippy --all-targets ([#4330](https://github.com/dashpay/platform/issues/4330))
* release Kotlin and Swift SDKs with the platform release ([#4228](https://github.com/dashpay/platform/issues/4228))
* remove the CodeRabbit/PastaClaw AI review gate ([#4292](https://github.com/dashpay/platform/issues/4292))
* schedule Rust workspace tests on macOS or Linux self-hosted runners ([#4287](https://github.com/dashpay/platform/issues/4287))
* skip shielded Rust tests on PRs without shielded changes ([#4293](https://github.com/dashpay/platform/issues/4293))


### Code Refactoring

* **dpp:** extract shared try_from_schema parsing helpers ([#4276](https://github.com/dashpay/platform/issues/4276))
* **rs-sdk-ffi:** use the rs-sdk voting-key helpers instead of its own ([#4341](https://github.com/dashpay/platform/issues/4341))
* **sdk:** drop the vestigial v2 suffix from the finalized-transaction surface ([#4325](https://github.com/dashpay/platform/issues/4325))
* **sdk:** remove deprecated v1 split build/sign transaction surface ([#4323](https://github.com/dashpay/platform/issues/4323))


### Miscellaneous Chores

* fix rustfmt drift on v4.2-dev ([#4278](https://github.com/dashpay/platform/issues/4278))
* **platform-wallet:** bump key-wallet pin and migrate to new AddressState/asset-lock API ([#4305](https://github.com/dashpay/platform/issues/4305))
* **swift-sdk:** remove Account.derivePrivateKeyWIF ([#4339](https://github.com/dashpay/platform/issues/4339))
* update rust-dashcore to b056d07c ([#4343](https://github.com/dashpay/platform/issues/4343))


### Tests

* **drive-abci:** gate PR runs to one comprehensive chain simulation ([#4297](https://github.com/dashpay/platform/issues/4297))
* **sdk:** add proof-vector regression corpus for drive-proof-verifier ([#4345](https://github.com/dashpay/platform/issues/4345))

## [4.1.0](https://github.com/dashpay/platform/compare/v4.0.0...v4.1.0) (2026-07-27)


### ⚠ BREAKING CHANGES

* **dashmate:** bump gateway Envoy to 1.39.0 (#4233)
* **platform:** revise shielded identity-create denominations in protocol version 13
* **platform:** document history system contract with per-doctype opt-in (#4171)
* **kotlin-sdk:** provision DashPay registration keys (#4173)
* **platform:** enable DPNS username transfers and sales in protocol version 13 (#4145)
* managed identity top-up from asset lock (iOS + shared FFI) (#4093)

### Features

* **dashmate:** bump gateway Envoy to 1.39.0 ([#4233](https://github.com/dashpay/platform/issues/4233))
* complete dashpay in platform wallet and swift example app ([#3841](https://github.com/dashpay/platform/issues/3841))
* **contract:** qa-contract v5 tags redesign + publish testnet contract 9tshSfq5 ([#4031](https://github.com/dashpay/platform/issues/4031))
* document replace/delete/transfer, data-contract update, and Kotlin QA fixes ([#4110](https://github.com/dashpay/platform/issues/4110))
* **dpp:** unify JSON/Value conversion traits ([#3573](https://github.com/dashpay/platform/issues/3573))
* **kotlin-app:** multi-recipient Core send (CORE-10)
* **kotlin-sdk:** add Import Existing Wallet toggle to CreateWalletScreen (CORE-02)
* **kotlin-sdk:** add Kotlin SDK and KotlinExampleApp (Android port of SwiftExampleApp)
* **kotlin-sdk:** add maven-publish for the release AAR ([#4182](https://github.com/dashpay/platform/issues/4182))
* **kotlin-sdk:** bind every wallet's shielded sub-wallet for multi-wallet flows ([#4046](https://github.com/dashpay/platform/issues/4046))
* **kotlin-sdk:** bridge identity create-from-addresses (ID-08) and transfer-credits-to-addresses (ID-11)
* **kotlin-sdk:** bridge the DIP-15 auto-accept QR pair (K3)
* **kotlin-sdk:** DashPay detail, payment, profile and list screens + parity (K3 slice C)
* **kotlin-sdk:** DashPay hub, contacts, requests and add-contact screens (K3 slice B)
* **kotlin-sdk:** DashPay persistence completion + read bridges (K1), closes [#3841](https://github.com/dashpay/platform/issues/3841)
* **kotlin-sdk:** DashPay sync service, seedless unlock and writes (K2)
* **kotlin-sdk:** DashPay tab navigation + contact meta infra (K3 slice A)
* **kotlin-sdk:** Platform receive tab + testnet faucet in the Receive sheet
* **kotlin-sdk:** provision DashPay registration keys ([#4173](https://github.com/dashpay/platform/issues/4173))
* **kotlin-sdk:** reach the shielded screens from the WalletDetail balance card
* **kotlin-sdk:** show Platform balance in DASH with an actions menu
* **kotlin-sdk:** surface DPNS contested/premium names in Register Name
* **kotlin-sdk:** transaction decoder JNI binding over key-wallet-ffi transaction_decode ([#4187](https://github.com/dashpay/platform/issues/4187))
* **kotlin:** create identity from shielded pool (SH-11, Type 20)
* **kotlin:** Delete Wallet UI + removeWallet cascade (CORE-17)
* **kotlin:** resume UI for pending address top-up asset locks (ADDR-03)
* **kotlin:** shield from Platform balance (SH-03, Type 15)
* managed identity top-up from asset lock (iOS + shared FFI) ([#4093](https://github.com/dashpay/platform/issues/4093))
* **platform-wallet:** actively re-drive unconfirmed shielded spends ([#3988](https://github.com/dashpay/platform/issues/3988))
* **platform-wallet:** anchor fresh wallets at a checkpoint, set birth height on import ([#4063](https://github.com/dashpay/platform/issues/4063))
* **platform-wallet:** dip-13 dashpay invitations ([#4041](https://github.com/dashpay/platform/issues/4041))
* **platform-wallet:** expose classified connected SPV peers ([#4050](https://github.com/dashpay/platform/issues/4050))
* **platform-wallet:** expose SPV filter rescan via wallet synced-height rewind ([#4099](https://github.com/dashpay/platform/issues/4099))
* **platform-wallet:** fixed provider key derivation via rust-dashcore bump + legacy BLS display ([#4120](https://github.com/dashpay/platform/issues/4120))
* **platform-wallet:** persist Orchard viewing keys for seedless shielded bind ([#4126](https://github.com/dashpay/platform/issues/4126))
* **platform-wallet:** persist typed BLS/EdDSA provider keys as core address rows ([#4127](https://github.com/dashpay/platform/issues/4127))
* **platform-wallet:** return the exact network fee from send_payment ([#4095](https://github.com/dashpay/platform/issues/4095))
* **platform-wallet:** skip startup mnemonic touches via seed-binding marker + gated contact-crypto drain ([#4125](https://github.com/dashpay/platform/issues/4125))
* **platform-wallet:** surface ProRegTx masternode details in iOS via rust-dashcore bump ([#4112](https://github.com/dashpay/platform/issues/4112))
* **platform-wallet:** surface provider operator/node keys and reveal address private keys ([#4072](https://github.com/dashpay/platform/issues/4072))
* **platform-wallet:** wallet masternode list with DML-derived status in iOS ([#4116](https://github.com/dashpay/platform/issues/4116))
* **platform:** document history system contract with per-doctype opt-in ([#4171](https://github.com/dashpay/platform/issues/4171))
* **platform:** enable DPNS username transfers and sales in protocol version 13 ([#4145](https://github.com/dashpay/platform/issues/4145))
* **platform:** introduce protocol version 13 ([#4143](https://github.com/dashpay/platform/issues/4143))
* **platform:** revise shielded identity-create denominations in protocol version 13
* **sdk:** adapt Android bridges to the completed DashPay surface ([#3841](https://github.com/dashpay/platform/issues/3841)) + address-balance height pin, closes [#3650](https://github.com/dashpay/platform/issues/3650)
* **sdk:** add asset-lock identity top-up (Top Up from Core)
* **sdk:** add DOC-02 create-document flow to KotlinExampleApp
* **sdk:** close all remaining parity gaps (query catalog, identity keys, document pricing, voting, diagnostics)
* **sdk:** DashPay migration follow-ups (seed hygiene, double-send guard, contract-ref cleaner)
* **sdk:** gate the shielded denomination pickers on the network protocol version
* **sdk:** port v4.1-dev deltas to Android (platform-address transfer/withdraw, error namespacing, sync clear), closes [#3923](https://github.com/dashpay/platform/issues/3923) [#3959](https://github.com/dashpay/platform/issues/3959)
* **sdk:** start the Rust periodic sync loops and bind shielded wallets on Android
* **sdk:** wire shielded outflow (transfer/unshield/withdraw) + Shielded receive tab to Android
* **swift-example-app:** bind every wallet's shielded sub-wallet for multi-wallet flows ([#4038](https://github.com/dashpay/platform/issues/4038))
* **swift-example-app:** wallet-signed Transfer & Withdraw for platform addresses (ADDR-02/04) ([#3923](https://github.com/dashpay/platform/issues/3923))
* **swift-sdk:** auto-start Core SPV sync on app launch ([#4105](https://github.com/dashpay/platform/issues/4105))
* **swift-sdk:** compact-filter rescan button in Core Sync Status ([#4103](https://github.com/dashpay/platform/issues/4103))
* **swift-sdk:** in-app log export for beta diagnostics ([#4131](https://github.com/dashpay/platform/issues/4131))
* **swift-sdk:** partial-amount platform address withdrawal wrapper ([#4139](https://github.com/dashpay/platform/issues/4139))
* **swift-sdk:** public RawKeySigner one-shot raw-key signing ([#4097](https://github.com/dashpay/platform/issues/4097))
* **swift-sdk:** surface payload-only special-tx involvement in account transaction lists ([#4108](https://github.com/dashpay/platform/issues/4108))
* **swift-sdk:** transaction decoder over upstream key-wallet-ffi transaction_decode ([#3981](https://github.com/dashpay/platform/issues/3981))
* **wasm-sdk:** support tiered direct-purchase prices in tokenSetPrice

### Bug Fixes

* **dashmate:** re-pin Drive and DAPI images to the stable tag when upgrading from a release candidate ([#4241](https://github.com/dashpay/platform/issues/4241))
* **platform-wallet:** make shielded re-bind non-destructive so it cannot wipe an in-flight sync pass ([#4237](https://github.com/dashpay/platform/issues/4237))
* **dashmate:** migrate Drive and DAPI images onto the prerelease line ([#4235](https://github.com/dashpay/platform/issues/4235))
* **platform:** harden v12 to v13 protocol upgrades ([#4222](https://github.com/dashpay/platform/issues/4222))
* **drive:** version the compacted address-balance proof wire format ([#4224](https://github.com/dashpay/platform/issues/4224))
* **kotlin-sdk:** persist active DashPay identity ([#4211](https://github.com/dashpay/platform/issues/4211))
* **build:** keep github token out of git error output
* **build:** prevent secret tracing in dependency stages
* bump rust-dashcore to c88264e7 for Android SPV file-lock fix ([#4014](https://github.com/dashpay/platform/issues/4014))
* **ci:** block fork code on self-hosted Swift runner
* **ci:** bound Swift SDK build disk usage
* **contract:** restore KotlinExampleApp app-code entry lost in the v4.1-dev merge, closes [#4031](https://github.com/dashpay/platform/issues/4031)
* **dapi-client:** require metadata on proved history responses
* **dapi:** bound core request inputs
* **dapi:** bound path element queries ([#4153](https://github.com/dashpay/platform/issues/4153))
* **dapi:** bound request metric labels
* **dapi:** bound streaming request work
* **dapi:** deliver the header stream's terminal error on a full queue
* **dapi:** preserve bounded stream delivery state
* **dapi:** retain stream permits until replay workers stop and serve full history ranges
* **dapi:** validate bounded request payloads
* **dashmate:** abort over-budget archive extraction
* **dashmate:** contain generated config paths
* **dashmate:** isolate diagnostic archive extraction
* **dashmate:** log helper API failures server-side
* **dashmate:** restore label-less volumes on macOS stock bash
* **dashmate:** restrict helper API capabilities
* **dashmate:** stage state archive restores
* **dpp:** bound document value validation depth ([#4115](https://github.com/dashpay/platform/issues/4115))
* **drive-abci:** admit asset-lock proof checks
* **drive-abci:** bound CheckTx verification work
* **drive-abci:** bound contested value nesting depth ([#4104](https://github.com/dashpay/platform/issues/4104))
* **drive-abci:** bound value decoding in contested-resource queries ([#4101](https://github.com/dashpay/platform/issues/4101))
* **drive-abci:** enforce historical query bounds ([#4150](https://github.com/dashpay/platform/issues/4150))
* **drive-abci:** record unshield and shield-surplus credits in recent address balance changes ([#4142](https://github.com/dashpay/platform/issues/4142))
* **drive-abci:** report configured main control group authority when unauthorized ([#4209](https://github.com/dashpay/platform/issues/4209))
* **drive-abci:** use testnet Core RPC port in env ([#3965](https://github.com/dashpay/platform/issues/3965))
* **drive:** accept MMR-aligned shielded note proof ranges ([#4201](https://github.com/dashpay/platform/issues/4201))
* **drive:** bind and bound proof decoding ([#4165](https://github.com/dashpay/platform/issues/4165))
* **drive:** bind proof-verifier queries to trusted context ([#4166](https://github.com/dashpay/platform/issues/4166))
* **drive:** bound shielded anchor query work ([#4169](https://github.com/dashpay/platform/issues/4169))
* **drive:** enforce aggregate query limits ([#4149](https://github.com/dashpay/platform/issues/4149))
* **drive:** order ascending cursor range bounds ([#4148](https://github.com/dashpay/platform/issues/4148))
* **drive:** preserve typed proof query fields ([#4156](https://github.com/dashpay/platform/issues/4156))
* **drive:** reject zero effective distinct count limit ([#4197](https://github.com/dashpay/platform/issues/4197))
* **drive:** strengthen execution proof result validation (tagged-outcome variant) ([#4207](https://github.com/dashpay/platform/issues/4207))
* **js-sdk:** fail closed without proof verification
* **kotlin-app:** catch identity balance-refresh failures before they crash the screen
* **kotlin-app:** gate Replace/Delete on DPP effective document capabilities, closes [#3999](https://github.com/dashpay/platform/issues/3999)
* **kotlin-app:** only offer Platform shielding when Platform credits exist
* **kotlin-app:** refresh contact payment history once manager + wallet are available
* **kotlin-app:** show asset-lock burn amount instead of 0.00000000 DASH
* **kotlin-app:** show identity keys under Base58 id + add identity balance refresh
* **kotlin-sdk:** accept Base58 identity ids on the Load Identity screen ([#4017](https://github.com/dashpay/platform/issues/4017))
* **kotlin-sdk:** address Android JNI review issues ([#4002](https://github.com/dashpay/platform/issues/4002))
* **kotlin-sdk:** address multi-agent review findings on the wallet-lifecycle hardening commit
* **kotlin-sdk:** bind stored identity-key blobs to the current KEYS_ALIAS keypair
* **kotlin-sdk:** carry unrecoverable phrase in rollback error, durable privkey owner index
* **kotlin-sdk:** clean phantom owner-index entries, exclude wallet secrets from Android backup
* **kotlin-sdk:** clean up failed manager initialization
* **kotlin-sdk:** close the tombstone TOCTOU on createWallet, make alias-rollback ownership check atomic with delete
* **kotlin-sdk:** decode all-zero Base58 identifiers without a synthetic byte
* **kotlin-sdk:** don't swallow CancellationException in ProvenBalances
* **kotlin-sdk:** fold K1 code-review findings
* **kotlin-sdk:** fold K2 code-review findings
* **kotlin-sdk:** fold K3 code-review findings
* **kotlin-sdk:** give the unlocked-device state time to propagate before running tests
* **kotlin-sdk:** harden identity-key Keystore recovery and reconcile parity docs ([#4172](https://github.com/dashpay/platform/issues/4172))
* **kotlin-sdk:** harden wallet-lifecycle init cleanup, cross-wallet key ownership, and mnemonic-loss-on-rotation, closes [#3999](https://github.com/dashpay/platform/issues/3999)
* **kotlin-sdk:** include immature funds in the Empty Wallet guard
* **kotlin-sdk:** make storeIfAbsentRejectsATombstonedWallet a valid JUnit @Test method
* **kotlin-sdk:** only FK-link proven token balances to identities that exist locally
* **kotlin-sdk:** persist proven token balance after mint
* **kotlin-sdk:** propagate rollback failures, atomic alias deletion, caller-allocated create out-buffers
* **kotlin-sdk:** re-arm deletion tombstone on createWallet rollback, close alias-rollback cross-wallet gap
* **kotlin-sdk:** re-bind wallet-scoped services when the wallet set changes
* **kotlin-sdk:** reject malformed contact/profile strings instead of clearing metadata, closes [#3999](https://github.com/dashpay/platform/issues/3999)
* **kotlin-sdk:** restore the Empty Wallet placeholder in the Balances section
* **kotlin-sdk:** retain rollback mnemonic, fence the identity-key derive/store lifecycle
* **kotlin-sdk:** serialize wallet removal with key producers, track mnemonic persistence in rollback, checkout release tag
* **kotlin-sdk:** track only round-created aliases, retain cleanup state until deletion succeeds
* **kotlin-sdk:** wallet-scoped platform addresses, strict secret deletion, create-rollback Room cleanup
* **kotlin:** allow pre-programmed distribution recipients to open the token Claim row
* **kotlin:** allow the Direct Purchase action row now that the form computes the price, closes [#4043](https://github.com/dashpay/platform/issues/4043)
* **kotlin:** complete TeardownGate coverage, cancellation-safe teardown, key-material zeroization
* **kotlin:** compute direct-purchase price so token Buy can be submitted
* **kotlin:** DashPay Add Contact no longer crashes on unmanaged recipient profile
* **kotlin:** forward tx input outpoints through JNI for pending-input spend reconciliation
* **kotlin:** gate the last resolver borrows + lease Sdk queries against close
* **kotlin:** gate the new mutation bridges, lease castVote, harden Sdk close, and lint the fence, closes [#4110](https://github.com/dashpay/platform/issues/4110)
* **kotlin:** init AccountSpecFFI node-keys fields null on the JNI load path
* **kotlin:** key health flags undecryptable pre-RSA key blobs so Repair can re-derive them
* **kotlin:** manager teardown joins in-flight JNI + owns the persistence executor + mnemonic error codes, closes [#4103](https://github.com/dashpay/platform/issues/4103) [#4099](https://github.com/dashpay/platform/issues/4099)
* **kotlin:** preserve unconfirmed-create identity id, gate DashPay persisters, replace tri-state
* **kotlin:** reject required-field clears in replace + carry the unconfirmed-create diagnostic
* **kotlin:** suspend manager teardown (Main-thread ANR) + fence caller-owned handle borrows
* make Kotlin shielded Clear button functional on-device
* platform-address on-demand signing (ID-06) + shielded reset durability
* **platform-value:** reject lossy CBOR integer conversions ([#4146](https://github.com/dashpay/platform/issues/4146))
* **platform-wallet-ffi:** distinguish missing wallet handle from missing identity in get_dashpay_profile
* **platform-wallet:** auto-release a stranded shielded-spend reservation on sync ([#3982](https://github.com/dashpay/platform/issues/3982))
* **platform-wallet:** build shielded spends against a Platform-recorded anchor ([#3977](https://github.com/dashpay/platform/issues/3977))
* **platform-wallet:** data-integrity follow-ups from the [#3990](https://github.com/dashpay/platform/issues/3990) sync review ([#4008](https://github.com/dashpay/platform/issues/4008))
* **platform-wallet:** deliver typed AddressNonceMismatch error across wallet, FFI, and Swift host ([#4047](https://github.com/dashpay/platform/issues/4047))
* **platform-wallet:** enforce FFI lifetime invariants ([#4160](https://github.com/dashpay/platform/issues/4160))
* **platform-wallet:** freeze sync watermark on persistence fault — TXO loss/duplication ([#4069](https://github.com/dashpay/platform/issues/4069)) ([#4071](https://github.com/dashpay/platform/issues/4071))
* **platform-wallet:** gate ADDR-09 watermark invalidation inside the reconcile seam ([#4005](https://github.com/dashpay/platform/issues/4005))
* **platform-wallet:** height-pin address balances so delta replay cannot double-count (ADDR-09) ([#4019](https://github.com/dashpay/platform/issues/4019))
* **platform-wallet:** idempotent load_from_persistor to stop double-register crash
* **platform-wallet:** index-conflicting removal no longer orphans a restored address balance ([#4013](https://github.com/dashpay/platform/issues/4013))
* **platform-wallet:** keep prederived typed keys for legacy key-less rows + land the [#893](https://github.com/dashpay/platform/issues/893) crash-fix pin ([#4132](https://github.com/dashpay/platform/issues/4132))
* **platform-wallet:** make post-acceptance identity bookkeeping best-effort ([#4011](https://github.com/dashpay/platform/issues/4011))
* **platform-wallet:** only key group mint balance with an explicit recipient, closes [#4044](https://github.com/dashpay/platform/issues/4044)
* **platform-wallet:** persist address usage discovered during SPV block processing ([#4107](https://github.com/dashpay/platform/issues/4107))
* **platform-wallet:** persist the DashPay send used-flip after releasing the wallet-manager lock ([#4176](https://github.com/dashpay/platform/issues/4176))
* **platform-wallet:** persist top-up account + DashPay payment-address rotation across restart, closes [#3999](https://github.com/dashpay/platform/issues/3999)
* **platform-wallet:** poll platform-address transfer FFI on the big-stack worker ([#3989](https://github.com/dashpay/platform/issues/3989))
* **platform-wallet:** poll Sync Now FFI passes on big-stack threads to stop SIGBUS crash ([#4033](https://github.com/dashpay/platform/issues/4033))
* **platform-wallet:** re-broadcast a Broadcast-status lock on resume ([#4009](https://github.com/dashpay/platform/issues/4009))
* **platform-wallet:** reconcile platform-address balances after top-up-from-addresses ([#3969](https://github.com/dashpay/platform/issues/3969))
* **platform-wallet:** reflect asset-lock top-up balance once, not doubled ([#4004](https://github.com/dashpay/platform/issues/4004))
* **platform-wallet:** register data contract for returned-proof verification on writes ([#4035](https://github.com/dashpay/platform/issues/4035))
* **platform-wallet:** register fetched token contract for post-broadcast proof verification
* **platform-wallet:** release UTXO reservation when broadcast fails ([#3985](https://github.com/dashpay/platform/issues/3985))
* **platform-wallet:** report the exact send_payment fee (inputs − outputs) ([#4049](https://github.com/dashpay/platform/issues/4049))
* **platform-wallet:** satisfy cargo fmt import ordering in test_support.rs
* **platform-wallet:** scope broadcaster test mocks to cfg(test), not the test-utils feature
* **platform-wallet:** size withdrawal plan from on-chain balances ([#3994](https://github.com/dashpay/platform/issues/3994))
* **platform-wallet:** source ADDR-02 transfer input balances on-chain
* **platform-wallet:** spv client clear storage works after stopping the client ([#4042](https://github.com/dashpay/platform/issues/4042))
* **platform-wallet:** union hydrated balance map into address input selection
* **platform-wallet:** wait indefinitely for asset-lock ChainLock finality ([#4006](https://github.com/dashpay/platform/issues/4006))
* **platform:** bind token group authorization
* **platform:** identify the offending transition in group-action rebind error
* provision identity top-up asset-lock account for external-signable wallets
* **qa-contract:** align codes.mjs with v5 contract (System=10, no Group)
* **qa-contract:** unblock default seed run — schema tags property, header-aware plan parser, Swift plan to v5 shape
* **scripts:** validate operator restore inputs
* **sdk:** adapt Android core-wallet send to the new TransactionBuilder FFI, closes [#3970](https://github.com/dashpay/platform/issues/3970)
* **sdk:** address Android review round — JNI boundary guards, atomic handle refs, HASH160 add-key, TLS target gating, closes [#3999](https://github.com/dashpay/platform/issues/3999) [#4002](https://github.com/dashpay/platform/issues/4002)
* **sdk:** address parity review findings
* **sdk:** address-sync no longer silently discards balance changes for post-snapshot addresses (Found-025) ([#3650](https://github.com/dashpay/platform/issues/3650))
* **sdk:** authenticate missing contract history
* **sdk:** bound blob row count before allocating in JNI decoders
* **sdk:** consolidate Kotlin and Swift parity paths
* **sdk:** consume-once native handles for signer/resolver; commit network switch only on SDK success
* **sdk:** correct two JNI descriptor mismatches; scrub previewed key scalars
* **sdk:** default aggregate query limit to -1, not 0
* **sdk:** defer the dead getGroupInfos query (shared FFI lacks contract_id)
* **sdk:** enforce effective freshness floors
* **sdk:** enforce response freshness anchors
* **sdk:** fail load callbacks on malformed fixed-size IDs
* **sdk:** feed platform-address balances back on Android wallet load (SH-06 credit), closes [#4019](https://github.com/dashpay/platform/issues/4019) [#4019](https://github.com/dashpay/platform/issues/4019)
* **sdk:** gate API-30 keystore call on minSdk 29 + serialize KEYS_ALIAS first-use creation
* **sdk:** install protoc v32.0 in Kotlin CI (apt's 3.21 breaks tenderdash-proto)
* **sdk:** make build_android.sh exFAT detection macOS-only
* **sdk:** make empty-as-absent opt-in for optional id fields only
* **sdk:** make the split CoreTransactionBuilder API internal so the funding/signing race can't be reopened
* **sdk:** NativeLoader marks loaded only after load+init succeed
* **sdk:** pass the SDK through the wallet-manager cache mutex
* **sdk:** pin emulator DNS in Kotlin CI
* **sdk:** provision the full identity key set at creation, not just MASTER
* **sdk:** rehydrate asset-lock resume state on cold restart
* **sdk:** rehydrate Core UTXOs on Android wallet load (CORE-06 balance)
* **sdk:** reject negative money/index/id values at every public SDK seam
* **sdk:** replace java.lang.ref.Cleaner with an API-29-safe PhantomReference backstop
* **sdk:** restore core address pools so out-of-window UTXOs stay signable
* **sdk:** restore empty-array absent sentinel in fixed-size ID loads
* **sdk:** restore executable bits, rustfmt new FFI module, add testnet nightly
* **sdk:** restore identity public keys into IdentityManager on wallet load
* **sdk:** retryable ShieldedBroadcastFailed mapping, BIP-350 case guard, balance-service rebind
* **sdk:** roll back the manager registration on JNI-internal createWallet failures
* **sdk:** scan from genesis when recovering an orphan mnemonic (birth-height override)
* **sdk:** scan historical Core funds when importing a mnemonic (birth-height override)
* **sdk:** scope known-contract preload to the SDK network; guard two more blob decoders
* **sdk:** scope the wallet-list load to the manager's network
* **sdk:** serialize per-wallet Core sends so split-builder funding+signing stay atomic, closes [#3970](https://github.com/dashpay/platform/issues/3970)
* **sdk:** share the Core-send lock per wallet id across all wrappers
* **sdk:** split live shielded scan counter from lifetime total; destroy unpublishable wallet handle
* **sdk:** stage load-row buffers as owned Vecs; cooperative cancellation in the faucet PoW solver
* **sdk:** store identity keys without auth (RSA public-key encrypt), keep signing auth-gated
* **sdk:** tolerate untyped success() results in JNI unwrappers
* **sdk:** unregister the wallet from the native manager on createWallet rollback
* **sdk:** unwrap signer result as DashSDKSignature, not binary
* **swift-example-app:** compute direct-purchase price so token Buy can be submitted ([#4043](https://github.com/dashpay/platform/issues/4043))
* **swift-example-app:** fail closed on persistence recovery ([#4177](https://github.com/dashpay/platform/issues/4177))
* **swift-example-app:** let pre-programmed distribution recipients open token Claim ([#4048](https://github.com/dashpay/platform/issues/4048))
* **swift-example-app:** make Platform Sync "Clear" actually clear synced data ([#3959](https://github.com/dashpay/platform/issues/3959))
* **swift-example-app:** persist proven token balance after mint ([#4044](https://github.com/dashpay/platform/issues/4044))
* **swift-example-app:** resume orphaned Broadcast asset-lock top-ups ([#4007](https://github.com/dashpay/platform/issues/4007))
* **swift-example-app:** resume orphaned Broadcast identity-registration locks ([#4010](https://github.com/dashpay/platform/issues/4010))
* **swift-sdk:** cap the faucet captcha's aggregate proof-of-work ([#4100](https://github.com/dashpay/platform/issues/4100))
* **swift-sdk:** decode DIP-0018 platform addresses in address queries ([#4021](https://github.com/dashpay/platform/issues/4021))
* **swift-sdk:** detect later added wallet tx to a synced spv client ([#4062](https://github.com/dashpay/platform/issues/4062))
* **swift-sdk:** drop legacy headers pre-processing in build_ios.sh ([#3853](https://github.com/dashpay/platform/issues/3853))
* **swift-sdk:** label provider special txs instead of Self-Transfer ([#4109](https://github.com/dashpay/platform/issues/4109))
* **swift-sdk:** remove EstablishedContact's clone-mutating setters ([#4140](https://github.com/dashpay/platform/issues/4140))
* **swift-sdk:** remove the tmp todo in build_ios.sh ([#4040](https://github.com/dashpay/platform/issues/4040))
* **swift-sdk:** run withdrawal preflight async off the main actor ([#3995](https://github.com/dashpay/platform/issues/3995))
* **swift-sdk:** set CI keychain in user domain
* **swift-sdk:** stored default on keyType so pre-column stores can migrate ([#4129](https://github.com/dashpay/platform/issues/4129))
* **swift-sdk:** updated the swift integration tests to use the new tx broadcast flow ([#4037](https://github.com/dashpay/platform/issues/4037))
* **wallet-lib:** validate remote transaction proofs ([#4157](https://github.com/dashpay/platform/issues/4157))
* **wallet-lib:** validate SPV block header batches ([#4159](https://github.com/dashpay/platform/issues/4159))
* **wallet:** authoritative L1 broadcast outcomes with SPV peer-echo fallback, closes [dashpay/rust-dashcore#913](https://github.com/dashpay/rust-dashcore/issues/913) [dashpay/rust-dashcore#913](https://github.com/dashpay/rust-dashcore/issues/913) [#4181](https://github.com/dashpay/platform/issues/4181)
* **wallet:** release reservations when a broadcast provably never left the process
* **wasm-sdk:** accept zero-credit tiers in tokenSetPrice priceTiers
* **wasm-sdk:** protocol upgrade state misreported vote count as activation height ([#3979](https://github.com/dashpay/platform/issues/3979))
* **wasm-sdk:** tighten tokenSetPrice priceTiers and pricing-mode validation
* **wasm-sdk:** verify identity key searches ([#4158](https://github.com/dashpay/platform/issues/4158))

### Documentation

* document the new-npm-package publish gotcha ([#4219](https://github.com/dashpay/platform/issues/4219))
* document offset-version fix and release publishing ([#4213](https://github.com/dashpay/platform/issues/4213))
* add DashPay contact request encryption guide ([#3787](https://github.com/dashpay/platform/issues/3787))
* add missing group-read rows (TOK-18/19/20) to Android test plan ([#4118](https://github.com/dashpay/platform/issues/4118))
* **book:** fix drifted source links in the count group-by chapters ([#3974](https://github.com/dashpay/platform/issues/3974))
* **dapi:** note latest-version bound is a pre-filter
* **dashpay:** add Kotlin migration spec + PARITY interim correction, closes [#3841](https://github.com/dashpay/platform/issues/3841) [#3841](https://github.com/dashpay/platform/issues/3841)
* **kotlin-sdk:** add KotlinExampleApp QA test plan and app code
* point README badges + NIGHTLY_STATUS at v4.1-dev (new default) ([#3983](https://github.com/dashpay/platform/issues/3983))
* **qa:** backfill multiwallet tag on CORE-14..23 in both test plans
* **qa:** port ADDR-07/08/09 to Android test plan + reconcile ADDR-03 numbering
* retire SYS-07 — redundant with cross-wallet receive tests ([#4114](https://github.com/dashpay/platform/issues/4114))
* **sdk:** port DashPay test cases (DP-01..11) to the Kotlin TEST_PLAN + fix PARITY count
* **sdk:** sync onWalletChangesetTransaction KDoc to the corrected descriptor
* **swift-example-app:** drop ADDR-05 from the QA test plan ([#3998](https://github.com/dashpay/platform/issues/3998))
* **swift-example-app:** fix ADDR-05 category reference ([#4001](https://github.com/dashpay/platform/issues/4001))
* unify CLAUDE.md and AGENTS.md into one canonical agent-instructions file ([#4180](https://github.com/dashpay/platform/issues/4180))

### Code Refactoring

* **dapi:** make the last metrics label unbounded-proof
* **dpp:** pin denomination tests to explicit protocol versions
* **platform-wallet:** consolidate address-balance reconciliation into one guarded seam ([#3987](https://github.com/dashpay/platform/issues/3987))
* **platform-wallet:** dedup the account-address-pool snapshot loop, closes [#4041](https://github.com/dashpay/platform/issues/4041)
* **platform-wallet:** expose the new core TransactionBuilder API ([#3970](https://github.com/dashpay/platform/issues/3970))
* **qa-contract:** dissolve MultiWallet into tags for Kotlin test plan
* **sdk:** dedup shared wallet code + remove dead FFI/JNI chains ([#4106](https://github.com/dashpay/platform/issues/4106))
* **swift-sdk:** drop vestigial cross-network gate in wallet-deletion purge ([#4122](https://github.com/dashpay/platform/issues/4122))
* **swift-sdk:** promote the testnet faucet client into the SDK package ([#4098](https://github.com/dashpay/platform/issues/4098))
* **wallet:** drop the unused DapiBroadcaster overhaul
* **wallet:** follow dash-spv's removal of BIP61 reject handling, closes [rust-dashcore#913](https://github.com/dashpay/rust-dashcore/issues/913)
* **wallet:** pure-SPV broadcast path — no DAPI in the SPV wallet's send, closes [rust-dashcore#913](https://github.com/dashpay/rust-dashcore/issues/913)

### Tests

* **dapi:** enforce the metrics allowlist against the served protos
* **dapi:** make transactionsFilter bloom-filter test deterministic ([#4023](https://github.com/dashpay/platform/issues/4023))
* **drive-abci:** add token supply edge-case coverage ([#3849](https://github.com/dashpay/platform/issues/3849))
* **drive:** cover multi-range and paginated compacted balance proofs ([#4208](https://github.com/dashpay/platform/issues/4208))
* **drive:** cover shared-prefix aggregate index insertion ([#3961](https://github.com/dashpay/platform/issues/3961))
* **js-evo-sdk:** cover tiered token direct-purchase pricing in setPrice
* **js-evo-sdk:** fix unit test loader invocation
* **js-evo-sdk:** run unit tests with native type stripping instead of ts-node
* **kotlin:** pin the shielded-create payload codec boundary
* **platform-wallet:** fund wallet fixture with a chain-locked tx ([#4034](https://github.com/dashpay/platform/issues/4034))
* **platform-wallet:** pin Orchard key derivation to official ZIP-32 vectors ([#4032](https://github.com/dashpay/platform/issues/4032))
* **rs-sdk:** expect network floor in mock sdk seed test ([#3938](https://github.com/dashpay/platform/issues/3938))
* **sdk:** pin the identity/public-key restore round-trip
* **suite:** fail closed on cross-network verification and polish specs
* **suite:** wire an EvoSDK-backed platform proof verifier into client factories
* **swift-sdk:** first swift sdk integration tests with local network ([#3712](https://github.com/dashpay/platform/issues/3712))
* **swift-sdk:** port SpvRestart integration test off the removed ManagedCoreWallet.sendToAddresses, closes [#3970](https://github.com/dashpay/platform/issues/3970)
* **swift-sdk:** port the remaining two SpvRestart-sibling tests off ManagedCoreWallet.sendToAddresses, closes [#3970](https://github.com/dashpay/platform/issues/3970)
* **swift-sdk:** update address vectors to DIP-0018 after [#4021](https://github.com/dashpay/platform/issues/4021) ([#4024](https://github.com/dashpay/platform/issues/4024))

### Build System

* **docker:** build evo-sdk + wasm-sdk in the test-suite image ([#4215](https://github.com/dashpay/platform/issues/4215))
* bump grovedb to v5.0.1 ([#4119](https://github.com/dashpay/platform/issues/4119))

### Continuous Integration

* accept platform-wallet-storage as a conventional-commit scope ([#4061](https://github.com/dashpay/platform/issues/4061))
* cover rs-unified-sdk-jni and missing contract crates in package filters
* don't block PRs when CodeRabbit is rate limited ([#4175](https://github.com/dashpay/platform/issues/4175))
* fix Swift warnings-as-errors build and harden coverage cleanup retry, closes [#4198](https://github.com/dashpay/platform/issues/4198)
* **kotlin-sdk:** dismiss the keyguard after enrolling the emulator lock screen
* **kotlin-sdk:** enroll a CI emulator lock screen so KEYS_ALIAS RSA key generation works
* **kotlin:** retry emulator credential unlock
* **kotlin:** submit emulator unlock credential
* register rs-unified-sdk-jni in the wallet fast-path closure + fix clippy lints, closes [#4003](https://github.com/dashpay/platform/issues/4003)
* **rust:** allow cold macOS workspace builds to finish
* **rust:** cover shielded tests in macOS timeout
* **rust:** tolerate transient coverage cleanup races
* scoped Rust test fast path for wallet-only PRs ([#4003](https://github.com/dashpay/platform/issues/4003))
* **swift-sdk:** drop internal ticket reference from guard comments
* **swift-sdk:** trust thepastaclaw forks and harden runner policy checker
* unblock full-pipeline runs broken by untested paths, closes [#4171](https://github.com/dashpay/platform/issues/4171) [#4203](https://github.com/dashpay/platform/issues/4203)

### Styles

* **jni:** format transaction decoder
* **kotlin-sdk:** balance hero on WalletDetail + polished transaction rows
* **kotlin-sdk:** brand-cohesive theme + polish key KotlinExampleApp screens
* **kotlin-sdk:** consistent EntityRow rows in the state-transitions catalog
* **kotlin-sdk:** consistent list rows across Wallets/Identities/Contracts homes
* **kotlin-sdk:** replace leading-space icon spacing with Spacer on WalletDetail
* **platform-wallet:** cargo fmt after [#4071](https://github.com/dashpay/platform/issues/4071) watermark-freeze merge
* **sdk:** cargo fmt signer.rs (DashPay-migration merge left it unformatted)
* **sdk:** rustfmt the queries.rs import block after the getGroupInfos removal

### Miscellaneous Chores

* add project-level release skill
* bump rust-dashcore to 647fa98 ([#4022](https://github.com/dashpay/platform/issues/4022))
* bump rust-dashcore to afcff156, export xpub via ExtendedPubKeySigner ([#3976](https://github.com/dashpay/platform/issues/3976))
* **kotlin-example-app:** add emulator-control skill for KotlinExampleApp ([#4174](https://github.com/dashpay/platform/issues/4174))
* **kotlin-sdk:** apply rustfmt to rs-unified-sdk-jni, closes [#4192](https://github.com/dashpay/platform/issues/4192)
* **kotlin-sdk:** free more runner disk before the emulator to fix "no space left" flakes
* **kotlin-sdk:** tx-decode follow-up — blob hardening, net_from_ord hoist, prevVout docs ([#4187](https://github.com/dashpay/platform/issues/4187) review) ([#4192](https://github.com/dashpay/platform/issues/4192))
* pin rust-dashcore to the [rust-dashcore#913](https://github.com/dashpay/rust-dashcore/issues/913) merge commit
* remove ignored cargo build config
* **sdk:** drop the empty companion object left by the Cleaner migration
* **swift-sdk:** reduce swift-sdk test time in CI ([#3869](https://github.com/dashpay/platform/issues/3869))
* **swift-sdk:** script to get spv stortage from iOS sim ([#3950](https://github.com/dashpay/platform/issues/3950))
* update rust-dashcore to 1ee1c94 ([#4094](https://github.com/dashpay/platform/issues/4094))

## [4.1.0-rc.3](https://github.com/dashpay/platform/compare/v4.1.0-rc.2...v4.1.0-rc.3) (2026-07-27)


### Bug Fixes

* **dashmate:** migrate Drive and DAPI images onto the prerelease line ([#4235](https://github.com/dashpay/platform/issues/4235))

## [4.1.0-rc.2](https://github.com/dashpay/platform/compare/v4.1.0-rc.1...v4.1.0-rc.2) (2026-07-26)


### ⚠ BREAKING CHANGES

* **dashmate:** bump gateway Envoy to 1.39.0 (#4233)

### Features

* **dashmate:** bump gateway Envoy to 1.39.0 ([#4233](https://github.com/dashpay/platform/issues/4233))


### Bug Fixes

* **platform:** harden v12 to v13 protocol upgrades ([#4222](https://github.com/dashpay/platform/issues/4222))

## [4.1.0-rc.1](https://github.com/dashpay/platform/compare/v4.1.0-beta.2...v4.1.0-rc.1) (2026-07-24)


### Bug Fixes

* **drive:** version the compacted address-balance proof wire format ([#4224](https://github.com/dashpay/platform/issues/4224))


### Documentation

* document the new-npm-package publish gotcha ([#4219](https://github.com/dashpay/platform/issues/4219))

## [4.1.0-beta.2](https://github.com/dashpay/platform/compare/v4.1.0-beta.1...v4.1.0-beta.2) (2026-07-23)


### Bug Fixes

* **kotlin-sdk:** persist active DashPay identity ([#4211](https://github.com/dashpay/platform/issues/4211))


### Documentation

* document offset-version fix and release publishing ([#4213](https://github.com/dashpay/platform/issues/4213))


### Build System

* **docker:** build evo-sdk + wasm-sdk in the test-suite image ([#4215](https://github.com/dashpay/platform/issues/4215))

## [4.1.0-beta.1](https://github.com/dashpay/platform/compare/v4.0.0...v4.1.0-beta.1) (2026-07-23)


### ⚠ BREAKING CHANGES

* **platform:** revise shielded identity-create denominations in protocol version 13
* **platform:** document history system contract with per-doctype opt-in (#4171)
* **kotlin-sdk:** provision DashPay registration keys (#4173)
* **platform:** enable DPNS username transfers and sales in protocol version 13 (#4145)
* managed identity top-up from asset lock (iOS + shared FFI) (#4093)

### Features

* complete dashpay in platform wallet and swift example app ([#3841](https://github.com/dashpay/platform/issues/3841))
* **contract:** qa-contract v5 tags redesign + publish testnet contract 9tshSfq5 ([#4031](https://github.com/dashpay/platform/issues/4031))
* document replace/delete/transfer, data-contract update, and Kotlin QA fixes ([#4110](https://github.com/dashpay/platform/issues/4110))
* **dpp:** unify JSON/Value conversion traits ([#3573](https://github.com/dashpay/platform/issues/3573))
* **kotlin-app:** multi-recipient Core send (CORE-10)
* **kotlin-sdk:** add Import Existing Wallet toggle to CreateWalletScreen (CORE-02)
* **kotlin-sdk:** add Kotlin SDK and KotlinExampleApp (Android port of SwiftExampleApp)
* **kotlin-sdk:** add maven-publish for the release AAR ([#4182](https://github.com/dashpay/platform/issues/4182))
* **kotlin-sdk:** bind every wallet's shielded sub-wallet for multi-wallet flows ([#4046](https://github.com/dashpay/platform/issues/4046))
* **kotlin-sdk:** bridge identity create-from-addresses (ID-08) and transfer-credits-to-addresses (ID-11)
* **kotlin-sdk:** bridge the DIP-15 auto-accept QR pair (K3)
* **kotlin-sdk:** DashPay detail, payment, profile and list screens + parity (K3 slice C)
* **kotlin-sdk:** DashPay hub, contacts, requests and add-contact screens (K3 slice B)
* **kotlin-sdk:** DashPay persistence completion + read bridges (K1), closes [#3841](https://github.com/dashpay/platform/issues/3841)
* **kotlin-sdk:** DashPay sync service, seedless unlock and writes (K2)
* **kotlin-sdk:** DashPay tab navigation + contact meta infra (K3 slice A)
* **kotlin-sdk:** Platform receive tab + testnet faucet in the Receive sheet
* **kotlin-sdk:** provision DashPay registration keys ([#4173](https://github.com/dashpay/platform/issues/4173))
* **kotlin-sdk:** reach the shielded screens from the WalletDetail balance card
* **kotlin-sdk:** show Platform balance in DASH with an actions menu
* **kotlin-sdk:** surface DPNS contested/premium names in Register Name
* **kotlin-sdk:** transaction decoder JNI binding over key-wallet-ffi transaction_decode ([#4187](https://github.com/dashpay/platform/issues/4187))
* **kotlin:** create identity from shielded pool (SH-11, Type 20)
* **kotlin:** Delete Wallet UI + removeWallet cascade (CORE-17)
* **kotlin:** resume UI for pending address top-up asset locks (ADDR-03)
* **kotlin:** shield from Platform balance (SH-03, Type 15)
* managed identity top-up from asset lock (iOS + shared FFI) ([#4093](https://github.com/dashpay/platform/issues/4093))
* **platform-wallet:** actively re-drive unconfirmed shielded spends ([#3988](https://github.com/dashpay/platform/issues/3988))
* **platform-wallet:** anchor fresh wallets at a checkpoint, set birth height on import ([#4063](https://github.com/dashpay/platform/issues/4063))
* **platform-wallet:** dip-13 dashpay invitations ([#4041](https://github.com/dashpay/platform/issues/4041))
* **platform-wallet:** expose classified connected SPV peers ([#4050](https://github.com/dashpay/platform/issues/4050))
* **platform-wallet:** expose SPV filter rescan via wallet synced-height rewind ([#4099](https://github.com/dashpay/platform/issues/4099))
* **platform-wallet:** fixed provider key derivation via rust-dashcore bump + legacy BLS display ([#4120](https://github.com/dashpay/platform/issues/4120))
* **platform-wallet:** persist Orchard viewing keys for seedless shielded bind ([#4126](https://github.com/dashpay/platform/issues/4126))
* **platform-wallet:** persist typed BLS/EdDSA provider keys as core address rows ([#4127](https://github.com/dashpay/platform/issues/4127))
* **platform-wallet:** return the exact network fee from send_payment ([#4095](https://github.com/dashpay/platform/issues/4095))
* **platform-wallet:** skip startup mnemonic touches via seed-binding marker + gated contact-crypto drain ([#4125](https://github.com/dashpay/platform/issues/4125))
* **platform-wallet:** surface ProRegTx masternode details in iOS via rust-dashcore bump ([#4112](https://github.com/dashpay/platform/issues/4112))
* **platform-wallet:** surface provider operator/node keys and reveal address private keys ([#4072](https://github.com/dashpay/platform/issues/4072))
* **platform-wallet:** wallet masternode list with DML-derived status in iOS ([#4116](https://github.com/dashpay/platform/issues/4116))
* **platform:** document history system contract with per-doctype opt-in ([#4171](https://github.com/dashpay/platform/issues/4171))
* **platform:** enable DPNS username transfers and sales in protocol version 13 ([#4145](https://github.com/dashpay/platform/issues/4145))
* **platform:** introduce protocol version 13 ([#4143](https://github.com/dashpay/platform/issues/4143))
* **platform:** revise shielded identity-create denominations in protocol version 13
* **sdk:** adapt Android bridges to the completed DashPay surface ([#3841](https://github.com/dashpay/platform/issues/3841)) + address-balance height pin, closes [#3650](https://github.com/dashpay/platform/issues/3650)
* **sdk:** add asset-lock identity top-up (Top Up from Core)
* **sdk:** add DOC-02 create-document flow to KotlinExampleApp
* **sdk:** close all remaining parity gaps (query catalog, identity keys, document pricing, voting, diagnostics)
* **sdk:** DashPay migration follow-ups (seed hygiene, double-send guard, contract-ref cleaner)
* **sdk:** gate the shielded denomination pickers on the network protocol version
* **sdk:** port v4.1-dev deltas to Android (platform-address transfer/withdraw, error namespacing, sync clear), closes [#3923](https://github.com/dashpay/platform/issues/3923) [#3959](https://github.com/dashpay/platform/issues/3959)
* **sdk:** start the Rust periodic sync loops and bind shielded wallets on Android
* **sdk:** wire shielded outflow (transfer/unshield/withdraw) + Shielded receive tab to Android
* **swift-example-app:** bind every wallet's shielded sub-wallet for multi-wallet flows ([#4038](https://github.com/dashpay/platform/issues/4038))
* **swift-example-app:** wallet-signed Transfer & Withdraw for platform addresses (ADDR-02/04) ([#3923](https://github.com/dashpay/platform/issues/3923))
* **swift-sdk:** auto-start Core SPV sync on app launch ([#4105](https://github.com/dashpay/platform/issues/4105))
* **swift-sdk:** compact-filter rescan button in Core Sync Status ([#4103](https://github.com/dashpay/platform/issues/4103))
* **swift-sdk:** in-app log export for beta diagnostics ([#4131](https://github.com/dashpay/platform/issues/4131))
* **swift-sdk:** partial-amount platform address withdrawal wrapper ([#4139](https://github.com/dashpay/platform/issues/4139))
* **swift-sdk:** public RawKeySigner one-shot raw-key signing ([#4097](https://github.com/dashpay/platform/issues/4097))
* **swift-sdk:** surface payload-only special-tx involvement in account transaction lists ([#4108](https://github.com/dashpay/platform/issues/4108))
* **swift-sdk:** transaction decoder over upstream key-wallet-ffi transaction_decode ([#3981](https://github.com/dashpay/platform/issues/3981))
* **wasm-sdk:** support tiered direct-purchase prices in tokenSetPrice


### Bug Fixes

* **build:** keep github token out of git error output
* **build:** prevent secret tracing in dependency stages
* bump rust-dashcore to c88264e7 for Android SPV file-lock fix ([#4014](https://github.com/dashpay/platform/issues/4014))
* **ci:** block fork code on self-hosted Swift runner
* **ci:** bound Swift SDK build disk usage
* **contract:** restore KotlinExampleApp app-code entry lost in the v4.1-dev merge, closes [#4031](https://github.com/dashpay/platform/issues/4031)
* **dapi-client:** require metadata on proved history responses
* **dapi:** bound core request inputs
* **dapi:** bound path element queries ([#4153](https://github.com/dashpay/platform/issues/4153))
* **dapi:** bound request metric labels
* **dapi:** bound streaming request work
* **dapi:** deliver the header stream's terminal error on a full queue
* **dapi:** preserve bounded stream delivery state
* **dapi:** retain stream permits until replay workers stop and serve full history ranges
* **dapi:** validate bounded request payloads
* **dashmate:** abort over-budget archive extraction
* **dashmate:** contain generated config paths
* **dashmate:** isolate diagnostic archive extraction
* **dashmate:** log helper API failures server-side
* **dashmate:** restore label-less volumes on macOS stock bash
* **dashmate:** restrict helper API capabilities
* **dashmate:** stage state archive restores
* **dpp:** bound document value validation depth ([#4115](https://github.com/dashpay/platform/issues/4115))
* **drive-abci:** admit asset-lock proof checks
* **drive-abci:** bound CheckTx verification work
* **drive-abci:** bound contested value nesting depth ([#4104](https://github.com/dashpay/platform/issues/4104))
* **drive-abci:** bound value decoding in contested-resource queries ([#4101](https://github.com/dashpay/platform/issues/4101))
* **drive-abci:** enforce historical query bounds ([#4150](https://github.com/dashpay/platform/issues/4150))
* **drive-abci:** record unshield and shield-surplus credits in recent address balance changes ([#4142](https://github.com/dashpay/platform/issues/4142))
* **drive-abci:** report configured main control group authority when unauthorized ([#4209](https://github.com/dashpay/platform/issues/4209))
* **drive-abci:** use testnet Core RPC port in env ([#3965](https://github.com/dashpay/platform/issues/3965))
* **drive:** accept MMR-aligned shielded note proof ranges ([#4201](https://github.com/dashpay/platform/issues/4201))
* **drive:** bind and bound proof decoding ([#4165](https://github.com/dashpay/platform/issues/4165))
* **drive:** bind proof-verifier queries to trusted context ([#4166](https://github.com/dashpay/platform/issues/4166))
* **drive:** bound shielded anchor query work ([#4169](https://github.com/dashpay/platform/issues/4169))
* **drive:** enforce aggregate query limits ([#4149](https://github.com/dashpay/platform/issues/4149))
* **drive:** order ascending cursor range bounds ([#4148](https://github.com/dashpay/platform/issues/4148))
* **drive:** preserve typed proof query fields ([#4156](https://github.com/dashpay/platform/issues/4156))
* **drive:** reject zero effective distinct count limit ([#4197](https://github.com/dashpay/platform/issues/4197))
* **drive:** strengthen execution proof result validation (tagged-outcome variant) ([#4207](https://github.com/dashpay/platform/issues/4207))
* **js-sdk:** fail closed without proof verification
* **kotlin-app:** catch identity balance-refresh failures before they crash the screen
* **kotlin-app:** gate Replace/Delete on DPP effective document capabilities, closes [#3999](https://github.com/dashpay/platform/issues/3999)
* **kotlin-app:** only offer Platform shielding when Platform credits exist
* **kotlin-app:** refresh contact payment history once manager + wallet are available
* **kotlin-app:** show asset-lock burn amount instead of 0.00000000 DASH
* **kotlin-app:** show identity keys under Base58 id + add identity balance refresh
* **kotlin-sdk:** accept Base58 identity ids on the Load Identity screen ([#4017](https://github.com/dashpay/platform/issues/4017))
* **kotlin-sdk:** address Android JNI review issues ([#4002](https://github.com/dashpay/platform/issues/4002))
* **kotlin-sdk:** address multi-agent review findings on the wallet-lifecycle hardening commit
* **kotlin-sdk:** bind stored identity-key blobs to the current KEYS_ALIAS keypair
* **kotlin-sdk:** carry unrecoverable phrase in rollback error, durable privkey owner index
* **kotlin-sdk:** clean phantom owner-index entries, exclude wallet secrets from Android backup
* **kotlin-sdk:** clean up failed manager initialization
* **kotlin-sdk:** close the tombstone TOCTOU on createWallet, make alias-rollback ownership check atomic with delete
* **kotlin-sdk:** decode all-zero Base58 identifiers without a synthetic byte
* **kotlin-sdk:** don't swallow CancellationException in ProvenBalances
* **kotlin-sdk:** fold K1 code-review findings
* **kotlin-sdk:** fold K2 code-review findings
* **kotlin-sdk:** fold K3 code-review findings
* **kotlin-sdk:** give the unlocked-device state time to propagate before running tests
* **kotlin-sdk:** harden identity-key Keystore recovery and reconcile parity docs ([#4172](https://github.com/dashpay/platform/issues/4172))
* **kotlin-sdk:** harden wallet-lifecycle init cleanup, cross-wallet key ownership, and mnemonic-loss-on-rotation, closes [#3999](https://github.com/dashpay/platform/issues/3999)
* **kotlin-sdk:** include immature funds in the Empty Wallet guard
* **kotlin-sdk:** make storeIfAbsentRejectsATombstonedWallet a valid JUnit @Test method
* **kotlin-sdk:** only FK-link proven token balances to identities that exist locally
* **kotlin-sdk:** persist proven token balance after mint
* **kotlin-sdk:** propagate rollback failures, atomic alias deletion, caller-allocated create out-buffers
* **kotlin-sdk:** re-arm deletion tombstone on createWallet rollback, close alias-rollback cross-wallet gap
* **kotlin-sdk:** re-bind wallet-scoped services when the wallet set changes
* **kotlin-sdk:** reject malformed contact/profile strings instead of clearing metadata, closes [#3999](https://github.com/dashpay/platform/issues/3999)
* **kotlin-sdk:** restore the Empty Wallet placeholder in the Balances section
* **kotlin-sdk:** retain rollback mnemonic, fence the identity-key derive/store lifecycle
* **kotlin-sdk:** serialize wallet removal with key producers, track mnemonic persistence in rollback, checkout release tag
* **kotlin-sdk:** track only round-created aliases, retain cleanup state until deletion succeeds
* **kotlin-sdk:** wallet-scoped platform addresses, strict secret deletion, create-rollback Room cleanup
* **kotlin:** allow pre-programmed distribution recipients to open the token Claim row
* **kotlin:** allow the Direct Purchase action row now that the form computes the price, closes [#4043](https://github.com/dashpay/platform/issues/4043)
* **kotlin:** complete TeardownGate coverage, cancellation-safe teardown, key-material zeroization
* **kotlin:** compute direct-purchase price so token Buy can be submitted
* **kotlin:** DashPay Add Contact no longer crashes on unmanaged recipient profile
* **kotlin:** forward tx input outpoints through JNI for pending-input spend reconciliation
* **kotlin:** gate the last resolver borrows + lease Sdk queries against close
* **kotlin:** gate the new mutation bridges, lease castVote, harden Sdk close, and lint the fence, closes [#4110](https://github.com/dashpay/platform/issues/4110)
* **kotlin:** init AccountSpecFFI node-keys fields null on the JNI load path
* **kotlin:** key health flags undecryptable pre-RSA key blobs so Repair can re-derive them
* **kotlin:** manager teardown joins in-flight JNI + owns the persistence executor + mnemonic error codes, closes [#4103](https://github.com/dashpay/platform/issues/4103) [#4099](https://github.com/dashpay/platform/issues/4099)
* **kotlin:** preserve unconfirmed-create identity id, gate DashPay persisters, replace tri-state
* **kotlin:** reject required-field clears in replace + carry the unconfirmed-create diagnostic
* **kotlin:** suspend manager teardown (Main-thread ANR) + fence caller-owned handle borrows
* make Kotlin shielded Clear button functional on-device
* platform-address on-demand signing (ID-06) + shielded reset durability
* **platform-value:** reject lossy CBOR integer conversions ([#4146](https://github.com/dashpay/platform/issues/4146))
* **platform-wallet-ffi:** distinguish missing wallet handle from missing identity in get_dashpay_profile
* **platform-wallet:** auto-release a stranded shielded-spend reservation on sync ([#3982](https://github.com/dashpay/platform/issues/3982))
* **platform-wallet:** build shielded spends against a Platform-recorded anchor ([#3977](https://github.com/dashpay/platform/issues/3977))
* **platform-wallet:** data-integrity follow-ups from the [#3990](https://github.com/dashpay/platform/issues/3990) sync review ([#4008](https://github.com/dashpay/platform/issues/4008))
* **platform-wallet:** deliver typed AddressNonceMismatch error across wallet, FFI, and Swift host ([#4047](https://github.com/dashpay/platform/issues/4047))
* **platform-wallet:** enforce FFI lifetime invariants ([#4160](https://github.com/dashpay/platform/issues/4160))
* **platform-wallet:** freeze sync watermark on persistence fault — TXO loss/duplication ([#4069](https://github.com/dashpay/platform/issues/4069)) ([#4071](https://github.com/dashpay/platform/issues/4071))
* **platform-wallet:** gate ADDR-09 watermark invalidation inside the reconcile seam ([#4005](https://github.com/dashpay/platform/issues/4005))
* **platform-wallet:** gate ADDR-09 watermark invalidation inside the reconcile seam ([#4005](https://github.com/dashpay/platform/issues/4005))
* **platform-wallet:** height-pin address balances so delta replay cannot double-count (ADDR-09) ([#4019](https://github.com/dashpay/platform/issues/4019))
* **platform-wallet:** idempotent load_from_persistor to stop double-register crash
* **platform-wallet:** index-conflicting removal no longer orphans a restored address balance ([#4013](https://github.com/dashpay/platform/issues/4013))
* **platform-wallet:** keep prederived typed keys for legacy key-less rows + land the [#893](https://github.com/dashpay/platform/issues/893) crash-fix pin ([#4132](https://github.com/dashpay/platform/issues/4132))
* **platform-wallet:** make post-acceptance identity bookkeeping best-effort ([#4011](https://github.com/dashpay/platform/issues/4011))
* **platform-wallet:** only key group mint balance with an explicit recipient, closes [#4044](https://github.com/dashpay/platform/issues/4044)
* **platform-wallet:** persist address usage discovered during SPV block processing ([#4107](https://github.com/dashpay/platform/issues/4107))
* **platform-wallet:** persist the DashPay send used-flip after releasing the wallet-manager lock ([#4176](https://github.com/dashpay/platform/issues/4176))
* **platform-wallet:** persist top-up account + DashPay payment-address rotation across restart, closes [#3999](https://github.com/dashpay/platform/issues/3999)
* **platform-wallet:** poll platform-address transfer FFI on the big-stack worker ([#3989](https://github.com/dashpay/platform/issues/3989))
* **platform-wallet:** poll Sync Now FFI passes on big-stack threads to stop SIGBUS crash ([#4033](https://github.com/dashpay/platform/issues/4033))
* **platform-wallet:** re-broadcast a Broadcast-status lock on resume ([#4009](https://github.com/dashpay/platform/issues/4009))
* **platform-wallet:** reconcile platform-address balances after top-up-from-addresses ([#3969](https://github.com/dashpay/platform/issues/3969))
* **platform-wallet:** reflect asset-lock top-up balance once, not doubled ([#4004](https://github.com/dashpay/platform/issues/4004))
* **platform-wallet:** reflect asset-lock top-up balance once, not doubled ([#4004](https://github.com/dashpay/platform/issues/4004))
* **platform-wallet:** register data contract for returned-proof verification on writes ([#4035](https://github.com/dashpay/platform/issues/4035))
* **platform-wallet:** register fetched token contract for post-broadcast proof verification
* **platform-wallet:** release UTXO reservation when broadcast fails ([#3985](https://github.com/dashpay/platform/issues/3985))
* **platform-wallet:** report the exact send_payment fee (inputs − outputs) ([#4049](https://github.com/dashpay/platform/issues/4049))
* **platform-wallet:** satisfy cargo fmt import ordering in test_support.rs
* **platform-wallet:** scope broadcaster test mocks to cfg(test), not the test-utils feature
* **platform-wallet:** size withdrawal plan from on-chain balances ([#3994](https://github.com/dashpay/platform/issues/3994))
* **platform-wallet:** source ADDR-02 transfer input balances on-chain
* **platform-wallet:** spv client clear storage works after stopping the client ([#4042](https://github.com/dashpay/platform/issues/4042))
* **platform-wallet:** union hydrated balance map into address input selection
* **platform-wallet:** wait indefinitely for asset-lock ChainLock finality ([#4006](https://github.com/dashpay/platform/issues/4006))
* **platform:** bind token group authorization
* **platform:** identify the offending transition in group-action rebind error
* provision identity top-up asset-lock account for external-signable wallets
* **qa-contract:** align codes.mjs with v5 contract (System=10, no Group)
* **qa-contract:** unblock default seed run — schema tags property, header-aware plan parser, Swift plan to v5 shape
* **scripts:** validate operator restore inputs
* **sdk:** adapt Android core-wallet send to the new TransactionBuilder FFI, closes [#3970](https://github.com/dashpay/platform/issues/3970)
* **sdk:** address Android review round — JNI boundary guards, atomic handle refs, HASH160 add-key, TLS target gating, closes [#3999](https://github.com/dashpay/platform/issues/3999) [#4002](https://github.com/dashpay/platform/issues/4002)
* **sdk:** address parity review findings
* **sdk:** address-sync no longer silently discards balance changes for post-snapshot addresses (Found-025) ([#3650](https://github.com/dashpay/platform/issues/3650))
* **sdk:** authenticate missing contract history
* **sdk:** bound blob row count before allocating in JNI decoders
* **sdk:** consolidate Kotlin and Swift parity paths
* **sdk:** consume-once native handles for signer/resolver; commit network switch only on SDK success
* **sdk:** correct two JNI descriptor mismatches; scrub previewed key scalars
* **sdk:** default aggregate query limit to -1, not 0
* **sdk:** defer the dead getGroupInfos query (shared FFI lacks contract_id)
* **sdk:** enforce effective freshness floors
* **sdk:** enforce response freshness anchors
* **sdk:** fail load callbacks on malformed fixed-size IDs
* **sdk:** feed platform-address balances back on Android wallet load (SH-06 credit), closes [#4019](https://github.com/dashpay/platform/issues/4019) [#4019](https://github.com/dashpay/platform/issues/4019)
* **sdk:** gate API-30 keystore call on minSdk 29 + serialize KEYS_ALIAS first-use creation
* **sdk:** install protoc v32.0 in Kotlin CI (apt's 3.21 breaks tenderdash-proto)
* **sdk:** make build_android.sh exFAT detection macOS-only
* **sdk:** make empty-as-absent opt-in for optional id fields only
* **sdk:** make the split CoreTransactionBuilder API internal so the funding/signing race can't be reopened
* **sdk:** NativeLoader marks loaded only after load+init succeed
* **sdk:** pass the SDK through the wallet-manager cache mutex
* **sdk:** pin emulator DNS in Kotlin CI
* **sdk:** provision the full identity key set at creation, not just MASTER
* **sdk:** rehydrate asset-lock resume state on cold restart
* **sdk:** rehydrate Core UTXOs on Android wallet load (CORE-06 balance)
* **sdk:** reject negative money/index/id values at every public SDK seam
* **sdk:** replace java.lang.ref.Cleaner with an API-29-safe PhantomReference backstop
* **sdk:** restore core address pools so out-of-window UTXOs stay signable
* **sdk:** restore empty-array absent sentinel in fixed-size ID loads
* **sdk:** restore executable bits, rustfmt new FFI module, add testnet nightly
* **sdk:** restore identity public keys into IdentityManager on wallet load
* **sdk:** retryable ShieldedBroadcastFailed mapping, BIP-350 case guard, balance-service rebind
* **sdk:** roll back the manager registration on JNI-internal createWallet failures
* **sdk:** scan from genesis when recovering an orphan mnemonic (birth-height override)
* **sdk:** scan historical Core funds when importing a mnemonic (birth-height override)
* **sdk:** scope known-contract preload to the SDK network; guard two more blob decoders
* **sdk:** scope the wallet-list load to the manager's network
* **sdk:** serialize per-wallet Core sends so split-builder funding+signing stay atomic, closes [#3970](https://github.com/dashpay/platform/issues/3970)
* **sdk:** share the Core-send lock per wallet id across all wrappers
* **sdk:** split live shielded scan counter from lifetime total; destroy unpublishable wallet handle
* **sdk:** stage load-row buffers as owned Vecs; cooperative cancellation in the faucet PoW solver
* **sdk:** store identity keys without auth (RSA public-key encrypt), keep signing auth-gated
* **sdk:** tolerate untyped success() results in JNI unwrappers
* **sdk:** unregister the wallet from the native manager on createWallet rollback
* **sdk:** unwrap signer result as DashSDKSignature, not binary
* **swift-example-app:** compute direct-purchase price so token Buy can be submitted ([#4043](https://github.com/dashpay/platform/issues/4043))
* **swift-example-app:** fail closed on persistence recovery ([#4177](https://github.com/dashpay/platform/issues/4177))
* **swift-example-app:** let pre-programmed distribution recipients open token Claim ([#4048](https://github.com/dashpay/platform/issues/4048))
* **swift-example-app:** make Platform Sync "Clear" actually clear synced data ([#3959](https://github.com/dashpay/platform/issues/3959))
* **swift-example-app:** persist proven token balance after mint ([#4044](https://github.com/dashpay/platform/issues/4044))
* **swift-example-app:** resume orphaned Broadcast asset-lock top-ups ([#4007](https://github.com/dashpay/platform/issues/4007))
* **swift-example-app:** resume orphaned Broadcast identity-registration locks ([#4010](https://github.com/dashpay/platform/issues/4010))
* **swift-sdk:** cap the faucet captcha's aggregate proof-of-work ([#4100](https://github.com/dashpay/platform/issues/4100))
* **swift-sdk:** decode DIP-0018 platform addresses in address queries ([#4021](https://github.com/dashpay/platform/issues/4021))
* **swift-sdk:** detect later added wallet tx to a synced spv client ([#4062](https://github.com/dashpay/platform/issues/4062))
* **swift-sdk:** drop legacy headers pre-processing in build_ios.sh ([#3853](https://github.com/dashpay/platform/issues/3853))
* **swift-sdk:** label provider special txs instead of Self-Transfer ([#4109](https://github.com/dashpay/platform/issues/4109))
* **swift-sdk:** remove EstablishedContact's clone-mutating setters ([#4140](https://github.com/dashpay/platform/issues/4140))
* **swift-sdk:** remove the tmp todo in build_ios.sh ([#4040](https://github.com/dashpay/platform/issues/4040))
* **swift-sdk:** run withdrawal preflight async off the main actor ([#3995](https://github.com/dashpay/platform/issues/3995))
* **swift-sdk:** set CI keychain in user domain
* **swift-sdk:** stored default on keyType so pre-column stores can migrate ([#4129](https://github.com/dashpay/platform/issues/4129))
* **swift-sdk:** updated the swift integration tests to use the new tx broadcast flow ([#4037](https://github.com/dashpay/platform/issues/4037))
* **wallet-lib:** validate remote transaction proofs ([#4157](https://github.com/dashpay/platform/issues/4157))
* **wallet-lib:** validate SPV block header batches ([#4159](https://github.com/dashpay/platform/issues/4159))
* **wallet:** authoritative L1 broadcast outcomes with SPV peer-echo fallback, closes [dashpay/rust-dashcore#913](https://github.com/dashpay/rust-dashcore/issues/913) [dashpay/rust-dashcore#913](https://github.com/dashpay/rust-dashcore/issues/913) [#4181](https://github.com/dashpay/platform/issues/4181)
* **wallet:** release reservations when a broadcast provably never left the process
* **wasm-sdk:** accept zero-credit tiers in tokenSetPrice priceTiers
* **wasm-sdk:** protocol upgrade state misreported vote count as activation height ([#3979](https://github.com/dashpay/platform/issues/3979))
* **wasm-sdk:** tighten tokenSetPrice priceTiers and pricing-mode validation
* **wasm-sdk:** verify identity key searches ([#4158](https://github.com/dashpay/platform/issues/4158))


### Build System

* bump grovedb to v5.0.1 ([#4119](https://github.com/dashpay/platform/issues/4119))


### Styles

* **jni:** format transaction decoder
* **jni:** format transaction decoder
* **kotlin-sdk:** balance hero on WalletDetail + polished transaction rows
* **kotlin-sdk:** brand-cohesive theme + polish key KotlinExampleApp screens
* **kotlin-sdk:** consistent EntityRow rows in the state-transitions catalog
* **kotlin-sdk:** consistent list rows across Wallets/Identities/Contracts homes
* **kotlin-sdk:** replace leading-space icon spacing with Spacer on WalletDetail
* **platform-wallet:** cargo fmt after [#4071](https://github.com/dashpay/platform/issues/4071) watermark-freeze merge
* **sdk:** cargo fmt signer.rs (DashPay-migration merge left it unformatted)
* **sdk:** rustfmt the queries.rs import block after the getGroupInfos removal


### Continuous Integration

* accept platform-wallet-storage as a conventional-commit scope ([#4061](https://github.com/dashpay/platform/issues/4061))
* cover rs-unified-sdk-jni and missing contract crates in package filters
* don't block PRs when CodeRabbit is rate limited ([#4175](https://github.com/dashpay/platform/issues/4175))
* fix Swift warnings-as-errors build and harden coverage cleanup retry, closes [#4198](https://github.com/dashpay/platform/issues/4198)
* **kotlin-sdk:** dismiss the keyguard after enrolling the emulator lock screen
* **kotlin-sdk:** enroll a CI emulator lock screen so KEYS_ALIAS RSA key generation works
* **kotlin:** retry emulator credential unlock
* **kotlin:** retry emulator credential unlock
* **kotlin:** submit emulator unlock credential
* **kotlin:** submit emulator unlock credential
* register rs-unified-sdk-jni in the wallet fast-path closure + fix clippy lints, closes [#4003](https://github.com/dashpay/platform/issues/4003)
* **rust:** allow cold macOS workspace builds to finish
* **rust:** allow cold macOS workspace builds to finish
* **rust:** cover shielded tests in macOS timeout
* **rust:** cover shielded tests in macOS timeout
* **rust:** tolerate transient coverage cleanup races
* **rust:** tolerate transient coverage cleanup races
* scoped Rust test fast path for wallet-only PRs ([#4003](https://github.com/dashpay/platform/issues/4003))
* scoped Rust test fast path for wallet-only PRs ([#4003](https://github.com/dashpay/platform/issues/4003))
* **swift-sdk:** drop internal ticket reference from guard comments
* **swift-sdk:** trust thepastaclaw forks and harden runner policy checker
* unblock full-pipeline runs broken by untested paths, closes [#4171](https://github.com/dashpay/platform/issues/4171) [#4203](https://github.com/dashpay/platform/issues/4203)


### Code Refactoring

* **dapi:** make the last metrics label unbounded-proof
* **dpp:** pin denomination tests to explicit protocol versions
* **platform-wallet:** consolidate address-balance reconciliation into one guarded seam ([#3987](https://github.com/dashpay/platform/issues/3987))
* **platform-wallet:** dedup the account-address-pool snapshot loop, closes [#4041](https://github.com/dashpay/platform/issues/4041)
* **platform-wallet:** expose the new core TransactionBuilder API ([#3970](https://github.com/dashpay/platform/issues/3970))
* **qa-contract:** dissolve MultiWallet into tags for Kotlin test plan
* **sdk:** dedup shared wallet code + remove dead FFI/JNI chains ([#4106](https://github.com/dashpay/platform/issues/4106))
* **swift-sdk:** drop vestigial cross-network gate in wallet-deletion purge ([#4122](https://github.com/dashpay/platform/issues/4122))
* **swift-sdk:** promote the testnet faucet client into the SDK package ([#4098](https://github.com/dashpay/platform/issues/4098))
* **wallet:** drop the unused DapiBroadcaster overhaul
* **wallet:** follow dash-spv's removal of BIP61 reject handling, closes [rust-dashcore#913](https://github.com/dashpay/rust-dashcore/issues/913)
* **wallet:** pure-SPV broadcast path — no DAPI in the SPV wallet's send, closes [rust-dashcore#913](https://github.com/dashpay/rust-dashcore/issues/913)


### Documentation

* add DashPay contact request encryption guide ([#3787](https://github.com/dashpay/platform/issues/3787))
* add missing group-read rows (TOK-18/19/20) to Android test plan ([#4118](https://github.com/dashpay/platform/issues/4118))
* **book:** fix drifted source links in the count group-by chapters ([#3974](https://github.com/dashpay/platform/issues/3974))
* **dapi:** note latest-version bound is a pre-filter
* **dashpay:** add Kotlin migration spec + PARITY interim correction, closes [#3841](https://github.com/dashpay/platform/issues/3841) [#3841](https://github.com/dashpay/platform/issues/3841)
* **kotlin-sdk:** add KotlinExampleApp QA test plan and app code
* point README badges + NIGHTLY_STATUS at v4.1-dev (new default) ([#3983](https://github.com/dashpay/platform/issues/3983))
* **qa:** backfill multiwallet tag on CORE-14..23 in both test plans
* **qa:** port ADDR-07/08/09 to Android test plan + reconcile ADDR-03 numbering
* retire SYS-07 — redundant with cross-wallet receive tests ([#4114](https://github.com/dashpay/platform/issues/4114))
* **sdk:** port DashPay test cases (DP-01..11) to the Kotlin TEST_PLAN + fix PARITY count
* **sdk:** sync onWalletChangesetTransaction KDoc to the corrected descriptor
* **swift-example-app:** drop ADDR-05 from the QA test plan ([#3998](https://github.com/dashpay/platform/issues/3998))
* **swift-example-app:** fix ADDR-05 category reference ([#4001](https://github.com/dashpay/platform/issues/4001))
* **swift-example-app:** fix ADDR-05 category reference ([#4001](https://github.com/dashpay/platform/issues/4001))
* unify CLAUDE.md and AGENTS.md into one canonical agent-instructions file ([#4180](https://github.com/dashpay/platform/issues/4180))


### Tests

* **dapi:** enforce the metrics allowlist against the served protos
* **dapi:** make transactionsFilter bloom-filter test deterministic ([#4023](https://github.com/dashpay/platform/issues/4023))
* **drive-abci:** add token supply edge-case coverage ([#3849](https://github.com/dashpay/platform/issues/3849))
* **drive:** cover multi-range and paginated compacted balance proofs ([#4208](https://github.com/dashpay/platform/issues/4208))
* **drive:** cover shared-prefix aggregate index insertion ([#3961](https://github.com/dashpay/platform/issues/3961))
* **js-evo-sdk:** cover tiered token direct-purchase pricing in setPrice
* **js-evo-sdk:** fix unit test loader invocation
* **js-evo-sdk:** run unit tests with native type stripping instead of ts-node
* **kotlin:** pin the shielded-create payload codec boundary
* **platform-wallet:** fund wallet fixture with a chain-locked tx ([#4034](https://github.com/dashpay/platform/issues/4034))
* **platform-wallet:** pin Orchard key derivation to official ZIP-32 vectors ([#4032](https://github.com/dashpay/platform/issues/4032))
* **rs-sdk:** expect network floor in mock sdk seed test ([#3938](https://github.com/dashpay/platform/issues/3938))
* **sdk:** pin the identity/public-key restore round-trip
* **suite:** fail closed on cross-network verification and polish specs
* **suite:** wire an EvoSDK-backed platform proof verifier into client factories
* **swift-sdk:** first swift sdk integration tests with local network ([#3712](https://github.com/dashpay/platform/issues/3712))
* **swift-sdk:** port SpvRestart integration test off the removed ManagedCoreWallet.sendToAddresses, closes [#3970](https://github.com/dashpay/platform/issues/3970)
* **swift-sdk:** port the remaining two SpvRestart-sibling tests off ManagedCoreWallet.sendToAddresses, closes [#3970](https://github.com/dashpay/platform/issues/3970)
* **swift-sdk:** update address vectors to DIP-0018 after [#4021](https://github.com/dashpay/platform/issues/4021) ([#4024](https://github.com/dashpay/platform/issues/4024))


### Miscellaneous Chores

* add project-level release skill
* bump rust-dashcore to 647fa98 ([#4022](https://github.com/dashpay/platform/issues/4022))
* bump rust-dashcore to afcff156, export xpub via ExtendedPubKeySigner ([#3976](https://github.com/dashpay/platform/issues/3976))
* **kotlin-example-app:** add emulator-control skill for KotlinExampleApp ([#4174](https://github.com/dashpay/platform/issues/4174))
* **kotlin-sdk:** apply rustfmt to rs-unified-sdk-jni, closes [#4192](https://github.com/dashpay/platform/issues/4192)
* **kotlin-sdk:** free more runner disk before the emulator to fix "no space left" flakes
* **kotlin-sdk:** tx-decode follow-up — blob hardening, net_from_ord hoist, prevVout docs ([#4187](https://github.com/dashpay/platform/issues/4187) review) ([#4192](https://github.com/dashpay/platform/issues/4192))
* pin rust-dashcore to the [rust-dashcore#913](https://github.com/dashpay/rust-dashcore/issues/913) merge commit
* remove ignored cargo build config
* **sdk:** drop the empty companion object left by the Cleaner migration
* **swift-sdk:** reduce swift-sdk test time in CI ([#3869](https://github.com/dashpay/platform/issues/3869))
* **swift-sdk:** script to get spv stortage from iOS sim ([#3950](https://github.com/dashpay/platform/issues/3950))
* update rust-dashcore to 1ee1c94 ([#4094](https://github.com/dashpay/platform/issues/4094))

## [4.0.0](https://github.com/dashpay/platform/compare/v4.0.0-rc.2...v4.0.0) (2026-07-01)


### ⚠ BREAKING CHANGES

* **dpp:** make platform/orchard address decoders network-agnostic (#3781)

### Features

* **contract:** on-chain QA framework storage layer (testCase + testRun) ([#3910](https://github.com/dashpay/platform/issues/3910))
* **dpp:** add getters and setters for new shielded state transitions ([#3879](https://github.com/dashpay/platform/issues/3879))
* **platform-wallet:** external signable wallets ([#3639](https://github.com/dashpay/platform/issues/3639))
* **platform:** shielded transaction history ([#3870](https://github.com/dashpay/platform/issues/3870))
* record DAPI address ban reason and expose via platform-wallet FFI ([#3890](https://github.com/dashpay/platform/issues/3890))
* **rs-sdk-ffi:** add masternode contested-resource vote broadcast (FFI + Swift UI) ([#3883](https://github.com/dashpay/platform/issues/3883))
* **rs-sdk-ffi:** expose dash_sdk_signer_can_sign for signer-delegated key preflight ([#3924](https://github.com/dashpay/platform/issues/3924))
* **sdk:** implement document sum/average aggregation FFI (DOC-13/14) ([#3935](https://github.com/dashpay/platform/issues/3935))
* **swift-example-app:** add identity→identity Transfer Credits production UI ([#3891](https://github.com/dashpay/platform/issues/3891))
* **swift-example-app:** add production Disable Key action with safety gates (ID-12) ([#3918](https://github.com/dashpay/platform/issues/3918))
* **swift-example-app:** document count aggregation view (DOC-10/11/12) ([#3926](https://github.com/dashpay/platform/issues/3926))
* **swift-example-app:** document sum/average aggregation view (DOC-13/14) ([#3942](https://github.com/dashpay/platform/issues/3942))
* **swift-example-app:** GroveDB path elements diagnostic view (SYS-06) ([#3931](https://github.com/dashpay/platform/issues/3931))
* **swift-example-app:** multi-recipient Core L1 send (CORE-10) ([#3904](https://github.com/dashpay/platform/issues/3904))
* **swift-example-app:** one-tap testnet DASH faucet button ([#3905](https://github.com/dashpay/platform/issues/3905))
* **swift-example-app:** production document replace/delete/transfer/price/purchase UI (DOC-03..07) ([#3945](https://github.com/dashpay/platform/issues/3945))
* **swift-example-app:** production Withdraw Credits UI on identity detail (ID-10) ([#3906](https://github.com/dashpay/platform/issues/3906))
* **swift-example-app:** rebrand to "Dash Developer Pro" for TestFlight ([#3952](https://github.com/dashpay/platform/issues/3952))
* **swift-example-app:** send from Core balance to a shielded recipient ([#3885](https://github.com/dashpay/platform/issues/3885))
* **swift-sdk:** add BIP39 word-validation helpers for the recover flow ([#3842](https://github.com/dashpay/platform/issues/3842))
* **swift-sdk:** add identityUpdate handler to the generic transition builder ([#3880](https://github.com/dashpay/platform/issues/3880))
* **swift-sdk:** add production create-document flow via platform-wallet FFI ([#3908](https://github.com/dashpay/platform/issues/3908))
* **swift-sdk:** log tokio runtime metrics from rs-sdk-ffi and platform-wallet-ffi ([#3901](https://github.com/dashpay/platform/issues/3901))
* **swift-sdk:** wire data contract update through the platform-wallet path ([#3882](https://github.com/dashpay/platform/issues/3882))


### Bug Fixes

* **dashmate:** use active_dkgs for safe DKG stop ([#3941](https://github.com/dashpay/platform/issues/3941))
* **dpp:** enforce byte array encoding stability in data contract updates ([#3868](https://github.com/dashpay/platform/issues/3868))
* **dpp:** guard index property parsing against empty/oversized maps to prevent check_tx panic ([#3866](https://github.com/dashpay/platform/issues/3866))
* **dpp:** make platform/orchard address decoders network-agnostic ([#3781](https://github.com/dashpay/platform/issues/3781))
* **dpp:** reject empty token pricing schedules to prevent a direct-purchase chain halt ([#3865](https://github.com/dashpay/platform/issues/3865))
* **drive:** return empty history for contracts that don't keep history ([#3884](https://github.com/dashpay/platform/issues/3884))
* **drive:** unify shielded per-action processing fee across all protocol versions ([#3877](https://github.com/dashpay/platform/issues/3877))
* **platform-wallet:** add TRANSFER key to default identity registration key set ([#3894](https://github.com/dashpay/platform/issues/3894))
* **platform-wallet:** apply disabled-key flags to local cache after identity update ([#3915](https://github.com/dashpay/platform/issues/3915))
* **platform-wallet:** build data contract config at the protocol-required version ([#3881](https://github.com/dashpay/platform/issues/3881))
* **platform-wallet:** free SPV data dir on stop ([#3811](https://github.com/dashpay/platform/issues/3811))
* **platform-wallet:** spv error propagation ([#3810](https://github.com/dashpay/platform/issues/3810))
* **rs-sdk-ffi:** classify DAPI transport and timeout errors instead of internalError ([#3916](https://github.com/dashpay/platform/issues/3916))
* **rs-sdk-ffi:** don't export ContestedResourceVoteChoiceFFI to the C header ([#3892](https://github.com/dashpay/platform/issues/3892))
* **rs-sdk-ffi:** run blocking FFI calls on a large stack to prevent proof-verification stack overflow ([#3896](https://github.com/dashpay/platform/issues/3896))
* **rs-sdk-ffi:** stop labeling unclassified SDK errors as "failed to fetch balances" ([#3878](https://github.com/dashpay/platform/issues/3878))
* **rs-sdk:** case-insensitive .dash suffix in DPNS name resolution ([#3914](https://github.com/dashpay/platform/issues/3914))
* **sdk:** ban node and retry elsewhere on UNIMPLEMENTED responses ([#3875](https://github.com/dashpay/platform/issues/3875))
* **sdk:** ban rate-limited node for Envoy-advertised reset window ([#3951](https://github.com/dashpay/platform/issues/3951))
* **sdk:** default initial protocol version to 10 when unpinned (upgrade-safe ratchet floor) ([#3809](https://github.com/dashpay/platform/issues/3809))
* **sdk:** default to latest protocol version instead of pinning testnet to v1 ([#3937](https://github.com/dashpay/platform/issues/3937))
* **sdk:** refresh protocol version via a proven query, not unproved getStatus ([#3893](https://github.com/dashpay/platform/issues/3893))
* **sdk:** refresh SDK protocol version to the network's on startup and network switch ([#3886](https://github.com/dashpay/platform/issues/3886))
* **sdk:** verify quorum signature on broadcast wait-path before trusting metadata ([#3872](https://github.com/dashpay/platform/issues/3872))
* **swift-example-app:** expose Picker options to UI automation ([#3903](https://github.com/dashpay/platform/issues/3903))
* **swift-example-app:** fix compile timeout on on newest xcode versions ([#3899](https://github.com/dashpay/platform/issues/3899))
* **swift-example-app:** harden Withdraw Credits amount + address validation ([#3907](https://github.com/dashpay/platform/issues/3907))
* **swift-example-app:** load document price on Purchase sheet appear ([#3947](https://github.com/dashpay/platform/issues/3947))
* **swift-example-app:** prevent UInt64.max overflow crash in TransferCreditsView amount parsing ([#3909](https://github.com/dashpay/platform/issues/3909))
* **swift-example-app:** show success title on DPNS registration alert ([#3873](https://github.com/dashpay/platform/issues/3873))
* **swift-sdk:** keychain priv key storage indexes by label ([#3946](https://github.com/dashpay/platform/issues/3946))
* **swift-sdk:** make Document transition contract/type pickers idb-drivable ([#3921](https://github.com/dashpay/platform/issues/3921))
* **swift-sdk:** parse getIdentitiesTokenBalances NSNumber result ([#3943](https://github.com/dashpay/platform/issues/3943))
* **swift-sdk:** seed testnet at the per-network protocol-version floor instead of pinning PV11 ([#3944](https://github.com/dashpay/platform/issues/3944))
* **swift-sdk:** sign document state transitions with an available AUTHENTICATION key ([#3922](https://github.com/dashpay/platform/issues/3922))
* **swift-sdk:** stamp firstSeen at insert for unconfirmed transactions ([#3874](https://github.com/dashpay/platform/issues/3874))
* **swift-sdk:** update token balance from the transfer/burn proof result (MW-02) ([#3934](https://github.com/dashpay/platform/issues/3934))
* **wasm-sdk:** preserve user-supplied addresses in withTrustedContext ([#3912](https://github.com/dashpay/platform/issues/3912))


### Continuous Integration

* **swift-sdk:** remove swift-sdk artifact upload ([#3778](https://github.com/dashpay/platform/issues/3778))


### Documentation

* **swift-example-app:** add MANUAL tier to iOS test plan ([#3895](https://github.com/dashpay/platform/issues/3895))
* **swift-example-app:** add multi-wallet test cases (Core + Platform) to iOS test plan ([#3888](https://github.com/dashpay/platform/issues/3888))
* **swift-example-app:** add tiered iOS feature test plan ([#3887](https://github.com/dashpay/platform/issues/3887))
* **swift-example-app:** link on-chain QA status dashboard from TEST_PLAN ([#3911](https://github.com/dashpay/platform/issues/3911))
* **swift-example-app:** re-tier + re-status SH-11 (shielded identity create) → Common, ✅ in-UI ([#3913](https://github.com/dashpay/platform/issues/3913))
* **swift-example-app:** remove CORE-11/12/13 (not-implemented Core actions) from test plan ([#3919](https://github.com/dashpay/platform/issues/3919))
* **swift-example-app:** remove TOK-17 (calculate token ID utility) from test plan ([#3925](https://github.com/dashpay/platform/issues/3925))
* **swift-example-app:** retire GRP-04 from the QA catalog ([#3932](https://github.com/dashpay/platform/issues/3932))
* **swift-example-app:** retire stub builder rows VOTE-07/ID-13 ([#3929](https://github.com/dashpay/platform/issues/3929))


### Tests

* **drive-abci:** fix direct-selling tests broken by empty-schedule validation ([#3876](https://github.com/dashpay/platform/issues/3876))
* **swift-example-app:** cover KeyDisableGate and correct consensus-framing comments ([#3920](https://github.com/dashpay/platform/issues/3920))
* **swift-sdk:** move swift-sdk unit test defined in the example app to the swift-sdk unit tests suite ([#3917](https://github.com/dashpay/platform/issues/3917))
* **wasm-sdk:** raise withdrawal test amount to the v12 minimum ([#3867](https://github.com/dashpay/platform/issues/3867))


### Code Refactoring

* **sdk:** per-network protocol-version floor + version_pinned unification ([#3900](https://github.com/dashpay/platform/issues/3900))


### Build System

* **dashmate:** update Tenderdash image to v1.6.0 ([#3940](https://github.com/dashpay/platform/issues/3940))
* update grovedb dependency from git revision to v5.0.0 tag ([#3971](https://github.com/dashpay/platform/issues/3971))
* update rust-dashcore to v0.44.0 ([#3973](https://github.com/dashpay/platform/issues/3973))


### Miscellaneous Chores

* bump rust-dashcore to rev without Address::network() method ([#3788](https://github.com/dashpay/platform/issues/3788))
* retarget in-repo branch references from v3.1-dev to v4.0-dev ([#3972](https://github.com/dashpay/platform/issues/3972))

## [4.0.0-rc.2](https://github.com/dashpay/platform/compare/v4.0.0-rc.1...v4.0.0-rc.2) (2026-06-12)


### Features

* **swift-sdk:** seed shielded pool notes from the example app ([#3858](https://github.com/dashpay/platform/issues/3858))


### Bug Fixes

* **dpp:** harden nested document-property position parsing ([#3857](https://github.com/dashpay/platform/issues/3857))
* **drive:** return error instead of panicking on empty SetPrices direct purchase ([#3856](https://github.com/dashpay/platform/issues/3856))
* **drive:** verify identity-create-from-shielded-pool results without unbounded terminal-key queries ([#3859](https://github.com/dashpay/platform/issues/3859))
* **platform-wallet:** keep note reservations on ambiguous shielded spend confirmation failures ([#3863](https://github.com/dashpay/platform/issues/3863))
* **platform:** derive identity-rescan keys through the wallet signer ([#3860](https://github.com/dashpay/platform/issues/3860))
* **platform:** load identity by index through the wallet signer ([#3861](https://github.com/dashpay/platform/issues/3861))
* **platform:** zero cached platform-address balances absent from state ([#3855](https://github.com/dashpay/platform/issues/3855))
* **swift-sdk:** attribute shielded registration errors to the right step and keep unconfirmed broadcasts safe ([#3862](https://github.com/dashpay/platform/issues/3862))
* **swift-sdk:** fixed mempool tx categorization after restart ([#3777](https://github.com/dashpay/platform/issues/3777))
* **swift-sdk:** freeze failed registration step at the failure instant ([#3854](https://github.com/dashpay/platform/issues/3854))
* **wasm-sdk:** label getTokenContractInfo parameter as tokenId, not contractId ([#3851](https://github.com/dashpay/platform/issues/3851))


### Miscellaneous Chores

* **swift-sdk:** reduced swift-sdk static library size using Cargo profiles ([#3837](https://github.com/dashpay/platform/issues/3837))


### Continuous Integration

* retry docker metadata step on transient github api failures ([#3847](https://github.com/dashpay/platform/issues/3847))

## [4.0.0-rc.1](https://github.com/dashpay/platform/compare/v4.0.0-beta.4...v4.0.0-rc.1) (2026-06-10)


### Features

* **platform:** send memos with shielded transfers ([#3836](https://github.com/dashpay/platform/issues/3836))
* **swift-sdk:** fund identity creation from shielded balance ([#3838](https://github.com/dashpay/platform/issues/3838))
* **swift-sdk:** scan recipient address QR codes on the Send screen ([#3835](https://github.com/dashpay/platform/issues/3835))
* **swift-sdk:** select Core or Platform source when shielding in example app ([#3830](https://github.com/dashpay/platform/issues/3830))


### Bug Fixes

* build shielded FFI load path under --all-features ([#3826](https://github.com/dashpay/platform/issues/3826))
* **drive-abci:** make shielded snapshot ingest idempotent across InitChain retries ([#3824](https://github.com/dashpay/platform/issues/3824))
* **platform-wallet:** align shielded_sync example with filler-only seeded notes ([#3832](https://github.com/dashpay/platform/issues/3832))
* **platform:** derive shielded identity-create id from the padded bundle's published nullifiers ([#3843](https://github.com/dashpay/platform/issues/3843))
* **platform:** encrypt shielded outputs to the sender's outgoing viewing key ([#3839](https://github.com/dashpay/platform/issues/3839))
* **platform:** tag restored wallet addresses with the wallet's network ([#3834](https://github.com/dashpay/platform/issues/3834))
* **sdk:** wallet-flow network fixes for SwiftExampleApp ([#3772](https://github.com/dashpay/platform/issues/3772))
* **swift-sdk:** example app ask if the spv is running directly instead of using the sync state ([#3821](https://github.com/dashpay/platform/issues/3821))
* **swift-sdk:** show shielded funding steps and real fee estimate ([#3845](https://github.com/dashpay/platform/issues/3845))


### Code Refactoring

* remove orphaned shielded nullifier-changes subsystem ([#3823](https://github.com/dashpay/platform/issues/3823))


### Miscellaneous Chores

* expand code ownership for SDK, wallet stack, proof verifier and wasm-dpp2 ([#3840](https://github.com/dashpay/platform/issues/3840))


### Tests

* **dpp:** require shielded-client in shield-from-asset-lock signing tests ([#3827](https://github.com/dashpay/platform/issues/3827))
* **drive-abci:** assert check_tx never mutates committed grovedb state ([#3844](https://github.com/dashpay/platform/issues/3844))
* **platform-wallet:** fix stale balance assertion in paloma shielded sync example ([#3831](https://github.com/dashpay/platform/issues/3831))
* source shielded minimum-fee from compute_minimum_shielded_fee ([#3829](https://github.com/dashpay/platform/issues/3829))
* **wallet-storage:** add missing wallet_group_id in test initializers ([#3833](https://github.com/dashpay/platform/issues/3833))

## [4.0.0-beta.4](///compare/v4.0.0-beta.3...v4.0.0-beta.4) (2026-06-09)


### ⚠ BREAKING CHANGES

* **platform-wallet:** add platform-wallet-storage crate (sqlite persister) (#3625)

### Features

* add IdentityCreateFromShieldedPool state transition (shielded-pool-funded identity creation) ([#3816](undefined/undefined/undefined/issues/3816))
* **drive:** shielded fees for Shield/ShieldFromAssetLock + shield credit conservation ([#3793](undefined/undefined/undefined/issues/3793))
* **platform-wallet:** add platform-wallet-storage crate (sqlite persister) ([#3625](undefined/undefined/undefined/issues/3625))
* shielded scan-based spend detection and OVK outgoing-note history ([#3819](undefined/undefined/undefined/issues/3819))
* **swift-sdk:** iOS simluator writes logs to disk ([#3785](undefined/undefined/undefined/issues/3785))


### Bug Fixes

* **dpp:** return error instead of panicking on storage-fee refund div-by-zero ([#3799](undefined/undefined/undefined/issues/3799))
* **drive:** charge fees for unshield and shielded withdrawal ([#3800](undefined/undefined/undefined/issues/3800))
* **drive:** correct fee/credit accounting on the address-funding asset-lock penalty path ([#3818](undefined/undefined/undefined/issues/3818))
* **drive:** strict merged-query verification for unshield & shielded withdrawal proofs ([#3814](undefined/undefined/undefined/issues/3814))
* **drive:** unify shielded pool genesis/upgrade construction to prevent state divergence ([#3801](undefined/undefined/undefined/issues/3801))
* **platform-wallet:** zeroize private keys when freeing preview rows ([#3797](undefined/undefined/undefined/issues/3797))
* **rs-sdk-ffi:** shrink signature allocation to len before leaking (capacity UB) ([#3798](undefined/undefined/undefined/issues/3798))


### Miscellaneous Chores

* tidy follow-ups from the shielded withdrawal fee review ([#3802](undefined/undefined/undefined/issues/3802))

## [4.0.0-beta.3](https://github.com/dashpay/platform/compare/v4.0.0-beta.2...v4.0.0-beta.3) (2026-06-04)


### Bug Fixes

* **dashmate:** bump Envoy gateway to 1.35.11 for HTTP/2 DoS (CVE-2026-47774) ([#3794](https://github.com/dashpay/platform/issues/3794))
* grovedb incompatibilty issues ([#3789](https://github.com/dashpay/platform/issues/3789))


## [4.0.0-beta.2](https://github.com/dashpay/platform/compare/v4.0.0-beta.1...v4.0.0-beta.2) (2026-06-02)


### Bug Fixes

* **platform-version:** gate shielded-pool block methods to protocol v12 ([#3782](https://github.com/dashpay/platform/issues/3782))

## [4.0.0-beta.1](///compare/v3.1.0-dev.8...v4.0.0-beta.1) (2026-06-02)


### Features

* add register-contract script to rs-scripts ([#3744](undefined/undefined/undefined/issues/3744))
* **dashmate:** configure docker build args via config ([#3764](undefined/undefined/undefined/issues/3764))
* **drive-abci:** gate shielded-pool seeding behind `shielded_test_data` feature ([#3774](undefined/undefined/undefined/issues/3774))
* **drive:** add document history retrieval ([#3725](undefined/undefined/undefined/issues/3725))
* **platform:** add GetShieldedNotesCount query for sync progress ([#3769](undefined/undefined/undefined/issues/3769))
* seed Orchard shielded pool at genesis with fast, observable sync ([#3732](undefined/undefined/undefined/issues/3732))


### Bug Fixes

* **dashmate:** prevent orphaned verification container blocking SSL renewal ([#3162](undefined/undefined/undefined/issues/3162))
* **dpp:** block pre-programmed distribution changes on token update ([#3461](undefined/undefined/undefined/issues/3461))
* **drive:** consolidate historical contract proof verification retry logic ([#3165](undefined/undefined/undefined/issues/3165))
* **platform-wallet:** fix spv client deadlocking himself when trying to stop ([#3742](undefined/undefined/undefined/issues/3742))
* **platform-wallet:** satisfy accessors clippy lints ([#3596](undefined/undefined/undefined/issues/3596))


### Tests

* **swift-sdk:** swift-sdk test updated and added to CI ([#3479](undefined/undefined/undefined/issues/3479))
* **wasm-sdk:** fix flaky tokenPaymentInfo document balance assertions ([#3771](undefined/undefined/undefined/issues/3771))

## [3.1.0-dev.8](https://github.com/dashpay/platform/compare/v3.1.0-dev.7...v3.1.0-dev.8) (2026-05-28)


### Features

* enable DashPay iOS flow + key health tooling ([#3765](https://github.com/dashpay/platform/issues/3765))
* **sdk:** expose document count/sum/average aggregates in js-evo-sdk facade ([#3767](https://github.com/dashpay/platform/issues/3767))
* shielded funding from asset-lock proofs ([#3753](https://github.com/dashpay/platform/issues/3753))
* **swift-example-app:** enable SPV on devnet ([#3763](https://github.com/dashpay/platform/issues/3763))
* **swift-sdk,rs-sdk-ffi:** wire devnet SDK config + auto-discover masternodes ([#3755](https://github.com/dashpay/platform/issues/3755))


### Bug Fixes

* **platform-wallet:** auto_select_inputs honors Σ inputs == Σ outputs ([#3554](https://github.com/dashpay/platform/issues/3554))


### Miscellaneous Chores

* bump rust-dashcore to eb889af ([#3762](https://github.com/dashpay/platform/issues/3762))


### Tests

* **dpp,drive-abci:** cover transfer-key signing rules for token transfers ([#3766](https://github.com/dashpay/platform/issues/3766))
* **rs-sdk:** relocate DPNS network tests from src/ to tests/ ([#3721](https://github.com/dashpay/platform/issues/3721))

## [3.1.0-dev.7](https://github.com/dashpay/platform/compare/v3.1.0-dev.6...v3.1.0-dev.7) (2026-05-27)


### ⚠ BREAKING CHANGES

* **drive-abci, sdk:** allow shielded-notes queries to span 4 MMR chunks (#3756)

### Features

* **drive-abci, sdk:** allow shielded-notes queries to span 4 MMR chunks ([#3756](https://github.com/dashpay/platform/issues/3756))
* platform-address funding from asset-lock proofs ([#3671](https://github.com/dashpay/platform/issues/3671))
* **rs-platform-wallet-ffi:** expose devnet name and LLMQ_DEVNET override in spv_start ([#3758](https://github.com/dashpay/platform/issues/3758))
* **rs-sdk-ffi:** expose optional platform_version in DashSDKConfig ([#3751](https://github.com/dashpay/platform/issues/3751))
* **wasm-sdk:** first-class devnet support with trusted-context prefetch ([#3748](https://github.com/dashpay/platform/issues/3748))


### Bug Fixes

* **swift-sdk:** sort transfer outputs lexicographically before ReduceOutput ([#3752](https://github.com/dashpay/platform/issues/3752))


### Miscellaneous Chores

* bump rust-dashcore to 58d61ea ([#3757](https://github.com/dashpay/platform/issues/3757))
* **dapi-grpc:** regenerate obj-c client for SUM/AVG doc updates ([#3759](https://github.com/dashpay/platform/issues/3759))

## [3.1.0-dev.6](https://github.com/dashpay/platform/compare/v3.1.0-dev.5...v3.1.0-dev.6) (2026-05-26)


### Features

* **platform-wallet:** expose sync_watermark() on PlatformAddressWallet ([#3723](https://github.com/dashpay/platform/issues/3723))
* **platform-wallet:** IdentityManager::identity_ids + FFI no-selectable-inputs error mapping ([#3651](https://github.com/dashpay/platform/issues/3651))
* **platform-wallet:** serde support ([#3637](https://github.com/dashpay/platform/issues/3637))
* **swift-sdk,platform-wallet:** wire shielded send end-to-end (all 4 transitions) ([#3603](https://github.com/dashpay/platform/issues/3603))


### Bug Fixes

* **drive-abci:** bill batch transformer drive reads ([#3670](https://github.com/dashpay/platform/issues/3670))
* **drive-abci:** correct DECRYPTION bounds branch + bill grovedb reads in bounds validation ([#3697](https://github.com/dashpay/platform/issues/3697))
* **platform-wallet:** fail-closed on registration persist error (Found-017) [backport] ([#3659](https://github.com/dashpay/platform/issues/3659))
* **platform-wallet:** spv client deadlocking when sending a tx ([#3730](https://github.com/dashpay/platform/issues/3730))
* **sdk:** forward wasm grpc-web trailers to tonic ([#3726](https://github.com/dashpay/platform/issues/3726))
* **sdk:** sdk emits incompatible getDocuments wire against pre-v3.1 networks (QueryContext approach) ([#3711](https://github.com/dashpay/platform/issues/3711))
* **wasm-sdk:** support binary grove path elements ([#3657](https://github.com/dashpay/platform/issues/3657))


### Miscellaneous Chores

* bump rust-dashcore to rev f569e7b7b99dfe589c41f9ba7d36fbbe6805acdc ([#3729](https://github.com/dashpay/platform/issues/3729))
* **dapi-client,dapi-grpc:** cleanup — drop unused deps, inline winston/fetch/promisify shims ([#3679](https://github.com/dashpay/platform/issues/3679))

## [3.1.0-dev.5](https://github.com/dashpay/platform/compare/v3.1.0-dev.4...v3.1.0-dev.5) (2026-05-21)


### Bug Fixes

* **dpp:** remove erroneous keywords field from document-meta schema and fix contract keywords docs ([#3471](https://github.com/dashpay/platform/issues/3471))


### Continuous Integration

* **release:** fix dashmate deb pack by configuring oclif targets ([#3713](https://github.com/dashpay/platform/issues/3713))


### Documentation

* **sdk:** update js-evo-sdk README for configuration, shielded facade, and wallet utilities ([#3701](https://github.com/dashpay/platform/issues/3701))

## [3.1.0-dev.4](https://github.com/dashpay/platform/compare/v3.1.0-dev.3...v3.1.0-dev.4) (2026-05-20)


### ⚠ BREAKING CHANGES

* **platform-wallet:** add birth_height_override to wallet creation API (#3636)

### Features

* **platform-wallet:** add birth_height_override to wallet creation API ([#3636](https://github.com/dashpay/platform/issues/3636))


### Continuous Integration

* **release:** drop 32-bit dashmate pack targets unsupported by Node 24 ([#3709](https://github.com/dashpay/platform/issues/3709))

## [3.1.0-dev.3](https://github.com/dashpay/platform/compare/v3.1.0-dev.2...v3.1.0-dev.3) (2026-05-20)


### Bug Fixes

* **swift-sdk:** drop transitive keypath from PersistentTxo unspent prefetch ([#3691](https://github.com/dashpay/platform/issues/3691))

## [3.1.0-dev.2](https://github.com/dashpay/platform/compare/v3.1.0-dev.1...v3.1.0-dev.2) (2026-05-20)


### ⚠ BREAKING CHANGES

* **platform:** verifiable, bounded count queries on a unified endpoint (#3623)
* **wasm-sdk:** add shielded pool WASM bindings and query methods (#3235)
* **platform:** derive+persist + sign-with-resolver Rust-owned pipelines (#3542)
* **platform:** external KeychainSigner end-to-end + identity flow sweep (#3541)
* **platform:** iOS late-April pass + IdentityManager restructure (#3538)
* **dpp:** convert Signer trait to async (#3492)
* **swift-sdk:** realign Swift FFI shims with current rust-dashcore + platform-wallet-ffi
* remove unused feature-flags system contract (#3522)
* **dpp:** enforce bincode byte-budget limit on enum deserialization (#3223)
* **dpp:** cleanup and unify JSON/Object conversion (#3167)

### Features

* add rs-scripts crate with decode-document CLI tool ([#3391](https://github.com/dashpay/platform/issues/3391))
* **address-sync:** two-phase commit via sync_finished + per_wallet_in_sync
* **dashmate:** default-on the BIP158 compact-filter index across all presets ([#3587](https://github.com/dashpay/platform/issues/3587))
* **dpp:** add documents_countable to DocumentTypeV2 for O(1) total document counts ([#3457](https://github.com/dashpay/platform/issues/3457))
* **dpp:** add max_asset_lock_transaction_inputs limit to prevent stuck funds ([#3491](https://github.com/dashpay/platform/issues/3491))
* **dpp:** convert Signer trait to async ([#3492](https://github.com/dashpay/platform/issues/3492))
* **dpp:** shielded state transitions and Orchard bundle types (Medusa) ([#3177](https://github.com/dashpay/platform/issues/3177))
* **drive-abci:** add shielded pool drive-abci integration (medusa part 3) ([#3220](https://github.com/dashpay/platform/issues/3220))
* **drive:** add paginated fetch_contract_ids and fetch_contracts ([#3480](https://github.com/dashpay/platform/issues/3480))
* **drive:** add shielded pool storage, actions, and verification (Medusa part 2) ([#3198](https://github.com/dashpay/platform/issues/3198))
* **drive:** allow deleting non-empty trees in targeted batch operations ([#3210](https://github.com/dashpay/platform/issues/3210))
* **drive:** bump grovedb and expose key_exists_as_boundary for pagination ([#3373](https://github.com/dashpay/platform/issues/3373))
* **drive:** document sum + average proof primitives, with SDK fan-out scaffolding and reproducible benchmarks ([#3661](https://github.com/dashpay/platform/issues/3661))
* **drive:** expand count-index group-by carrier shapes (G1a/G1b/G8a-c) ([#3652](https://github.com/dashpay/platform/issues/3652))
* identity registration with asset-lock proofs ([#3634](https://github.com/dashpay/platform/issues/3634))
* platform wallet ([#2855](https://github.com/dashpay/platform/issues/2855))
* **platform-wallet-ffi:** add WalletChangeSet FFI types and persistence callback
* **platform-wallet-ffi:** complete AssetLockManager FFI
* **platform-wallet-ffi:** expose identity revision + public keys
* **platform-wallet-ffi:** forward identity + key changesets to Swift
* **platform-wallet-ffi:** full DashPay contact + payment FFI surface
* **platform-wallet-ffi:** render PlatformPayment addresses as bech32m
* **platform-wallet-ffi:** stateless derive_ext_priv_key_from_mnemonic
* **platform-wallet-ffi:** thread mnemonic through identity registration
* **platform-wallet-ffi:** wire DIP-13 identity auth account variants, closes [rust-dashcore#672](https://github.com/dashpay/rust-dashcore/issues/672)
* **platform-wallet,swift-sdk:** query per-account balances via FFI ([#3572](https://github.com/dashpay/platform/issues/3572))
* **platform-wallet:** add *_with_signer methods to IdentityWallet
* **platform-wallet:** add *_with_signer methods to TokenWallet
* **platform-wallet:** add blocking accessors to AssetLockManager
* **platform-wallet:** add blocking address derivation methods
* **platform-wallet:** add blocking_wallet_info() for sync contexts
* **platform-wallet:** add blocking_wallet() for sync key derivation
* **platform-wallet:** add contact account to both Wallet and ManagedWalletInfo
* **platform-wallet:** add dashpay_profiles + dashpay_payments_overlay to PlatformWalletChangeSet
* **platform-wallet:** add funded_register/top_up_identity methods with IdentityFunding
* **platform-wallet:** add gap-limit identity discovery scan
* **platform-wallet:** add identity_manager_mut + try_identity_manager_mut
* **platform-wallet:** add IS-lock to ChainLock proof fallback
* **platform-wallet:** add list_tracked_locks accessors to AssetLockManager
* **platform-wallet:** add load_persisted_state() to PlatformWallet
* **platform-wallet:** add missing token operations — destroy, pause, resume, update_config
* **platform-wallet:** add next_unused_receive_address to PlatformAddressWallet
* **platform-wallet:** add persistence module — ChangeSet types, Merge trait, WalletPersistence trait
* **platform-wallet:** add PR-29 to PLAN, update event_forwarder docs, cleanup
* **platform-wallet:** add register/top-up with_signer + identity_manager accessor
* **platform-wallet:** add SPV lifecycle controls and progress FFI
* **platform-wallet:** add stage field and persist API to PlatformWallet
* **platform-wallet:** add state_mut_blocking for sync callers (Phase 9b-3)
* **platform-wallet:** add sync_progress, clear_storage, update_config to SpvRuntime
* **platform-wallet:** add try_state non-blocking lock variants
* **platform-wallet:** add try_wallet_info() non-blocking accessors
* **platform-wallet:** add WalletBalance — lock-free atomic balance
* **platform-wallet:** asset lock lifecycle, IdentityFunding, SPV finality wiring
* **platform-wallet:** AssetLockManager subscribes to SPV events, delete DAPI streaming
* **platform-wallet:** auto-refresh WalletBalance via WalletInfoWriteGuard
* **platform-wallet:** bridge core WalletPersistence to platform persister
* **platform-wallet:** changeset persistence for tracked asset locks
* **platform-wallet:** clean asset lock types — TrackedAssetLock + AssetLockStatus
* **platform-wallet:** complete iOS Rust-owned address sync transition
* **platform-wallet:** contact address indices via ContactChangeSet (Phase 9b-3a)
* **platform-wallet:** contested DPNS name cache + wallet-path read migration
* **platform-wallet:** CoreWallet FFI, mnemonic creation, TransactionBuilder integration ([#3489](https://github.com/dashpay/platform/issues/3489))
* **platform-wallet:** DashPay payment history via IdentityChangeSet (Phase 9b-2a)
* **platform-wallet:** DashPay profiles via IdentityChangeSet (Phase 9b-1a)
* **platform-wallet:** DashpayAddressMatch lookup API (Phase 9b-4a)
* **platform-wallet:** DashPayWallet owns profile sync + create/update (Phase 1)
* **platform-wallet:** DashPayWallet records incoming payments internally (Phase 2a)
* **platform-wallet:** DashPayWallet send_payment + external contact account (Phase 2b)
* **platform-wallet:** derive Clone on IdentityFunding for evo-tool retry path
* **platform-wallet:** DPNS name cache sync + read via platform-wallet
* **platform-wallet:** expose changeset types for caller-level staging
* **platform-wallet:** expose wallet_info/wallet lock accessors on CoreWallet
* **platform-wallet:** extend send_contact_request with label + auto-accept
* **platform-wallet:** extend SpvRuntime with broadcast, quorum, run
* **platform-wallet:** extract AssetLockManager, share across sub-wallets
* **platform-wallet:** incremental SwiftData persistence for address balances
* **platform-wallet:** Item 8.1a — AssetLockManager forwards changesets to persister
* **platform-wallet:** key TrackedAssetLock by OutPoint, add resumable asset lock operations
* **platform-wallet:** multi-wallet BLAST sync via unified provider
* **platform-wallet:** owned/watched identity split + ManagedIdentitySigner
* **platform-wallet:** persist and restore sync state for incremental BLAST sync
* **platform-wallet:** PlatformWallet::load_and_apply_persisted
* **platform-wallet:** PlatformWalletInfo::apply_changeset (Phase 9a-3)
* **platform-wallet:** PR-1 scaffold — PlatformWallet, Manager, sub-wallets
* **platform-wallet:** PR-10 — enrich ManagedIdentity with KeyStorage, status, DPNS
* **platform-wallet:** PR-11 — asset lock lifecycle + multi-mode funding
* **platform-wallet:** PR-12 — DashPay DIP-14/15 crypto + payment addresses
* **platform-wallet:** PR-14 — DashPay + Identity protocol completeness
* **platform-wallet:** PR-15 — shielded pool with storage abstraction
* **platform-wallet:** PR-16 — AssetLockFinalityEvent tracking
* **platform-wallet:** PR-17 — adopt dashcore asset lock builder, closes [rust-dashcore#604](https://github.com/dashpay/rust-dashcore/issues/604)
* **platform-wallet:** PR-2 — per-address data, signing, asset locks
* **platform-wallet:** PR-2 — signing, per-address data, asset locks, payments
* **platform-wallet:** PR-3 — IdentityWallet with real SDK calls + IdentitySigner
* **platform-wallet:** PR-3/PR-4 — IdentityWallet, DashPayWallet, review fixes
* **platform-wallet:** PR-5 — PlatformAddressWallet DIP-17 operations
* **platform-wallet:** PR-6 — SPV lifecycle, TransactionStatus, EventHandler
* **platform-wallet:** PR-6 follow-up — per-tx status tracking + finality events
* **platform-wallet:** PR-7 — identity update, address fund flows, DPNS
* **platform-wallet:** PR-8 — TokenWallet with registry-based balance tracking
* **platform-wallet:** register DashPay contact accounts in ManagedWalletInfo
* **platform-wallet:** register_from_addresses + FFI entry point
* **platform-wallet:** replace stage with persister, add apply()
* **platform-wallet:** scope docs, dashpay avatar fields, contact apply
* **platform-wallet:** snapshot PlatformPayment pools for storage explorer
* **platform-wallet:** SPV adapter produces and stages changesets after block processing
* **platform-wallet:** SPV adapter uses key-wallet native changesets
* **platform-wallet:** thread PutSettings through identity wallet, pre-check proof in wait_for_proof
* **platform-wallet:** TransactionBroadcaster trait for AssetLockManager (PR-25)
* **platform-wallet:** use dashcore WalletManager directly, delete SpvWalletAdapter
* **platform-wallet:** use v0.42-dev WalletManager two-map design
* **platform-wallet:** wallet-based identity search with resume / full-rescan
* **platform-wallet:** wallet-path contest vote state read
* **platform-wallet:** watch-only restore + core address persistence
* **platform-wallet:** wire DashPay profile read + edit through FFI to iOS
* **platform-wallet:** wire DPNS register/resolve/search through platform-wallet
* **platform-wallet:** wrap WalletBalance in Arc for shared access
* **platform:** add GetDocumentsCount and GetDocumentsSplitCount queries ([#3435](https://github.com/dashpay/platform/issues/3435))
* **platform:** add shielded pool query layer (medusa part 4) ([#3228](https://github.com/dashpay/platform/issues/3228))
* **platform:** derive+persist + sign-with-resolver Rust-owned pipelines ([#3542](https://github.com/dashpay/platform/issues/3542))
* **platform:** document count index ([#2516](https://github.com/dashpay/platform/issues/2516))
* **platform:** external KeychainSigner end-to-end + identity flow sweep ([#3541](https://github.com/dashpay/platform/issues/3541))
* **platform:** getDocuments v1 — SQL-shaped select + count surface ([#3633](https://github.com/dashpay/platform/issues/3633))
* **platform:** iOS late-April pass + IdentityManager restructure ([#3538](https://github.com/dashpay/platform/issues/3538))
* **platform:** verifiable, bounded count queries on a unified endpoint ([#3623](https://github.com/dashpay/platform/issues/3623))
* **rs-sdk-ffi:** add shielded pool FFI bindings with BLAST sync and transitions ([#3239](https://github.com/dashpay/platform/issues/3239))
* **rs-sdk:** add shielded pool SDK support ([#3230](https://github.com/dashpay/platform/issues/3230))
* **rs-sdk:** implement getTokenPreProgrammedDistributions query ([#3246](https://github.com/dashpay/platform/issues/3246))
* **rs-sdk:** implement incremental address balance synchronization ([#3152](https://github.com/dashpay/platform/issues/3152))
* **sdk:** add platform address transition WASM bindings ([#3147](https://github.com/dashpay/platform/issues/3147))
* **sdk:** auto-detect protocol version from network response metadata ([#3483](https://github.com/dashpay/platform/issues/3483))
* **sdk:** source mainnet/testnet bootstrap from dash-network-seeds (backport [#3533](https://github.com/dashpay/platform/issues/3533)) ([#3570](https://github.com/dashpay/platform/issues/3570))
* **swift-example-app:** add DashPay Profile section to Identity Details
* **swift-example-app:** collapse Platform Sync Status into compact view ([#3618](https://github.com/dashpay/platform/issues/3618))
* **swift-example-app:** debounced live validation of faucet RPC password ([#3590](https://github.com/dashpay/platform/issues/3590))
* **swift-example-app:** fetch avatar bytes before submitting DashPay profile
* **swift-example-app:** group storage-explorer public keys by wallet + identity
* **swift-example-app:** keychain explorer in Settings > Data
* **swift-example-app:** richer 5-row wallet list cells
* **swift-example-app:** send-payment sheet + contact display names
* **swift-example-app:** SendDashPayPaymentSheet balance + recipient profile
* **swift-example-app:** surface wallet relationship in storage detail
* **swift-example-app:** wire Create Identity submit
* **swift-example-app:** wire FriendsView to platform-wallet DashPay
* **swift-sdk:** add "View Seed Phrase" to Wallet Info
* **swift-sdk:** add BLAST address sync for iOS ([#3377](https://github.com/dashpay/platform/issues/3377))
* **swift-sdk:** add CoreWallet and AssetLock Swift wrappers
* **swift-sdk:** add full shielded pool (ZK) support for iOS ([#3348](https://github.com/dashpay/platform/issues/3348))
* **swift-sdk:** add mnemonic storage to WalletStorage ([#3477](https://github.com/dashpay/platform/issues/3477))
* **swift-sdk:** add state transitions ([#3008](https://github.com/dashpay/platform/issues/3008))
* **swift-sdk:** add Storage Explorer to browse all SwiftData models
* **swift-sdk:** add support for v3 queries ([#3004](https://github.com/dashpay/platform/issues/3004))
* **swift-sdk:** add SwiftData models for wallet, accounts, transactions, UTXOs
* **swift-sdk:** add ZK sync, local Docker support, and account management (part 1) ([#3393](https://github.com/dashpay/platform/issues/3393))
* **swift-sdk:** adjust platform wallet for latest `rust-dashcore` changes ([#2935](https://github.com/dashpay/platform/issues/2935))
* **swift-sdk:** contracts tab, identity tokens, owner relationship ([#3544](https://github.com/dashpay/platform/issues/3544))
* **swift-sdk:** drive Receive Dash Core tab from persisted addresses
* **swift-sdk:** drive shielded sync from Rust platform-wallet ([#3601](https://github.com/dashpay/platform/issues/3601))
* **swift-sdk:** grey out zero-balance rows in Create Identity picker
* **swift-sdk:** Identity Registration Index picker in Create Identity
* **swift-sdk:** lower iOS deployment target to 17.0 and expose public APIs ([#3473](https://github.com/dashpay/platform/issues/3473))
* **swift-sdk:** multi-wallet keychain recovery + BIP32/BIP44 address routing fix
* **swift-sdk:** new Create Identity UI on the Wallets screen
* **swift-sdk:** obfuscate runtime mnemonic bytes ([#3545](https://github.com/dashpay/platform/issues/3545))
* **swift-sdk:** orphan-mnemonic recovery flow on wallet load
* **swift-sdk:** PersistentIdentity ↔ PersistentWallet relationship + partial read migration
* **swift-sdk:** platform-to-platform send ([#3626](https://github.com/dashpay/platform/issues/3626))
* **swift-sdk:** PlatformWalletManager.deleteWallet wipes full wallet footprint
* **swift-sdk:** populate Receive Dash Platform tab from persisted addresses
* **swift-sdk:** registerIdentityFromAddresses wrapper
* **swift-sdk:** reorder wallet accounts + platform-specific detail
* **swift-sdk:** send transaction ([#3130](https://github.com/dashpay/platform/issues/3130))
* **swift-sdk:** show balance + enforce unselectable zero-balance rows
* **swift-sdk:** show derivation path on Receive Dash
* **swift-sdk:** show public key hex on Receive Dash
* **swift-sdk:** split Storage Explorer Core vs Platform addresses
* **swift-sdk:** Swift wrapper for PlatformWalletManager SPV FFI
* **swift-sdk:** update iOS build destination to use generic simulator ([#3036](https://github.com/dashpay/platform/issues/3036))
* **swift-sdk:** wallet memory explorer + persistor UTXO/sync load ([#3576](https://github.com/dashpay/platform/issues/3576))
* **swift-sdk:** wire real token actions through platform-wallet ([#3548](https://github.com/dashpay/platform/issues/3548))
* **swift-sdk:** wire up PlatformWalletManager for BLAST sync
* **swift-sdk:** wire WalletChangeSet persistence callback into SwiftData
* **swift-sdk:** write Clear private keys to Keychain from persister callback
* update dependencies to version 0.41.0
* update wallet info interface and transaction handling ([#3001](https://github.com/dashpay/platform/issues/3001))
* **wasm-sdk:** add shielded pool WASM bindings and query methods ([#3235](https://github.com/dashpay/platform/issues/3235))
* **wasm-sdk:** add token-paid document support ([#3599](https://github.com/dashpay/platform/issues/3599))


### Bug Fixes

* adapt to dashcore InstantLock API changes and increase event channel capacity
* add rs-scripts to Docker build context ([#3455](https://github.com/dashpay/platform/issues/3455))
* bump dompurify and tar for security fixes ([#3184](https://github.com/dashpay/platform/issues/3184))
* **ci:** add rs-dash-async to Docker build context ([#3543](https://github.com/dashpay/platform/issues/3543))
* **ci:** install gpg on Mac runners for Codecov upload ([#3261](https://github.com/dashpay/platform/issues/3261))
* **ci:** remove stale feature-flags-contract references breaking Docker build ([#3527](https://github.com/dashpay/platform/issues/3527))
* **ci:** target only upgrade_fork_tests in nightly workflow ([#3243](https://github.com/dashpay/platform/issues/3243))
* **ci:** use step outputs instead of secrets context in reusable workflow if conditions
* **dapi:** use deterministic keys in subscribeToNewTransactions test to prevent bloom filter false positives ([#3160](https://github.com/dashpay/platform/issues/3160))
* **dashmate:** bump systeminformation and ajv to fix npm audit failures ([#3139](https://github.com/dashpay/platform/issues/3139))
* **dashmate:** lower HP node RAM requirement to 7.3GB ([#3153](https://github.com/dashpay/platform/issues/3153))
* **dpp:** add additionalProperties: false to document meta-schema ([#3475](https://github.com/dashpay/platform/issues/3475))
* **dpp:** add missing #[test] attribute to should_set_empty_schema_defs ([#3101](https://github.com/dashpay/platform/issues/3101))
* **dpp:** add toJSON() serialization to TokenContractInfoWasm ([#3089](https://github.com/dashpay/platform/issues/3089))
* **dpp:** add upper fee bound to unshield and withdrawal builders ([#3364](https://github.com/dashpay/platform/issues/3364))
* **dpp:** bind SetPriceForDirectPurchase action_id to full pricing schedule ([#3357](https://github.com/dashpay/platform/issues/3357))
* **dpp:** bind token config update action_id to payload value (v1) ([#3346](https://github.com/dashpay/platform/issues/3346))
* **dpp:** bind unshielding_amount to sighash in client builders ([#3362](https://github.com/dashpay/platform/issues/3362))
* **dpp:** correct misleading non-mainnet minimum-interval error message ([#3668](https://github.com/dashpay/platform/issues/3668))
* **dpp:** enforce bincode byte-budget limit on enum deserialization ([#3223](https://github.com/dashpay/platform/issues/3223))
* **dpp:** enforce sum(inputs) >= amount in shield transition ([#3240](https://github.com/dashpay/platform/issues/3240))
* **dpp:** populate transferred_at in random_document_with_params when required ([#3517](https://github.com/dashpay/platform/issues/3517))
* **dpp:** reduce max_shielded_transition_actions from 100 to 16 ([#3411](https://github.com/dashpay/platform/issues/3411)) ([#3498](https://github.com/dashpay/platform/issues/3498))
* **dpp:** use DIP-0002 version 3 in asset-lock tx fixtures ([#3621](https://github.com/dashpay/platform/issues/3621))
* **dpp:** validate encrypted_note length in structure validation ([#3368](https://github.com/dashpay/platform/issues/3368))
* **drive-abci:** add input bounds to batch query endpoints ([#3296](https://github.com/dashpay/platform/issues/3296))
* **drive-abci:** guard purpose cast overflow in identities_contract_keys query ([#3275](https://github.com/dashpay/platform/issues/3275))
* **drive-abci:** swap operands in core-sync chain lock height check ([#3518](https://github.com/dashpay/platform/issues/3518))
* **drive-abci:** use checked arithmetic in shielded fee calculation ([#3365](https://github.com/dashpay/platform/issues/3365))
* **drive-abci:** use checked_sub for ShieldFromAssetLock fee computation ([#3366](https://github.com/dashpay/platform/issues/3366))
* **drive,drive-abci:** post-merge follow-ups for shielded anchor refactor ([#3606](https://github.com/dashpay/platform/issues/3606))
* **drive,drive-abci:** retire SHIELDED_MOST_RECENT_ANCHOR_KEY; derive most-recent from [8] and never empty it ([#3605](https://github.com/dashpay/platform/issues/3605))
* **drive:** add bounds check for i64 cast in token balance addition ([#3295](https://github.com/dashpay/platform/issues/3295))
* **drive:** credits-not-balanced from shielded nullifier metadata ([#3624](https://github.com/dashpay/platform/issues/3624))
* **drive:** eliminate panic in grovedb operations logging under concurrent execution ([#3142](https://github.com/dashpay/platform/issues/3142))
* **drive:** error on unexpected element type in anchor retrieval ([#3369](https://github.com/dashpay/platform/issues/3369))
* **drive:** handle malicious quorum_type without panicking ([#3288](https://github.com/dashpay/platform/issues/3288))
* **drive:** prevent overflow in SetPrices direct purchase pricing ([#3292](https://github.com/dashpay/platform/issues/3292))
* **drive:** rebalance shielded credit pool subtree keys by access frequency ([#3607](https://github.com/dashpay/platform/issues/3607))
* **drive:** replace silent epoch u16 truncation with checked conversion ([#3293](https://github.com/dashpay/platform/issues/3293))
* **drive:** verify root hash consistency in double-proof identity lookup ([#3341](https://github.com/dashpay/platform/issues/3341))
* **ffi:** post-merge compilation fixes for iOS SDK ([#3159](https://github.com/dashpay/platform/issues/3159))
* include UTXO IS-lock changeset in process_instant_send_lock
* paid/unpaid classification for invalid batch transitions ([#3616](https://github.com/dashpay/platform/issues/3616))
* **platform-wallet-ffi:** correct PlatformAddress import and annotate try_from
* **platform-wallet-ffi:** diagnose + harden register_from_addresses
* **platform-wallet-ffi:** extract balance from AddressFunds in persistence and changeset conversion
* **platform-wallet-ffi:** make FFI entry points C-ABI clean
* **platform-wallet-ffi:** refresh address nonces from Platform
* **platform-wallet-ffi:** run identity registration on 8MB-stack worker
* **platform-wallet:** add reset_filter_committed_height for test rescan
* **platform-wallet:** address review findings
* **platform-wallet:** apply Phase 9a-3 reviewer feedback
* **platform-wallet:** BalanceUpdateHandler reads from wallets map
* **platform-wallet:** carry full EstablishedContact in ContactChangeSet
* **platform-wallet:** clippy issues and imports ([#3156](https://github.com/dashpay/platform/issues/3156))
* **platform-wallet:** close races in deleteWallet identity snapshot
* **platform-wallet:** fall back to persister for chainlocked asset-lock tx records ([#3619](https://github.com/dashpay/platform/issues/3619))
* **platform-wallet:** fix critical asset lock issues — ChainLock verification, documentation
* **platform-wallet:** fix dead WalletEvent channel causing SPV crash
* **platform-wallet:** fix failing CI ([#3638](https://github.com/dashpay/platform/issues/3638))
* **platform-wallet:** fix flaky Base58 identifier length assertion ([#3245](https://github.com/dashpay/platform/issues/3245))
* **platform-wallet:** fix review findings — broadcast doc, run() error handling
* **platform-wallet:** fix SPV test — real SDK, logging, 600s timeout
* **platform-wallet:** fix test compilation and DIP-14 account test
* **platform-wallet:** guard set_contact_bloom_registered_count against zero
* **platform-wallet:** Item 8.1d — recover_asset_lock_blocking queues changeset
* **platform-wallet:** make remove_asset_lock pub(crate)
* **platform-wallet:** make sync_profiles pub so evo-tool dispatcher can call it
* **platform-wallet:** make track_asset_lock private
* **platform-wallet:** match asset lock status to actual proof type
* **platform-wallet:** match new AssetLockCreditKeys enum shape, closes [#661](https://github.com/dashpay/platform/issues/661)
* **platform-wallet:** monotonic merge for last_scanned_index (S2)
* **platform-wallet:** persist identity balance after top-up + transfer
* **platform-wallet:** remove unnecessary 8-confirmation wait for chain-locked txs
* **platform-wallet:** replace blocking_read with async .await and fix FFI ABI issues
* **platform-wallet:** seed SPV peers from DAPI addresses in e2e test
* **platform-wallet:** token transitions require a CRITICAL signing key ([#3551](https://github.com/dashpay/platform/issues/3551))
* **platform-wallet:** update example for per-account spendable_utxos
* replace deadlocking futures::executor::block_on with runtime-aware dash-async crate ([#3432](https://github.com/dashpay/platform/issues/3432)) ([#3497](https://github.com/dashpay/platform/issues/3497))
* resolve clippy warnings across workspace after v3.1-dev merge
* **rs-dapi,sdk:** decode base64 CBOR error messages from Tenderdash ([#3350](https://github.com/dashpay/platform/issues/3350))
* **rs-dapi:** correct RPC error code to DapiError mapping ([#3316](https://github.com/dashpay/platform/issues/3316))
* **rs-dapi:** remove unused functions and unnecessary cast ([#3253](https://github.com/dashpay/platform/issues/3253))
* **rs-scripts:** remove redundant wildcard pattern blocking CI ([#3430](https://github.com/dashpay/platform/issues/3430))
* **rs-sdk-ffi:** fix double-free in address result free functions ([#3338](https://github.com/dashpay/platform/issues/3338))
* **rs-sdk-ffi:** fix Vec capacity mismatch across FFI boundary ([#3339](https://github.com/dashpay/platform/issues/3339))
* **rs-sdk-ffi:** prevent heap corruption from Vec capacity mismatch in FFI ([#3289](https://github.com/dashpay/platform/issues/3289))
* **rs-sdk-ffi:** update testnet DAPI addresses to new hp-masternodes
* **rs-sdk-ffi:** zeroize private key arrays after use in crypto/signer FFI ([#3433](https://github.com/dashpay/platform/issues/3433))
* **rs-sdk:** withdrawals orderBy bug ([#2409](https://github.com/dashpay/platform/issues/2409)) ([#3536](https://github.com/dashpay/platform/issues/3536))
* **sdk:** add custom deallocator to signer vtable for FFI safety ([#3304](https://github.com/dashpay/platform/issues/3304))
* **sdk:** add Regtest support to trusted context provider activation height ([#3464](https://github.com/dashpay/platform/issues/3464))
* **sdk:** add unified dash_sdk_result_free to prevent memory leaks ([#3298](https://github.com/dashpay/platform/issues/3298))
* **sdk:** default to nonce 0 for first-time identity/contract interactions ([#3170](https://github.com/dashpay/platform/issues/3170))
* **sdk:** propagate PutSettings in token freeze/mint/unfreeze/set_price transitions ([#3132](https://github.com/dashpay/platform/issues/3132))
* **sdk:** remove Document fetch_many override referencing removed parse_proof ([#3179](https://github.com/dashpay/platform/issues/3179))
* **sdk:** remove unsafe Copy derive and fix ContextProviderWrapper leak ([#3301](https://github.com/dashpay/platform/issues/3301))
* **sdk:** remove unsound catch_unwind on raw pointer dereference in FFI ([#3299](https://github.com/dashpay/platform/issues/3299))
* **sdk:** remove unused Sdk::parse_proof and Sdk::parse_proof_with_metadata ([#3141](https://github.com/dashpay/platform/issues/3141))
* **sdk:** replace env::set_var with direct filter in FFI logging setup ([#3302](https://github.com/dashpay/platform/issues/3302))
* **sdk:** use deterministic identity ID in address funding proof verification ([#3208](https://github.com/dashpay/platform/issues/3208))
* **sdk:** use string keys instead of object keys in JavaScript Maps ([#3145](https://github.com/dashpay/platform/issues/3145))
* **swift-example-app:** avoid crash reading DashPay profile by id on fresh identities
* **swift-example-app:** drop misleading memo field from SendDashPayPaymentSheet
* **swift-example-app:** hold one PlatformWalletManager per network ([#3591](https://github.com/dashpay/platform/issues/3591))
* **swift-example-app:** keychain-explorer row tap no longer pops the screen
* **swift-example-app:** persist newly-created identity as non-local
* **swift-example-app:** pin dates to Gregorian calendar
* **swift-example-app:** point regtest+docker SPV at dashmate seed port ([#3589](https://github.com/dashpay/platform/issues/3589))
* **swift-example-app:** receive address always picks BIP44 account over BIP32 ([#3600](https://github.com/dashpay/platform/issues/3600))
* **swift-example-app:** route orphan recovery to per-network managers ([#3612](https://github.com/dashpay/platform/issues/3612))
* **swift-example-app:** Settings tab uses NavigationStack, not NavigationView
* **swift-example-app:** show real platform balance on Send screen ([#3602](https://github.com/dashpay/platform/issues/3602))
* **swift-example-app:** silence optional-interpolation warnings
* **swift-example-app:** silence Picker tag-mismatch warnings
* **swift-example-app:** stop the keychain-explorer row tap from bouncing back
* **swift-sdk:** add balanceText field + Color.secondary for picker row
* **swift-sdk:** add form validation helpers and stabilize example app tests ([#3030](https://github.com/dashpay/platform/issues/3030))
* **swift-sdk:** add fund source picker and Core-to-Core payment support
* **swift-sdk:** add missing on_persist_wallet_changeset_fn to PersistenceCallbacks
* **swift-sdk:** add missing SwiftData models to ModelContainerHelper schema
* **swift-sdk:** conform ManagedPlatformAddressWallet value types to Sendable
* **swift-sdk:** contracts integration polish ([#3604](https://github.com/dashpay/platform/issues/3604))
* **swift-sdk:** crash when switching to devnet in settings ([#3394](https://github.com/dashpay/platform/issues/3394))
* **swift-sdk:** denormalize walletId onto PersistentTransaction
* **swift-sdk:** drive Sync Status platform balance from SwiftData
* **swift-sdk:** drop standardTag filter for primary receive account
* **swift-sdk:** eliminate Swift compiler warnings that fail CI ([#3171](https://github.com/dashpay/platform/issues/3171))
* **swift-sdk:** fall back to scope wallet when persisting identities
* **swift-sdk:** fix spv usage ([#3026](https://github.com/dashpay/platform/issues/3026))
* **swift-sdk:** fix transaction list view not showing new transactions ([#3574](https://github.com/dashpay/platform/issues/3574))
* **swift-sdk:** fixed dUplicated symbols issue in BaseViewModel ([#3074](https://github.com/dashpay/platform/issues/3074))
* **swift-sdk:** fixed ios app transaction display ([#3081](https://github.com/dashpay/platform/issues/3081))
* **swift-sdk:** fixed wallet balance calculation ([#3082](https://github.com/dashpay/platform/issues/3082))
* **swift-sdk:** format platform balances with correct units
* **swift-sdk:** hide zero-balance accounts from Create Identity picker
* **swift-sdk:** include standardTag in persistAccount upsert filter
* **swift-sdk:** load address pools into the wallet after restart ([#3686](https://github.com/dashpay/platform/issues/3686))
* **swift-sdk:** made executeAsync generic implement Sendable ([#3058](https://github.com/dashpay/platform/issues/3058))
* **swift-sdk:** make deleteWalletData reach the wallet's transactions
* **swift-sdk:** make WalletStorage initializer public
* **swift-sdk:** mark ManagedPlatformWallet.CreatedIdentity Sendable
* **swift-sdk:** mirror BLAST balances onto PersistentCoreAddress
* **swift-sdk:** pass modelContainer and walletId to persistence layer
* **swift-sdk:** pending transaction display ([#3447](https://github.com/dashpay/platform/issues/3447))
* **swift-sdk:** persist addresses derived by gap-limit extension ([#3582](https://github.com/dashpay/platform/issues/3582))
* **swift-sdk:** pull Platform receive address from the Absent pool
* **swift-sdk:** query PersistentAccount directly for receive address
* **swift-sdk:** query platform balance per wallet instead of singleton
* **swift-sdk:** realign Swift FFI shims with current rust-dashcore + platform-wallet-ffi
* **swift-sdk:** rebind platform-balance sync when active wallet changes
* **swift-sdk:** reconcile spending tx <-> spent TXO + tx-detail UX + dashcore bump ([#3581](https://github.com/dashpay/platform/issues/3581))
* **swift-sdk:** restore custom SPV peers toggle on non-regtest ([#3559](https://github.com/dashpay/platform/issues/3559))
* **swift-sdk:** restore legacy SDK path for BLAST sync
* **swift-sdk:** restore Shielded Sync Status section in CoreContentView
* **swift-sdk:** restrict Create Identity funding picker to spendable accounts
* **swift-sdk:** scope platform sync state by network
* **swift-sdk:** scope UI by active network and add multi-wallet recovery sheet ([#3583](https://github.com/dashpay/platform/issues/3583))
* **swift-sdk:** serialize SwiftData ModelContext access from FFI callbacks ([#3558](https://github.com/dashpay/platform/issues/3558))
* **swift-sdk:** show combined wallet balance on Wallets list ([#3537](https://github.com/dashpay/platform/issues/3537))
* **swift-sdk:** show formatted local/UTC times in sync state detail view
* **swift-sdk:** split BIP44 vs BIP32 Standard accounts into sections
* **swift-sdk:** split Receive Dash address lookup into small functions
* **swift-sdk:** split reset() into clearDisplay() vs reset() for platform sync
* **swift-sdk:** update for upstream key-wallet-ffi and dash-spv-ffi changes
* **swift-sdk:** update SPVSyncState methods to match dash-spv implementation ([#3378](https://github.com/dashpay/platform/issues/3378))
* **swift-sdk:** use platform-wallet-ffi headers instead of hardcoded function signatures ([#3500](https://github.com/dashpay/platform/issues/3500))
* **swift-sdk:** wipe SwiftData before keychain in deleteWallet
* **swift-sdk:** wrapper for FFITxOutput that correctly handles alloc memory ([#3472](https://github.com/dashpay/platform/issues/3472))
* use header file to define platform-wallet-ffi public ABI and force swift-sdk to use it ([#3553](https://github.com/dashpay/platform/issues/3553))
* **wallet-lib:** fix broadcast retry not matching DAPI error message ([#3434](https://github.com/dashpay/platform/issues/3434))


### Performance Improvements

* **drive-abci:** consolidate shielded test proof generation for faster CI ([#3325](https://github.com/dashpay/platform/issues/3325))
* **platform-wallet-ffi,swift-sdk:** one atomic SwiftData save per changeset round
* split shielded tests to share verifying key build ([#3349](https://github.com/dashpay/platform/issues/3349))
* **swift-example-app:** count transactions via indexed query, not relationship faults
* **swift-sdk:** compound index on PersistentTransaction(walletId, firstSeen)
* **swift-sdk:** gate SPV progress polling on inequality ([#3555](https://github.com/dashpay/platform/issues/3555))
* **swift-sdk:** run PlatformAddress BLAST sync off the main actor


### Styles

* **platform-wallet-ffi:** cargo fmt
* **platform-wallet:** cargo fmt
* run cargo fmt ([#3140](https://github.com/dashpay/platform/issues/3140))


### Documentation

* add a book for platform development with design philosophy ([#3080](https://github.com/dashpay/platform/issues/3080))
* add Evo SDK chapters ([#3422](https://github.com/dashpay/platform/issues/3422))
* add nightly test status page and badge ([#3398](https://github.com/dashpay/platform/issues/3398))
* add plan
* add Platform Addresses chapter ([#3374](https://github.com/dashpay/platform/issues/3374))
* **book:** add BLAST sync chapter ([#3231](https://github.com/dashpay/platform/issues/3231))
* **book:** add document serialization wire format chapter ([#3392](https://github.com/dashpay/platform/issues/3392))
* **book:** add identity keys deep dive chapter ([#3232](https://github.com/dashpay/platform/issues/3232))
* **dpp:** add safety comments for auditor false-positive patterns ([#3363](https://github.com/dashpay/platform/issues/3363))
* **drive-abci:** clarify intentional absence of pool notes check in shielded transfer ([#3294](https://github.com/dashpay/platform/issues/3294))
* **drive-abci:** document that direct purchases bypass token pause by design ([#3309](https://github.com/dashpay/platform/issues/3309))
* **drive-abci:** document that unrestricted GetPathElements is by design ([#3305](https://github.com/dashpay/platform/issues/3305))
* **drive:** document trusted-state rationale for bincode NoLimit ([#3370](https://github.com/dashpay/platform/issues/3370))
* fix plan
* **platform-wallet-ffi:** fix stale DashPayWallet reference in dashpay_profile.rs
* **platform-wallet:** add compute-then-apply architecture to PLAN.md
* **platform-wallet:** add missing identity methods to PR-14 spec
* **platform-wallet:** add PR-17 for dashcore asset lock builder adoption, closes [rust-dashcore#604](https://github.com/dashpay/rust-dashcore/issues/604)
* **platform-wallet:** add PR-20 spec — complete identity/asset lock lifecycle
* **platform-wallet:** add PR-22 implementation plan
* **platform-wallet:** add PR-22 spec — ChangeSet-based persistence
* **platform-wallet:** add PR-26 for lock ordering audit
* **platform-wallet:** add PR-30 spec — switch to dashcore WalletManager
* **platform-wallet:** add PR-6 plan — upstream dashcore + evo-tool sync
* **platform-wallet:** add review findings to risk analysis + PR-19 FFI fix
* **platform-wallet:** add SingleKeyWallet migration to PR-22 done criteria
* **platform-wallet:** complete PLAN.md — steps 9-13, smart persistence strategy
* **platform-wallet:** comprehensive plan spec update to match code
* **platform-wallet:** comprehensive PLAN update — all sections rewritten
* **platform-wallet:** detailed migration tally with per-task breakdown
* **platform-wallet:** document confirmed deadlock risk in PR-26
* **platform-wallet:** expand by-value apply follow-up to cover key-wallet too
* **platform-wallet:** expand PR-20 — multi-funding, lifecycle tracking, recovery
* **platform-wallet:** fix PLAN.md — key-wallet uses changeset/ not persistence/
* **platform-wallet:** mark PR-14 complete
* **platform-wallet:** mark PR-15 complete
* **platform-wallet:** mark PR-16 complete
* **platform-wallet:** mark PR-19 complete — all 10 duplicate fields removed
* **platform-wallet:** mark PR-20/21 done, add PR-31 for leftovers
* **platform-wallet:** mark PR-9/10/11/12 as complete
* **platform-wallet:** Phase 9b gap candidates + Phase 10+ open questions
* **platform-wallet:** PR-17 blocked — asset lock builder not on v0.42-dev yet, closes [rust-dashcore#604](https://github.com/dashpay/rust-dashcore/issues/604)
* **platform-wallet:** PR-9 expanded — full evo-tool integration plan
* **platform-wallet:** rescope PR-16 — finality only, keep SpvManager
* **platform-wallet:** restructure plan — evo-tool integration moved up
* **platform-wallet:** rewrite PLAN.md — persister on wallet, no stage field
* **platform-wallet:** rewrite PR-19 spec with DashPay contact flow
* **platform-wallet:** rewrite PR-20 — CoreWallet owns asset lock lifecycle
* **platform-wallet:** rewrite PR-22 spec — two-layer ChangeSet architecture
* **platform-wallet:** spec PR-10/11/12 — library enrichment before full integration
* **platform-wallet:** spec PR-14 — DashPay protocol completeness
* **platform-wallet:** spec PR-15 — shielded pool with storage abstraction
* **platform-wallet:** update architecture + implementation sections for PR-10/11/12
* **platform-wallet:** update architecture and struct definitions
* **platform-wallet:** update PERSISTENCE_REDESIGN with evo-tool research
* **platform-wallet:** update plan with owned/watched split + ManagedIdentitySigner
* **platform-wallet:** update plan with PR-13 completion + migration tally
* **platform-wallet:** update PLAN with PR-2 architecture and risk analysis
* **platform-wallet:** update PLAN with PR-6/7/8 completion status
* **platform-wallet:** update PLAN.md — PR-18 completed
* **platform-wallet:** update PLAN.md — PR-18 final, add PR-19 spec
* **platform-wallet:** update PLAN.md with single-lock architecture and current status
* **platform-wallet:** update PR-14 with final migration tally
* **platform-wallet:** update PR-19 checklist — phases 1-4 done
* **platform-wallet:** update PR-20 — AssetLockManager SPV event subscription
* **platform-wallet:** warn about disconnected event channel in from_wallet_and_info
* **platform-wallet:** write-path catalogue + corrected Phase 9a plan
* publish Rust, gRPC, and JS/TS API docs on GitHub Pages ([#3157](https://github.com/dashpay/platform/issues/3157))
* **readme:** add commit activity and last commit badges ([#3308](https://github.com/dashpay/platform/issues/3308))
* **readme:** improve coverage table with crate links and line counts ([#3324](https://github.com/dashpay/platform/issues/3324))
* review and correct the plan
* rewrite README with technical overview of Dash Platform ([#3276](https://github.com/dashpay/platform/issues/3276))
* **rs-sdk-ffi:** document catch_unwind rationale at FFI boundaries ([#3344](https://github.com/dashpay/platform/issues/3344))
* **rs-sdk:** fix rustdoc inaccuracies and resolve all cargo doc warnings ([#3161](https://github.com/dashpay/platform/issues/3161))
* **sdk:** add JS SDK comparison matrix and note to use Evo SDK ([#3467](https://github.com/dashpay/platform/issues/3467))
* **sdk:** add README with install, usage, and facade reference ([#3234](https://github.com/dashpay/platform/issues/3234))
* **sdk:** fix platform book evo-sdk tutorial code to match 3.1.0-dev API ([#3423](https://github.com/dashpay/platform/issues/3423))
* simplify architecture
* slim README comparison table, add comprehensive book chapter ([#3312](https://github.com/dashpay/platform/issues/3312))
* **wasm-dpp:** document no-op user fee increase methods on vote transition ([#3209](https://github.com/dashpay/platform/issues/3209))


### Build System

* add rs-unified-sdk-ffi to Dockerfile COPY blocks ([#3418](https://github.com/dashpay/platform/issues/3418))
* bump wasm-bindgen to 0.2.108 ([#3108](https://github.com/dashpay/platform/issues/3108))
* **platform-wallet-ffi:** emit cbindgen header for xcframework umbrella
* remove timestamp from gRPC cache to prevent unnecessary pushes ([#3219](https://github.com/dashpay/platform/issues/3219))
* require sdk-ignore annotation for unimplemented gRPC queries ([#3213](https://github.com/dashpay/platform/issues/3213))
* respect CARGO_TARGET_DIR in WASM build scripts ([#3155](https://github.com/dashpay/platform/issues/3155))
* run all test jobs on workflow_dispatch ([#3226](https://github.com/dashpay/platform/issues/3226))
* update rust-dashcore crates to v0.42-dev (542a617)  ([#3104](https://github.com/dashpay/platform/issues/3104))
* update rust-dashcore to 2824e52a ([#3168](https://github.com/dashpay/platform/issues/3168))
* update rust-dashcore to latest v0.42-dev 9959201 ([#3225](https://github.com/dashpay/platform/issues/3225))


### Tests

* cover abci handler, drive contract/document/group, drive-abci config ([#3516](https://github.com/dashpay/platform/issues/3516))
* cover document_type, drive votes/tokens/identity/shielded, drive-abci validation ([#3525](https://github.com/dashpay/platform/issues/3525))
* cover drive contract/tokens, rs-dpp document, drive-abci platform_events ([#3523](https://github.com/dashpay/platform/issues/3523))
* cover drive/document, votes, queries, object_size_info, and lowcov ([#3513](https://github.com/dashpay/platform/issues/3513))
* cover low-coverage modules in dpp and drive ([#3506](https://github.com/dashpay/platform/issues/3506))
* cover proof-verifier, batch token actions, identity conversions ([#3526](https://github.com/dashpay/platform/issues/3526))
* cover query handlers, platform_events, state_transition verify, document v0 ([#3528](https://github.com/dashpay/platform/issues/3528))
* cover shielded queries and low-cov modules in drive/drive-abci ([#3511](https://github.com/dashpay/platform/issues/3511))
* **dpp:** add deserialization failure tests and fix stale test structs ([#3128](https://github.com/dashpay/platform/issues/3128))
* **dpp:** add numerical assertions to evaluate_interval distribution tests ([#3102](https://github.com/dashpay/platform/issues/3102))
* **dpp:** add validation error path tests for identity_nonce, max_depth, and GroupV0 ([#3323](https://github.com/dashpay/platform/issues/3323))
* **dpp:** cover state transitions, token config, perpetual distribution ([#3512](https://github.com/dashpay/platform/issues/3512))
* **dpp:** improve address_funds state transition test coverage ([#3285](https://github.com/dashpay/platform/issues/3285))
* **dpp:** improve batch_transition test coverage ([#3284](https://github.com/dashpay/platform/issues/3284))
* **dpp:** improve contract state transition test coverage ([#3290](https://github.com/dashpay/platform/issues/3290))
* **dpp:** improve coverage for cbor canonical, json utils, and document accessors ([#3383](https://github.com/dashpay/platform/issues/3383))
* **dpp:** improve coverage for data contract serialization and index validation ([#3438](https://github.com/dashpay/platform/issues/3438))
* **dpp:** improve coverage for distribution functions, config, core scripts, and asset lock proofs ([#3450](https://github.com/dashpay/platform/issues/3450))
* **dpp:** improve coverage for document property serialization and encoding ([#3439](https://github.com/dashpay/platform/issues/3439))
* **dpp:** improve coverage for document serialization, extended documents, and methods ([#3454](https://github.com/dashpay/platform/issues/3454))
* **dpp:** improve coverage for epoch distribution, JSON safe serialization, and address witness ([#3440](https://github.com/dashpay/platform/issues/3440))
* **dpp:** improve coverage for identity state transitions and public keys in creation ([#3456](https://github.com/dashpay/platform/issues/3456))
* **dpp:** improve coverage for token config validation, cbor utils, and identity factory ([#3381](https://github.com/dashpay/platform/issues/3381))
* **dpp:** improve document serialization and schema validation coverage ([#3388](https://github.com/dashpay/platform/issues/3388))
* **dpp:** improve document type property and index coverage ([#3387](https://github.com/dashpay/platform/issues/3387))
* **dpp:** improve state transition coverage for batched and shielded ([#3359](https://github.com/dashpay/platform/issues/3359))
* **dpp:** pin V0 config parser consensus-frozen quirk ([#3514](https://github.com/dashpay/platform/issues/3514))
* **drive-abci:** add comprehensive tests for group query modules ([#3268](https://github.com/dashpay/platform/issues/3268))
* **drive-abci:** add comprehensive tests for token query v0 modules ([#3265](https://github.com/dashpay/platform/issues/3265))
* **drive-abci:** add happy path tests for token burn, freeze, emergency action, and destroy frozen funds ([#3459](https://github.com/dashpay/platform/issues/3459))
* **drive-abci:** add shielded_common unit tests ([#3329](https://github.com/dashpay/platform/issues/3329))
* **drive-abci:** add tests for address_funds query modules ([#3266](https://github.com/dashpay/platform/issues/3266))
* **drive-abci:** improve abci handler test coverage ([#3326](https://github.com/dashpay/platform/issues/3326))
* **drive-abci:** improve abci module test coverage ([#3273](https://github.com/dashpay/platform/issues/3273))
* **drive-abci:** improve batch validation test coverage ([#3280](https://github.com/dashpay/platform/issues/3280))
* **drive-abci:** improve check_tx_verification test coverage ([#3281](https://github.com/dashpay/platform/issues/3281))
* **drive-abci:** improve common validation test coverage ([#3283](https://github.com/dashpay/platform/issues/3283))
* **drive-abci:** improve coverage for quorum sets, block proposals, and identity nonces ([#3453](https://github.com/dashpay/platform/issues/3453))
* **drive-abci:** improve data_contract_update validation coverage ([#3322](https://github.com/dashpay/platform/issues/3322))
* **drive-abci:** improve execution engine test coverage ([#3272](https://github.com/dashpay/platform/issues/3272))
* **drive-abci:** improve execution types test coverage ([#3274](https://github.com/dashpay/platform/issues/3274))
* **drive-abci:** improve identity state transition validation coverage ([#3320](https://github.com/dashpay/platform/issues/3320))
* **drive-abci:** improve identity_based_queries test coverage ([#3271](https://github.com/dashpay/platform/issues/3271))
* **drive-abci:** improve platform_events coverage (round 2) ([#3321](https://github.com/dashpay/platform/issues/3321))
* **drive-abci:** improve platform_events test coverage ([#3270](https://github.com/dashpay/platform/issues/3270))
* **drive-abci:** improve processor validation test coverage ([#3282](https://github.com/dashpay/platform/issues/3282))
* **drive-abci:** improve replay module test coverage ([#3328](https://github.com/dashpay/platform/issues/3328))
* **drive-abci:** improve validator_queries test coverage ([#3311](https://github.com/dashpay/platform/issues/3311))
* **drive-abci:** improve validator_set_update v2 coverage, ignore v0/v1 ([#3327](https://github.com/dashpay/platform/issues/3327))
* **drive:** add comprehensive tests for state_transition_action module ([#3229](https://github.com/dashpay/platform/issues/3229))
* **drive:** add comprehensive tests for verify module ([#3233](https://github.com/dashpay/platform/issues/3233))
* **drive:** add coverage for fee calculation engine ([#3429](https://github.com/dashpay/platform/issues/3429))
* **drive:** add error path tests for document CRUD and cache lifecycle ([#3129](https://github.com/dashpay/platform/issues/3129))
* **drive:** add tests for verify_state_transition_was_executed_with_proof v0 ([#3264](https://github.com/dashpay/platform/issues/3264))
* **drive:** add verify proof coverage for tokens and voting v0 modules ([#3263](https://github.com/dashpay/platform/issues/3263))
* **drive:** address review comments on identity test coverage ([#3336](https://github.com/dashpay/platform/issues/3336))
* **drive:** cover token transition action transformers and accessors ([#3505](https://github.com/dashpay/platform/issues/3505))
* **drive:** improve contract insert and update test coverage ([#3356](https://github.com/dashpay/platform/issues/3356))
* **drive:** improve contract module test coverage ([#3332](https://github.com/dashpay/platform/issues/3332))
* **drive:** improve coverage for query conditions and filter matching ([#3441](https://github.com/dashpay/platform/issues/3441))
* **drive:** improve coverage for storage forms, batch operations, and vote resolution ([#3449](https://github.com/dashpay/platform/issues/3449))
* **drive:** improve coverage for tokens subtree ([#3503](https://github.com/dashpay/platform/issues/3503))
* **drive:** improve coverage for vote paths, document info, token ops, and asset lock proofs ([#3452](https://github.com/dashpay/platform/issues/3452))
* **drive:** improve coverage for vote poll query modules ([#3445](https://github.com/dashpay/platform/issues/3445))
* **drive:** improve document module test coverage ([#3333](https://github.com/dashpay/platform/issues/3333))
* **drive:** improve group module test coverage ([#3331](https://github.com/dashpay/platform/issues/3331))
* **drive:** improve grove_operations test coverage ([#3343](https://github.com/dashpay/platform/issues/3343))
* **drive:** improve identity fetch, balance, and public key hash coverage ([#3443](https://github.com/dashpay/platform/issues/3443))
* **drive:** improve identity key fetch, prove, and queries coverage ([#3442](https://github.com/dashpay/platform/issues/3442))
* **drive:** improve identity module test coverage ([#3330](https://github.com/dashpay/platform/issues/3330))
* **drive:** improve query conditions and token paths coverage ([#3386](https://github.com/dashpay/platform/issues/3386))
* **drive:** optimize strategy test execution times ([#3241](https://github.com/dashpay/platform/issues/3241))
* improve coverage for dpp and drive-proof-verifier ([#3504](https://github.com/dashpay/platform/issues/3504))
* **platform-value:** improve coverage for pointer, bytes_36, path operations, and diff ([#3437](https://github.com/dashpay/platform/issues/3437))
* **platform-wallet:** add minimal SPV sync integration test
* **platform-wallet:** round-trip apply tests for sync mutation surface (Phase 9a-4)
* **platform-wallet:** verify core persistence in SPV e2e test
* **platform:** add 466 unit tests across 15 files for coverage gains ([#3427](https://github.com/dashpay/platform/issues/3427))
* **platform:** coverage round 3 — replace, index, bytes, distribution encode + exclusions ([#3431](https://github.com/dashpay/platform/issues/3431))
* **platform:** improve btreemap extensions test coverage ([#3380](https://github.com/dashpay/platform/issues/3380))
* **platform:** improve platform-serialization test coverage ([#3317](https://github.com/dashpay/platform/issues/3317))
* **platform:** improve platform-value coverage for inner_value, system_bytes, and serde ([#3384](https://github.com/dashpay/platform/issues/3384))
* **platform:** improve platform-value coverage for patch, value_map, converters, and replacement ([#3428](https://github.com/dashpay/platform/issues/3428))
* **platform:** improve platform-value test coverage ([#3313](https://github.com/dashpay/platform/issues/3313))
* **rs-dapi:** improve test coverage ([#3310](https://github.com/dashpay/platform/issues/3310))
* **rs-dapi:** improve test coverage for rs-dapi-client ([#3314](https://github.com/dashpay/platform/issues/3314))
* **rs-drive-proof-verifier:** improve test coverage ([#3355](https://github.com/dashpay/platform/issues/3355))
* **rs-sdk:** replace print-based DPNS tests with assertions, add identity error paths ([#3131](https://github.com/dashpay/platform/issues/3131))
* **sdk:** fix functional tests for local network and token config update ([#3218](https://github.com/dashpay/platform/issues/3218))
* **swift-sdk:** add wallet UI smoke flow ([#3550](https://github.com/dashpay/platform/issues/3550))
* **swift-sdk:** remove scaffold tests and fix tautological assertions ([#3222](https://github.com/dashpay/platform/issues/3222))
* **wasm-sdk:** fix flaky functional tests during local network warmup ([#3569](https://github.com/dashpay/platform/issues/3569))
* **wasm-sdk:** remove invalid `position` from document-type root in fixtures ([#3524](https://github.com/dashpay/platform/issues/3524))


### Miscellaneous Chores

* add audits/ to .gitignore ([#3372](https://github.com/dashpay/platform/issues/3372))
* add packages/rs-platform-encryption to the container ([#3164](https://github.com/dashpay/platform/issues/3164))
* add pre-commit hooks for code quality checks ([#3194](https://github.com/dashpay/platform/issues/3194))
* apply rustfmt ([#3662](https://github.com/dashpay/platform/issues/3662))
* bump grovedb to develop (352c2f55) ([#3656](https://github.com/dashpay/platform/issues/3656))
* bump rs-tenderdash-abci to v1.5.1 ([#3534](https://github.com/dashpay/platform/issues/3534))
* bump rust-dashcore to 53130869 for coin selector mempool fix ([#3627](https://github.com/dashpay/platform/issues/3627))
* bump rust-dashcore to commit 88e8a9aa1eadce79c8177f757f6741f8a55a83f5 ([#3446](https://github.com/dashpay/platform/issues/3446))
* bump rust-dashcore to commit dda1db7a7367bb7a6a48de7f4ed79da708266460 ([#3436](https://github.com/dashpay/platform/issues/3436))
* bump rust-dashcore to f92f114b83 (Rust-only) ([#3414](https://github.com/dashpay/platform/issues/3414))
* bump rust-dashcore to v0.42-dev (428b60d) ([#3617](https://github.com/dashpay/platform/issues/3617))
* **dapi-grpc:** regenerate obj-c client for count-query doc updates ([#3631](https://github.com/dashpay/platform/issues/3631))
* **deps:** bump rust-dashcore to ca507a9 (v0.42-dev) ([#3575](https://github.com/dashpay/platform/issues/3575))
* **dpp:** remove orphaned JSON schemas and dead wasm-dpp tests ([#3470](https://github.com/dashpay/platform/issues/3470))
* **drive:** bump grovedb to 8f25b20 (adds boundaries API) ([#3389](https://github.com/dashpay/platform/issues/3389))
* fix npm audit vulnerabilities (serialize-javascript RCE, deprecated text-encoding) ([#3174](https://github.com/dashpay/platform/issues/3174))
* fix trailing whitespace and missing final newlines ([#3196](https://github.com/dashpay/platform/issues/3196))
* fix typos across codebase ([#3195](https://github.com/dashpay/platform/issues/3195))
* gitignore .claude/scheduled_tasks.lock ([#3496](https://github.com/dashpay/platform/issues/3496))
* ignore .codex/ directory ([#3546](https://github.com/dashpay/platform/issues/3546))
* ignore project-local .mcp.json ([#3584](https://github.com/dashpay/platform/issues/3584))
* let claude auto-detect base branch in pr-description skill ([#3112](https://github.com/dashpay/platform/issues/3112))
* pin rust-dashcore to feat/platform-wallet2 rev 4c8bec36
* **platform-wallet-ffi:** drop unused serde_json dep
* **platform-wallet-ffi:** layout-drift guards on IdentityKeyEntryFFI
* **platform-wallet:** add TODO for event handler listener pattern
* **platform-wallet:** apply formatter changes
* **platform-wallet:** clear Rust warnings across trusted-context / platform-wallet / -ffi
* **platform-wallet:** rustfmt pass and TODO annotations
* regenerate Cargo.lock after merging v3.1-dev
* remove unused feature-flags system contract ([#3522](https://github.com/dashpay/platform/issues/3522))
* replace unmaintained paste crate with pastey ([#3238](https://github.com/dashpay/platform/issues/3238))
* **rs-sdk-ffi:** remove unused unified module ([#3415](https://github.com/dashpay/platform/issues/3415))
* **swift-example-app:** drop loadSampleIdentities + dead Developer section
* **swift-sdk:** add rs-platform-wallet-ffi to the docker containers ([#3175](https://github.com/dashpay/platform/issues/3175))
* **swift-sdk:** bump rust-dashcore to latests revision ([#3163](https://github.com/dashpay/platform/issues/3163))
* **swift-sdk:** clean up core transactions, wallet, balance, acccounts, etc in swift sdk ([#3079](https://github.com/dashpay/platform/issues/3079))
* **swift-sdk:** dash spv FFI update to lastest version ([#3049](https://github.com/dashpay/platform/issues/3049))
* **swift-sdk:** drop x86_64 architecture support ([#3448](https://github.com/dashpay/platform/issues/3448))
* **swift-sdk:** remove dash-spv-ffi crate usage, spv is wrapped by platform-wallet ([#3644](https://github.com/dashpay/platform/issues/3644))
* **swift-sdk:** remove not planned to use tx module in swift-sdk ([#3425](https://github.com/dashpay/platform/issues/3425))
* **swift-sdk:** remove placeholder Asset Lock section from send view
* **swift-sdk:** swift and unified sdk generation rewrite ([#3401](https://github.com/dashpay/platform/issues/3401))
* update dashcore deps and add Send+Sync to ContractLookupFn
* update dashcore deps to v0.42 ([#3375](https://github.com/dashpay/platform/issues/3375))
* update generated protobuf/gRPC client files
* update rust-dashcore PR [#579](https://github.com/dashpay/platform/issues/579) ([#3403](https://github.com/dashpay/platform/issues/3403))
* update to local dashcore dependency


### Continuous Integration

* add codecov carryforward flag for shielded tests ([#3287](https://github.com/dashpay/platform/issues/3287))
* add missing scopes to PR title linter ([#3250](https://github.com/dashpay/platform/issues/3250))
* add rs-dapi to package change detection filters ([#3254](https://github.com/dashpay/platform/issues/3254))
* add Rust code coverage with Codecov ([#3189](https://github.com/dashpay/platform/issues/3189))
* add weekly CI health check with Slack notification ([#3191](https://github.com/dashpay/platform/issues/3191))
* allow Swift SDK build to run when Rust tests are skipped ([#3351](https://github.com/dashpay/platform/issues/3351))
* allow thepastaclaw fork PRs to run on macOS runner ([#3318](https://github.com/dashpay/platform/issues/3318))
* benchmark code coverage on Mac runner ([#3260](https://github.com/dashpay/platform/issues/3260))
* bump actions/cache to v5 and codecov/codecov-action to v6 ([#3620](https://github.com/dashpay/platform/issues/3620))
* cache yarn build-state and install-state ([#3521](https://github.com/dashpay/platform/issues/3521))
* consolidate formatting and clippy into workspace-wide jobs ([#3252](https://github.com/dashpay/platform/issues/3252))
* consolidate remaining per-package checks, move check-each-feature to nightly ([#3255](https://github.com/dashpay/platform/issues/3255))
* **drive:** move long-running upgrade tests to nightly schedule ([#3242](https://github.com/dashpay/platform/issues/3242))
* drop '!path' negation patterns from JS package filter ([#3592](https://github.com/dashpay/platform/issues/3592))
* exclude additional boilerplate from code coverage ([#3382](https://github.com/dashpay/platform/issues/3382))
* exclude DPP state transition boilerplate from coverage ([#3358](https://github.com/dashpay/platform/issues/3358))
* exclude draft PRs from test-suite-gate approval requirement ([#3207](https://github.com/dashpay/platform/issues/3207))
* exclude error type definitions from code coverage ([#3337](https://github.com/dashpay/platform/issues/3337))
* exclude generated/boilerplate packages from coverage ([#3354](https://github.com/dashpay/platform/issues/3354))
* exclude infrastructure files from code coverage ([#3379](https://github.com/dashpay/platform/issues/3379))
* exclude more non-unit-testable code from coverage ([#3451](https://github.com/dashpay/platform/issues/3451))
* exclude SDK integration and wallet code from coverage ([#3385](https://github.com/dashpay/platform/issues/3385))
* exclude state transition boilerplate from coverage ([#3458](https://github.com/dashpay/platform/issues/3458))
* expand codecov exclusions for non-unit-testable code ([#3444](https://github.com/dashpay/platform/issues/3444))
* gate test suite behind manual approval when code unchanged ([#3185](https://github.com/dashpay/platform/issues/3185))
* gate Ubuntu backup runners on UBUNTU_BACKUP_ENABLED variable ([#3306](https://github.com/dashpay/platform/issues/3306))
* include omitted rust packages in ci filters ([#3663](https://github.com/dashpay/platform/issues/3663))
* include shielded tests in push coverage runs ([#3262](https://github.com/dashpay/platform/issues/3262))
* kill stale gpg-agent and locks before Codecov upload on Mac runners ([#3269](https://github.com/dashpay/platform/issues/3269))
* make DockerHub login conditional on secret availability
* make ECR login and Docker-dependent jobs conditional on secret availability
* move Docker builds and test suite to nightly, trigger on version change ([#3259](https://github.com/dashpay/platform/issues/3259))
* move security audits to nightly workflow and add dev status page ([#3190](https://github.com/dashpay/platform/issues/3190))
* narrow Rust CI filters to exclude JS-only files in shared packages ([#3176](https://github.com/dashpay/platform/issues/3176))
* narrow swift-sdk-build path triggers to actual dependencies ([#3197](https://github.com/dashpay/platform/issues/3197))
* only commit gRPC cache updates on pull requests ([#3193](https://github.com/dashpay/platform/issues/3193))
* preserve Mac runner build cache with 200GB safety valve ([#3249](https://github.com/dashpay/platform/issues/3249))
* preserve Swift SDK Rust build cache ([#3632](https://github.com/dashpay/platform/issues/3632))
* prune macOS coverage artifacts
* reduce Ubuntu backup shards from 6 to 4 ([#3257](https://github.com/dashpay/platform/issues/3257))
* reduce yarn install flakiness with concurrency cap and retries ([#3519](https://github.com/dashpay/platform/issues/3519))
* remove redundant rs-sdk-ffi iOS build workflow ([#3258](https://github.com/dashpay/platform/issues/3258))
* restore GPG cleanup step before Codecov upload ([#3279](https://github.com/dashpay/platform/issues/3279))
* run all tests everywhere, remove shielded/non-shielded split ([#3297](https://github.com/dashpay/platform/issues/3297))
* run Swift SDK build only on relevant changes, after Mac tests ([#3256](https://github.com/dashpay/platform/issues/3256))
* **sccache:** gracefully degrade when S3 credentials are missing
* skip @dashevo/wasm-dpp tests on pull_request (nightly-only) ([#3593](https://github.com/dashpay/platform/issues/3593))
* skip doctests when no doc examples changed ([#3251](https://github.com/dashpay/platform/issues/3251))
* skip gRPC coverage PR comment on fork PRs ([#3136](https://github.com/dashpay/platform/issues/3136))
* skip JS build when no JS-related code changed ([#3186](https://github.com/dashpay/platform/issues/3186))
* skip JS builds when only Rust test files changed ([#3360](https://github.com/dashpay/platform/issues/3360))
* skip matrix jobs when no packages changed ([#3192](https://github.com/dashpay/platform/issues/3192))
* skip ssh2 optional crypto native build ([#3520](https://github.com/dashpay/platform/issues/3520))
* skip tests on merge if coverage matches PR run ([#3300](https://github.com/dashpay/platform/issues/3300))
* **swift-sdk:** prune orphaned FFI header subdirs in build_ios.sh ([#3666](https://github.com/dashpay/platform/issues/3666))
* **swift-sdk:** skip PR comment for forked PRs ([#3236](https://github.com/dashpay/platform/issues/3236))
* temporarily allow CI to run on ci/* branches for testing
* **tests:** add self-hosted Mac runner with Ubuntu shard fallback ([#3248](https://github.com/dashpay/platform/issues/3248))
* **tests:** replace coverage with sccache for faster sharded tests ([#3244](https://github.com/dashpay/platform/issues/3244))
* use pull_request_target for milestone assignment workflow ([#3134](https://github.com/dashpay/platform/issues/3134))
* **workflows:** migrate test image transport to ghcr


### Code Refactoring

* **address-sync:** generic AddressToBytes + narrower provider types
* consolodate Network structs and enum variants into one ([#3567](https://github.com/dashpay/platform/issues/3567))
* **dpp:** cleanup and unify JSON/Object conversion ([#3167](https://github.com/dashpay/platform/issues/3167))
* **dpp:** extract user_fee_increase from StateTransitionLike into its own trait ([#3183](https://github.com/dashpay/platform/issues/3183))
* **drive:** remove dead deduct_from_prefunded_specialized_balance dispatcher ([#3508](https://github.com/dashpay/platform/issues/3508))
* **drive:** unify AVG no-prove dispatch into a single count+sum walk ([#3690](https://github.com/dashpay/platform/issues/3690))
* nonce auto-fetch belongs in rs-sdk, not FFI
* **platform-wallet-ffi:** return ManagedIdentity handle
* **platform-wallet,swift-sdk:** identity keys store private material client-side via derivation breadcrumb
* **platform-wallet:** adopt rust-dashcore wallet event-bus API ([#3556](https://github.com/dashpay/platform/issues/3556))
* **platform-wallet:** align PlatformAddressChangeSet with runtime shape
* **platform-wallet:** apply reviewer feedback on owned-cs commits
* **platform-wallet:** apply reviewer feedback on Phase 9a-2
* **platform-wallet:** apply_changeset consumes changeset by value
* **platform-wallet:** ArcSwap event manager + balance update via events
* **platform-wallet:** B2-B4 — all mutation methods persist internally
* **platform-wallet:** carry sync watermark in PlatformAddressChangeSet
* **platform-wallet:** check BIP44 account first for asset lock tx lookup
* **platform-wallet:** clean up CoreWallet, add broadcaster
* **platform-wallet:** clean up PlatformWallet constructors and manager API
* **platform-wallet:** collapse 7+ locks into single RwLock<PlatformWalletInfo>
* **platform-wallet:** consolidate SpvRuntime and SpvWalletAdapter fields
* **platform-wallet:** delegate address sync to key-wallet, PlatformAddress in AddressProvider ([#3482](https://github.com/dashpay/platform/issues/3482))
* **platform-wallet:** diff against result.found; add address_index on entries
* **platform-wallet:** diff-based address changeset, drop iter helpers
* **platform-wallet:** drop add_to_state arg from next_address calls
* **platform-wallet:** drop dead code flagged in Phase 9b review
* **platform-wallet:** extend changeset shapes for tombstones + metadata
* **platform-wallet:** extract SpvRuntime from PlatformWalletManager
* **platform-wallet:** extract WalletPersister to wallet/persister.rs
* **platform-wallet:** gate PlatformWalletManager behind manager feature
* **platform-wallet:** generic TransactionBroadcaster in leaf types
* **platform-wallet:** group SPV events under SpvEvent enum
* **platform-wallet:** improve AssetLockManager correctness and API
* **platform-wallet:** layered identity/ + fold dashpay/ under it
* **platform-wallet:** make create_wallet_from_seed_bytes async
* **platform-wallet:** make SpvRuntime::broadcast_transaction pub(crate)
* **platform-wallet:** merge DashPayWallet into IdentityWallet<B>
* **platform-wallet:** move asset lock modules to wallet/asset_lock/
* **platform-wallet:** move BlockTime to managed_identity module
* **platform-wallet:** move broadcaster to crate root, add SpvBroadcaster
* **platform-wallet:** move CoreAddressInfo to evo-tool
* **platform-wallet:** move identity key derivation to IdentityWallet, add PR-27/28 to PLAN
* **platform-wallet:** move SPV modules to src/spv/
* **platform-wallet:** move state getters from CoreWallet to PlatformWallet
* **platform-wallet:** multi-wallet SPV + SpvRuntime constructor
* **platform-wallet:** mutation methods return changesets (Phase 9a-2)
* **platform-wallet:** persist changesets internally, add metrics to SyncResult, drop removed field
* **platform-wallet:** PlatformAddressChangeSet carries AddressFunds
* **platform-wallet:** PlatformWalletPersistence::store() returns Result
* **platform-wallet:** PR-18 — remove CoreWallet convenience wrappers
* **platform-wallet:** register_from_addresses takes two signers
* **platform-wallet:** remove 9 dead AssetLockManager methods
* **platform-wallet:** remove block_in_place from AddressProvider impl, closes [#3495](https://github.com/dashpay/platform/issues/3495)
* **platform-wallet:** remove dead FromUtxo/FundWithUtxo variants
* **platform-wallet:** remove duplicate transaction_statuses from CoreWallet
* **platform-wallet:** remove duplicate TransactionStatusChanged
* **platform-wallet:** remove flush from CorePersistenceBridge
* **platform-wallet:** remove inner wrapper, gate manager at lib.rs
* **platform-wallet:** remove manager feature flag, extract SpvSyncState
* **platform-wallet:** remove redundant network field from sub-wallets
* **platform-wallet:** remove top_ups field — history is evo-tool's concern
* **platform-wallet:** remove track_asset_lock, inline insert
* **platform-wallet:** rename identity/wallet/names.rs to dpns.rs
* **platform-wallet:** rename persistence/ to changeset/
* **platform-wallet:** rename persister methods — store/flush/load
* **platform-wallet:** rename wallet_seed_hash → wallet_id on ManagedIdentity + IdentityEntry + PrivateKeyData
* **platform-wallet:** rename WalletChangeSet to PlatformWalletChangeSet, wire key-wallet changeset
* **platform-wallet:** reorganize modules and fix review issues
* **platform-wallet:** replace broadcast channel with PlatformEventManager
* **platform-wallet:** return WalletChangeSet from update_balance and mark_instant_send_utxos
* **platform-wallet:** roll back Phase 9b-3 duplicate derivation state
* **platform-wallet:** route load through WalletPersister
* **platform-wallet:** shared persister on manager with wallet_id-aware trait
* **platform-wallet:** simplify manager to single file + clean API
* **platform-wallet:** simplify wait_for_proof and add SPV broadcast to plan
* **platform-wallet:** split asset_lock/manager.rs into build.rs and sync.rs
* **platform-wallet:** split asset_lock/sync into tracking, recovery, proof
* **platform-wallet:** split dashpay/wallet.rs by operation
* **platform-wallet:** split identity keys out of IdentityEntry
* **platform-wallet:** split identity/manager.rs by function group
* **platform-wallet:** split identity/wallet.rs by operation
* **platform-wallet:** thread WalletPersister into IdentityWallet, DashPayWallet, TokenWallet
* **platform-wallet:** type PlatformWalletPersistence errors
* **platform-wallet:** type-safe contact request keys
* **platform-wallet:** unify PlatformWalletChangeSet with key-wallet types
* **platform-wallet:** use Arc<Sdk>, revert Arc<WalletBalance> to WalletBalance
* **platform-wallet:** use dash-spv event types directly
* **platform-wallet:** use local wallet info for tx status, not DAPI
* **platform-wallet:** use OutPoint in recover_asset_lock_blocking and resolve_status
* rs-platform-wallet-ffi error framework ([#3566](https://github.com/dashpay/platform/issues/3566))
* **rs-sdk:** async AddressProvider callbacks ([#3495](https://github.com/dashpay/platform/issues/3495))
* **rs-sdk:** generic AddressProvider::Tag and iterator-returning provider methods
* **sdk:** rewrite NonceCache with LRU eviction, drift detection, and structured errors ([#3111](https://github.com/dashpay/platform/issues/3111))
* simplify derive_account_xpub to use AccountType
* structure v1 getDocuments where/order_by/having as typed proto messages ([#3654](https://github.com/dashpay/platform/issues/3654))
* **swift-example-app:** fold Platform into Settings, split Wallets from Identities
* **swift-example-app:** migrate 6 identity detail views off IdentityModel onto PersistentIdentity
* **swift-example-app:** trim redundant app-layer bookkeeping after persister callbacks
* **swift-sdk,platform-wallet:** rebuild DashPay/DPNS persistence + identity sync, drop TokenWallet ([#3564](https://github.com/dashpay/platform/issues/3564))
* **swift-sdk:** add ViewModels for address operations ([#3034](https://github.com/dashpay/platform/issues/3034))
* **swift-sdk:** consolidate keychain under org.dashfoundation.wallet, drop legacy seed/PIN/mnemonic
* **swift-sdk:** data transformers ([#3045](https://github.com/dashpay/platform/issues/3045))
* **swift-sdk:** delete HDWallet + HDWalletModels; canonicalise on PersistentWallet
* **swift-sdk:** delete IdentityModel / ContractModel / DocumentModel
* **swift-sdk:** delete TokenModel, drive TokensView off SwiftData
* **swift-sdk:** drop denormalized PersistentIdentity.walletId
* **swift-sdk:** extract key management logic into centralized KeyManager ([#3033](https://github.com/dashpay/platform/issues/3033))
* **swift-sdk:** extract validation logic ([#3042](https://github.com/dashpay/platform/issues/3042))
* **swift-sdk:** group Core Addresses by account, not per-pool
* **swift-sdk:** key management ([#3050](https://github.com/dashpay/platform/issues/3050))
* **swift-sdk:** lean down stale app and SDK layers ([#3539](https://github.com/dashpay/platform/issues/3539))
* **swift-sdk:** PlatformWalletManager as ObservableObject
* **swift-sdk:** PlatformWalletManager holds N wallets
* **swift-sdk:** redesign Persistent* tx schema and fix per-wallet tx push stall ([#3561](https://github.com/dashpay/platform/issues/3561))
* **swift-sdk:** route everything through PlatformWalletManager
* **swift-sdk:** split ensureWalletRecord into find-or-create + find-only
* **swift-sdk:** split platform addresses into PersistentPlatformAddress
* **swift-sdk:** state management ([#3051](https://github.com/dashpay/platform/issues/3051))
* **swift-sdk:** switch BLAST sync from rs-sdk-ffi to platform-wallet
* **swift-sdk:** treat warnings as errors ([#3064](https://github.com/dashpay/platform/issues/3064))

## [3.1.0-dev.1](https://github.com/dashpay/platform/compare/v3.0.1...v3.1.0-dev.1) (2026-02-18)


### ⚠ BREAKING CHANGES

* **dashmate:** differentiate service ports between networks to avoid conflicts (#3085)
* **sdk:** state transition broadcast result for Evo SDK (#3077)
* **sdk:** fix type inconsistencies across wasm-sdk and js-evo-sdk (#3047)
* **sdk:** getSignableBytes is not compatible with sign and verify (#3048)
* **platform:** update PlatformAddress encoding and HRP constants (#3059)
* **platform:** 3.0 audit report fixes (#3053)
* **sdk:** comprehensive Evo SDK refactoring (#2999)
* upgrade bincode to 2.0.1 (#2991)

### Features

* **dapi:** add method to retrieve all non-banned endpoints ([#3072](https://github.com/dashpay/platform/issues/3072))
* **dashmate:** add Tenderdash 1.6 allowlistOnly option ([#3067](https://github.com/dashpay/platform/issues/3067))
* **drive-abci:** debugging tool to replay abci requests ([#2862](https://github.com/dashpay/platform/issues/2862))
* **platform:** update PlatformAddress encoding and HRP constants ([#3059](https://github.com/dashpay/platform/issues/3059))
* **sdk:** retry the wait for result request on deadline exeeded ([#3035](https://github.com/dashpay/platform/issues/3035))
* **sdk:** state transition broadcast result for Evo SDK ([#3077](https://github.com/dashpay/platform/issues/3077))
* **sdk:** token config update JS binding ([#3038](https://github.com/dashpay/platform/issues/3038))
* **wasm:** add pre-flight check for wasm-bindgen-cli version ([#3094](https://github.com/dashpay/platform/issues/3094))


### Bug Fixes

* **dapi-grpc:** files generated outside sandbox
* **dashmate:** differentiate service ports between networks to avoid conflicts ([#3085](https://github.com/dashpay/platform/issues/3085))
* **platform:** 3.0 audit report fixes ([#3053](https://github.com/dashpay/platform/issues/3053))
* **sdk:** deserialization error due to outdated contract cache ([#3052](https://github.com/dashpay/platform/issues/3052))
* **sdk:** getSignableBytes is not compatible with sign and verify ([#3048](https://github.com/dashpay/platform/issues/3048))
* **sdk:** inconsistent document query operator ([#3039](https://github.com/dashpay/platform/issues/3039))
* **sdk:** missing `getSignedBytes` method ([#3073](https://github.com/dashpay/platform/issues/3073))
* **sdk:** outdated platfrom version in JS SDK ([#3046](https://github.com/dashpay/platform/issues/3046))
* **sdk:** prevent sized_integer_types config downgrade that breaks document ([#3071](https://github.com/dashpay/platform/issues/3071))
* **wasm-sdk:** increment address nonce in identity_create_from_addresses ([#3084](https://github.com/dashpay/platform/issues/3084))


### Tests

* **drive-abci:** suppress tracing logs in test output ([#3014](https://github.com/dashpay/platform/issues/3014))
* regenerate test vectors for v3.0.1 ([#3065](https://github.com/dashpay/platform/issues/3065))


### Code Refactoring

* **sdk:** comprehensive Evo SDK refactoring ([#2999](https://github.com/dashpay/platform/issues/2999))
* **sdk:** fix type inconsistencies across wasm-sdk and js-evo-sdk ([#3047](https://github.com/dashpay/platform/issues/3047))
* **sdk:** get rid of static trusted contexts ([#3043](https://github.com/dashpay/platform/issues/3043))


### Build System

* bump Alpine to v3.23 ([#3022](https://github.com/dashpay/platform/issues/3022))
* bump tracing-subscriber to 0.3.22 ([#3023](https://github.com/dashpay/platform/issues/3023))
* update javascript grpc-js to 1.14.3 ([#3015](https://github.com/dashpay/platform/issues/3015))
* update js webpack to 5.104.0 ([#3068](https://github.com/dashpay/platform/issues/3068))
* update rs-tenderdash-abci to v1.5.0 ([#3025](https://github.com/dashpay/platform/issues/3025))


### Miscellaneous Chores

* add shumkov as code owner for SDK packages ([#3093](https://github.com/dashpay/platform/issues/3093))
* clippy
* introduce protocol version 12 ([#3017](https://github.com/dashpay/platform/issues/3017))
* rust dashcore made a workspace dependency  ([#3062](https://github.com/dashpay/platform/issues/3062))
* **sdk:** update address HRP prefix and encoding ([#3069](https://github.com/dashpay/platform/issues/3069))
* upgrade bincode to 2.0.1 ([#2991](https://github.com/dashpay/platform/issues/2991))
* use subdir of out_dir

### [3.0.1](https://github.com/dashpay/platform/compare/v3.0.1-hotfix.4...v3.0.1) (2026-02-06)

### [3.0.1-hotfix.4](https://github.com/dashpay/platform/compare/v3.0.1-hotfix.3...v3.0.1-hotfix.4) (2026-02-05)


### ⚠ BREAKING CHANGES

* **platform:** update PlatformAddress encoding and HRP constants (#3059)
* **platform:** 3.0 audit report fixes (#3053)

### Features

* **platform:** update PlatformAddress encoding and HRP constants ([#3059](https://github.com/dashpay/platform/issues/3059))


### Bug Fixes

* **platform:** 3.0 audit report fixes ([#3053](https://github.com/dashpay/platform/issues/3053))


### Miscellaneous Chores

* update all package versions to 3.0.1-hotfix.4 ([#3060](https://github.com/dashpay/platform/issues/3060))

### [3.0.1-hotfix.3](https://github.com/dashpay/platform/compare/v3.0.0...v3.0.1-hotfix.3) (2026-02-05)


### Bug Fixes

* **dashmate:** letsencrypt renewal and dashmate doctor fixes ([#3018](https://github.com/dashpay/platform/issues/3018))


### Miscellaneous Chores

* **dashmate:** upgrade to Core 23 ([#3054](https://github.com/dashpay/platform/issues/3054))
* **release:** update changelog and bump version to 3.0.1-hotfix.3 ([#3055](https://github.com/dashpay/platform/issues/3055))

### [3.0.1-hotfix.3](https://github.com/dashpay/platform/compare/v3.0.0...v3.0.1-hotfix.3) (2026-02-05)


### Bug Fixes

* **dashmate:** letsencrypt renewal and dashmate doctor fixes ([#3018](https://github.com/dashpay/platform/issues/3018))


### Miscellaneous Chores

* **dashmate:** upgrade to Core 23 ([#3054](https://github.com/dashpay/platform/issues/3054))

### [3.0.1-hotfix.2](https://github.com/dashpay/platform/compare/v3.0.1-hotfix.1...v3.0.1-hotfix.2) (2026-02-02)


### Bug Fixes

* **dashmate:** pass --profile shortlived on letsencrypt renewal

### [3.0.1-hotfix.1](https://github.com/dashpay/platform/compare/v3.0.0...v3.0.1-hotfix.1) (2026-01-26)


### Bug Fixes

* use single quotes and preserve ctx values in merge

## [3.0.0-rc.3](https://github.com/dashpay/platform/compare/v3.0.0-rc.2...v3.0.0-rc.3) (2026-01-20)


### Features

* **dashmate:** add Let's Encrypt SSL provider support ([#3000](https://github.com/dashpay/platform/issues/3000))
* **drive:** improve error handling in merk proof extraction ([#3003](https://github.com/dashpay/platform/issues/3003))


### Bug Fixes

* **platform:** update grovedb dependency to allow for larger proof sizes ([#3005](https://github.com/dashpay/platform/issues/3005))

## [3.0.0-rc.2](https://github.com/dashpay/platform/compare/v3.0.0-rc.1...v3.0.0-rc.2) (2026-01-16)


### Bug Fixes

* **sdk:** toJSON returns empty object ([#2995](https://github.com/dashpay/platform/issues/2995))


### Code Refactoring

* **sdk:** introduce `ProRegTxLike` and `NetworkLike` types  ([#2990](https://github.com/dashpay/platform/issues/2990))

## [3.0.0-rc.1](https://github.com/dashpay/platform/compare/v3.0.0-dev.11...v3.0.0-rc.1) (2026-01-13)


### ⚠ BREAKING CHANGES

* **sdk:** typed params for state transition methods (#2932)
* **sdk:** return last meaningful error on no available addresses (#2958)

### Features

* **dpp:** add Identity new_with_input_addresses_and_keys() ([#2971](https://github.com/dashpay/platform/issues/2971))
* **drive:** add WalletUtils system data contract during initialization on devnets/local networks ([#2696](https://github.com/dashpay/platform/issues/2696))
* **drive:** update verification logic for compacted address balance changes ([#2972](https://github.com/dashpay/platform/issues/2972))
* **sdk:** add validation/tests for registerName publicKeyId parameter ([#2832](https://github.com/dashpay/platform/issues/2832))
* **sdk:** return last meaningful error on no available addresses ([#2958](https://github.com/dashpay/platform/issues/2958))


### Bug Fixes

* **drive:** not setting `keeps_history` in proof verification for DataContractCreate and DataContractUpdate ([#2980](https://github.com/dashpay/platform/issues/2980))
* **drive:** use historical path query for contracts with keeps_history=true ([#2976](https://github.com/dashpay/platform/issues/2976))
* **wasm-sdk:** support ECDSA_SECP256K1 keys in contract create/update ([#2975](https://github.com/dashpay/platform/issues/2975))


### Performance Improvements

* **sdk:** cache contracts in JS SDK ([#2978](https://github.com/dashpay/platform/issues/2978))


### Tests

* **sdk:** test sync_address_balances ([#2957](https://github.com/dashpay/platform/issues/2957))
* **wasm-sdk:** enable contract token and group check ([#2952](https://github.com/dashpay/platform/issues/2952))


### Build System

* **drive:** update rkyv  to 0.7.46 ([#2982](https://github.com/dashpay/platform/issues/2982))


### Code Refactoring

* **sdk:** dpns JS SDK methods
* **sdk:** re-use sdk methods ([#2981](https://github.com/dashpay/platform/issues/2981))
* **sdk:** typed params for state transition methods ([#2932](https://github.com/dashpay/platform/issues/2932))

## [3.0.0-dev.11](https://github.com/dashpay/platform/compare/v3.0.0-dev.10...v3.0.0-dev.11) (2026-01-08)


### ⚠ BREAKING CHANGES

* **sdk:** failed address sync on invalid proof  (#2967)
* **platform:** add block-aware credit operations to manage address balance changes (#2968)
* **platform:** enhanced fetching of compacted address balance changes (#2966)

### Features

* **platform:** add block-aware credit operations to manage address balance changes ([#2968](https://github.com/dashpay/platform/issues/2968))
* **platform:** add tests for proof verification of recent address balance changes ([#2969](https://github.com/dashpay/platform/issues/2969))
* **platform:** enhanced fetching of compacted address balance changes ([#2966](https://github.com/dashpay/platform/issues/2966))
* **platform:** remove platform version patching and state migration logic ([#2961](https://github.com/dashpay/platform/issues/2961))
* **platform:** update address expiration time from 1 day to 1 week ([#2964](https://github.com/dashpay/platform/issues/2964))
* **sdk:** return checkpoint height with `AddressSyncResult` ([#2965](https://github.com/dashpay/platform/issues/2965))


### Bug Fixes

* **rs-sdk-ffi:** auto-increment document revision in replace function ([#2960](https://github.com/dashpay/platform/issues/2960))
* **sdk:** adjust metadata freshness criteria for get_addresses_trunk_state and get_addresses_branch_state ([#2954](https://github.com/dashpay/platform/issues/2954))
* **sdk:** clamp address sync branch query depth to platform limits ([#2955](https://github.com/dashpay/platform/issues/2955))
* **sdk:** failed address sync on invalid proof  ([#2967](https://github.com/dashpay/platform/issues/2967))
* **sdk:** match `ItemWithSumItem` in `extract_balance_from_element` ([#2956](https://github.com/dashpay/platform/issues/2956))

## [3.0.0-dev.10](https://github.com/dashpay/platform/compare/v3.0.0-dev.9...v3.0.0-dev.10) (2026-01-06)


### ⚠ BREAKING CHANGES

* **platform:** clean up expired compacted address balances (#2948)

### Features

* **platform:** clean up expired compacted address balances ([#2948](https://github.com/dashpay/platform/issues/2948))


### Bug Fixes

* **dpp:** broken chain lock proof deserialization ([#2950](https://github.com/dashpay/platform/issues/2950))
* **wasm-sdk:** enable identity_update to add ECDSA_SECP256K1 and BLS12_381 keys ([#2947](https://github.com/dashpay/platform/issues/2947))

## [3.0.0-dev.7](https://github.com/dashpay/platform/compare/v3.0.0-dev.6...v3.0.0-dev.7) (2025-12-30)


### ⚠ BREAKING CHANGES

* **dashmate:** add quroum list service (#2868)

### Features

* **dashmate:** add quroum list service ([#2868](https://github.com/dashpay/platform/issues/2868))
* **platform:** sdk support for platform addresses and checkpoint fix ([#2933](https://github.com/dashpay/platform/issues/2933))
* **sdk:** support platform address state transitions in JS SDK ([#2931](https://github.com/dashpay/platform/issues/2931))


### Bug Fixes

* **drive:** failing to prove absent platform addressess ([#2934](https://github.com/dashpay/platform/issues/2934))
* **sdk:** non proved JS SDK methods ([#2871](https://github.com/dashpay/platform/issues/2871))

## [3.0.0-dev.5](https://github.com/dashpay/platform/compare/v3.0.0-dev.4...v3.0.0-dev.5) (2025-12-23)


### Bug Fixes

* **drive-abci:** verify apphash in finalize_block ([#2878](https://github.com/dashpay/platform/issues/2878))


### Build System

* upgrade yarn to latest version ([#2926](https://github.com/dashpay/platform/issues/2926))

## [3.0.0-dev.4](https://github.com/dashpay/platform/compare/v3.0.0-dev.3...v3.0.0-dev.4) (2025-12-23)


### Continuous Integration

* fix NPM publish in release ([#2923](https://github.com/dashpay/platform/issues/2923))

## [2.2.0-dev.2](https://github.com/dashpay/platform/compare/v2.1.2...v2.2.0-dev.2) (2025-11-28)


### ⚠ BREAKING CHANGES

* use identity contract keys query (#2870)
* **sdk:** cleanup JS SDK params and return types (#2858)
* **sdk:** user-friendly evo sdk params (#2856)
* **dashmate:** port conflicts with mainnet and testnet  on the same host (#2829)

### Features

* **dashmate:** add  prometheus service discovery labels ([#2818](https://github.com/dashpay/platform/issues/2818))
* **drive:** add next epoch info to GetStatusResponse ([#2847](https://github.com/dashpay/platform/issues/2847))
* **sdk:** entities for Evo SDK ([#2800](https://github.com/dashpay/platform/issues/2800))


### Bug Fixes

* **dashmate:** port conflicts with mainnet and testnet  on the same host ([#2829](https://github.com/dashpay/platform/issues/2829))
* **dpp:** desiarilization of data contract JSON with token configuration ([#2857](https://github.com/dashpay/platform/issues/2857))
* resolve a few issues in iOS example app ([#2843](https://github.com/dashpay/platform/issues/2843))
* **sdk:** fail on invalid proof ([#2864](https://github.com/dashpay/platform/issues/2864))
* **sdk:** reset SDK nonce caches after failed transitions ([#2851](https://github.com/dashpay/platform/issues/2851))


### Tests

* platform-test-suite accepts DAPI_ADDRESSES ([#2798](https://github.com/dashpay/platform/issues/2798))


### Build System

* script to configure environments for ai coding agents ([#2845](https://github.com/dashpay/platform/issues/2845))
* **sdk:** wasm-sdk remove unmaintained wee_alloc (RUSTSEC-2022-0054) ([#2844](https://github.com/dashpay/platform/issues/2844))
* use workspace version in Cargo.toml ([#2831](https://github.com/dashpay/platform/issues/2831))


### Code Refactoring

* **dashmate:** remove deprecated javascript dapi ([#2827](https://github.com/dashpay/platform/issues/2827))
* **sdk:** cleanup JS SDK params and return types ([#2858](https://github.com/dashpay/platform/issues/2858))
* **sdk:** typed identifier ([#2848](https://github.com/dashpay/platform/issues/2848))
* **sdk:** typed wasm-sdk params ([#2849](https://github.com/dashpay/platform/issues/2849))
* **sdk:** user-friendly evo sdk params ([#2856](https://github.com/dashpay/platform/issues/2856))
* use identity contract keys query ([#2870](https://github.com/dashpay/platform/issues/2870))

### [2.1.3](https://github.com/dashpay/platform/compare/v2.1.2...v2.1.3) (2025-10-29)

### Bug Fixes

* **drive:** reuse existing platform node id during operator update ([#2834](https://github.com/dashpay/platform/issues/2834))

### [2.1.2](https://github.com/dashpay/platform/compare/v2.1.1...v2.1.2) (2025-10-27)


### Bug Fixes

* **dashmate:** rs-dapi not stopped when dashmate reset --platform -f is called ([#2824](https://github.com/dashpay/platform/issues/2824))
* incorrect JS package versions ([#2823](https://github.com/dashpay/platform/issues/2823))

### [2.1.1](https://github.com/dashpay/platform/compare/v2.1.0-rc.1...v2.1.1) (2025-10-27)


### Features

* **rs-sdk:** identity keys query ([#2806](https://github.com/dashpay/platform/issues/2806))
* various swift sdk / ui improvements ([#2811](https://github.com/dashpay/platform/issues/2811))


### Bug Fixes

* **dashmate:** dapi not removed after migration to rs-dapi ([#2817](https://github.com/dashpay/platform/issues/2817))
* **dashmate:** restart for rs-dapi and envoy ([#2821](https://github.com/dashpay/platform/issues/2821))
* **swift-sdk:** make SPV C callbacks Swift 6–safe; eliminate races and TOCTOU ([#2814](https://github.com/dashpay/platform/issues/2814))


### Documentation

* **dpp:** add TokenPaymentInfo file description ([#2813](https://github.com/dashpay/platform/issues/2813))


### Tests

* **wasm-sdk:** expand data contract test coverage ([#2803](https://github.com/dashpay/platform/issues/2803))


### Miscellaneous Chores

* **release:** update changelog and version to 2.1.0 ([#2820](https://github.com/dashpay/platform/issues/2820))
* update testnet DAPI address whitelist with currently enabled masternodes ([#2816](https://github.com/dashpay/platform/issues/2816))

## [2.1.0-rc.1](https://github.com/dashpay/platform/compare/v2.1.0-dev.8...v2.1.0-rc.1) (2025-10-21)


### Features

* **swift-sdk:** swift SDK improvements ([#2809](https://github.com/dashpay/platform/issues/2809))
* **wasm-sdk:** add custom masternode address configuration ([#2805](https://github.com/dashpay/platform/issues/2805))


### Code Refactoring

* **dapi:** rewrite dapi in Rust as rs-dapi ([#2716](https://github.com/dashpay/platform/issues/2716))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.1.0-rc.1 ([#2812](https://github.com/dashpay/platform/issues/2812))
* update to groveDB 3.1 ([#2808](https://github.com/dashpay/platform/issues/2808))
* update to rust dash core v0.40.0 ([#2810](https://github.com/dashpay/platform/issues/2810))

## [2.1.0-dev.8](https://github.com/dashpay/platform/compare/v2.1.0-dev.7...v2.1.0-dev.8) (2025-10-03)


### ⚠ BREAKING CHANGES

* **platform:** creator id and improved verification of document uniqueness before insertion (#2790)

### Features

* **platform:** creator id and improved verification of document uniqueness before insertion ([#2790](https://github.com/dashpay/platform/issues/2790))
* **sdk:** expose data contract from json ([#2791](https://github.com/dashpay/platform/issues/2791))


### Bug Fixes

* **dashmate:** consensus params in dashmate different than on testnet ([#2682](https://github.com/dashpay/platform/issues/2682))
* **sdk:** wasm is not initialized for some methods ([#2792](https://github.com/dashpay/platform/issues/2792))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.1.0-dev.8 ([#2797](https://github.com/dashpay/platform/issues/2797))
* script to backup and restore state

### [2.0.1](https://github.com/dashpay/platform/compare/v2.0.0...v2.0.1) (2025-07-10)


### ⚠ BREAKING CHANGES

* **platform:** update keyword search contract ID and owner ID bytes (#2693)

### Bug Fixes

* **platform:** update keyword search contract ID and owner ID bytes ([#2693](https://github.com/dashpay/platform/issues/2693))


### Miscellaneous Chores

* release version 2.0.1 ([#2695](https://github.com/dashpay/platform/issues/2695))

## [2.1.0-dev.7](https://github.com/dashpay/platform/compare/v2.1.0-dev.6...v2.1.0-dev.7) (2025-09-29)


### Bug Fixes

* **sdk:** wasm sdk is not initialized for static methods ([#2788](https://github.com/dashpay/platform/issues/2788))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.1.0-dev.7 ([#2789](https://github.com/dashpay/platform/issues/2789))

## [2.1.0-dev.6](https://github.com/dashpay/platform/compare/v2.1.0-dev.5...v2.1.0-dev.6) (2025-09-24)


### Features

* **drive:** document filter for state transition subscriptions part 2 ([#2781](https://github.com/dashpay/platform/issues/2781))
* **sdk:** add more SDK methods ([#2784](https://github.com/dashpay/platform/issues/2784))


### Bug Fixes

* **dashmate:** incompatible tenderdash version ([#2786](https://github.com/dashpay/platform/issues/2786))


### Performance Improvements

* **rs-sdk:** optimize wasm-sdk bundle size ([#2783](https://github.com/dashpay/platform/issues/2783))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.1.0-dev.6 ([#2785](https://github.com/dashpay/platform/issues/2785))

## [2.1.0-dev.5](https://github.com/dashpay/platform/compare/v2.1.0-dev.4...v2.1.0-dev.5) (2025-09-19)


### Features

* **drive:** document filter for state transition subscriptions part 1 ([#2761](https://github.com/dashpay/platform/issues/2761))


### Build System

* fix sdk npm packaging ([#2780](https://github.com/dashpay/platform/issues/2780))


### Miscellaneous Chores

* **release:** update changelog and bump version to 2.1.0-dev.5 ([#2782](https://github.com/dashpay/platform/issues/2782))

## [2.1.0-dev.4](https://github.com/dashpay/platform/compare/v2.0.1...v2.1.0-dev.4) (2025-09-19)


### ⚠ BREAKING CHANGES

* **wasm-sdk:**  handle identity create transition signing for all types of keys (#2754)
* **wasm-sdk:** remove unused key_id parameters from state transitions (#2759)
* **sdk:** provide all getStatus info (#2729)

### Features

* add tests for new token transitions
* add wasm bindings for Drive verification functions ([#2660](https://github.com/dashpay/platform/issues/2660))
* balance checker app ([#2688](https://github.com/dashpay/platform/issues/2688))
* **dashmate:** allow configuring zmq using dashmate ([#2697](https://github.com/dashpay/platform/issues/2697))
* evo sdk ([#2771](https://github.com/dashpay/platform/issues/2771))
* **sdk:** add request settings in wasm sdk ([#2707](https://github.com/dashpay/platform/issues/2707))
* **sdk:** add username search example in evo-sdk ([#2706](https://github.com/dashpay/platform/issues/2706))
* **sdk:** adding a trusted context provider package ([#2687](https://github.com/dashpay/platform/issues/2687))
* **sdk:** dpns sdk improvements ([#2692](https://github.com/dashpay/platform/issues/2692))
* **sdk:** enable proof support for most queries ([#2718](https://github.com/dashpay/platform/issues/2718))
* **sdk:** epic: rs-sdk-ffi and ios support ([#2756](https://github.com/dashpay/platform/issues/2756))
* **sdk:** identity creation in wasm ([#2711](https://github.com/dashpay/platform/issues/2711))
* **sdk:** make wasm sdk complete for all state transitions and most queries ([#2690](https://github.com/dashpay/platform/issues/2690))
* **sdk:** provide all getStatus info ([#2729](https://github.com/dashpay/platform/issues/2729))
* **sdk:** wasm docs and fixes ([#2700](https://github.com/dashpay/platform/issues/2700))
* **sdk:** wasm drive verify optimization ([#2683](https://github.com/dashpay/platform/issues/2683))
* **sdk:** wasm sdk core and test suite ([#2709](https://github.com/dashpay/platform/issues/2709))
* **wasm-sdk:** implement four missing token transitions
* **wasm-sdk:** remove unused key_id parameters from state transitions ([#2759](https://github.com/dashpay/platform/issues/2759))


### Bug Fixes

* **sdk:** fix documentation examples ([#2710](https://github.com/dashpay/platform/issues/2710))
* **sdk:** fix generate docs ([#2730](https://github.com/dashpay/platform/issues/2730))
* **sdk:** install wasm-opt from Github instead of apt ([#2701](https://github.com/dashpay/platform/issues/2701))
* **sdk:** js sdk audit warnings by adding crypto-related dependencies to package.json ([#2757](https://github.com/dashpay/platform/issues/2757))
* **sdk:** modifications to get wasm-sdk working again ([#2689](https://github.com/dashpay/platform/issues/2689))
* **wasm-sdk:**  handle identity create transition signing for all types of keys ([#2754](https://github.com/dashpay/platform/issues/2754))
* **wasm-sdk:** address compiler warnings ([#2734](https://github.com/dashpay/platform/issues/2734))
* **wasm-sdk:** connect where and orderBy clause functionality for getDocuments ([#2753](https://github.com/dashpay/platform/issues/2753))
* **wasm-sdk:** enable proofs for getContestedResourceVotersForIdentity ([#2732](https://github.com/dashpay/platform/issues/2732))
* **wasm-sdk:** fix nft transitions ([#2751](https://github.com/dashpay/platform/issues/2751))
* **wasm-sdk:** resolve CI test failures and build issues ([#2765](https://github.com/dashpay/platform/issues/2765))
* **wasm-sdk:** resolve test failures and optimize CI workflow ([#2735](https://github.com/dashpay/platform/issues/2735))
* **wasm-sdk:** use identity contract nonce for data contract updates ([#2738](https://github.com/dashpay/platform/issues/2738))


### Tests

* **sdk:** automate wasm-sdk page UI testing (partial) ([#2715](https://github.com/dashpay/platform/issues/2715))
* **sdk:** expand wasm-sdk page UI testing ([#2720](https://github.com/dashpay/platform/issues/2720))
* **wasm-sdk:** add ui tests for almost all state transitions ([#2739](https://github.com/dashpay/platform/issues/2739))


### Build System

* bump tenderdash-abci to v1.5.0-dev.2 ([#2770](https://github.com/dashpay/platform/issues/2770))
* update rust to 1.89 ([#2755](https://github.com/dashpay/platform/issues/2755))


### Code Refactoring

* **sdk:** wasm-sdk doc generation refactor ([#2726](https://github.com/dashpay/platform/issues/2726))
* swift sdk fixes ([#2772](https://github.com/dashpay/platform/issues/2772))
* **wasm-sdk:** improve documentation generation maintainability ([#2773](https://github.com/dashpay/platform/issues/2773))


### Continuous Integration

* dont do CI when it's not needed ([#2774](https://github.com/dashpay/platform/issues/2774))
* swift CI fixes ([#2775](https://github.com/dashpay/platform/issues/2775))
* Use self hosted mac runner ([#2776](https://github.com/dashpay/platform/issues/2776))


### Miscellaneous Chores

* add wasm-sdk as scope for pr linting ([#2731](https://github.com/dashpay/platform/issues/2731))
* clean dpp clippy ([#2764](https://github.com/dashpay/platform/issues/2764))
* **drive:** fix drive linting ([#2763](https://github.com/dashpay/platform/issues/2763))
* **platform:** add protocol version 10 support ([#2686](https://github.com/dashpay/platform/issues/2686))
* **release:** update changelog and bump version to 2.1.0-dev.4 ([#2779](https://github.com/dashpay/platform/issues/2779))
* sdk clippy issues ([#2767](https://github.com/dashpay/platform/issues/2767))
* **sdk:** use correct port for evo-sdk mainnet ([#2699](https://github.com/dashpay/platform/issues/2699))
* update yarn cache with new dependencies ([#2758](https://github.com/dashpay/platform/issues/2758))
* **wasm-sdk:** apply cargo fmt and cleanup ([#2766](https://github.com/dashpay/platform/issues/2766))

## [2.1.0](https://github.com/dashpay/platform/compare/v2.1.0-rc.1...v2.1.0) (2025-10-24)


### ⚠ BREAKING CHANGES

* **platform:** creator id and improved verification of document uniqueness before insertion (#2790)
* **wasm-sdk:**  handle identity create transition signing for all types of keys (#2754)
* **wasm-sdk:** remove unused key_id parameters from state transitions (#2759)
* **sdk:** provide all getStatus info (#2729)


### Features

* **swift-sdk:** swift SDK improvements ([#2809](https://github.com/dashpay/platform/issues/2809))
* **wasm-sdk:** add custom masternode address configuration ([#2805](https://github.com/dashpay/platform/issues/2805))
* **platform:** creator id and improved verification of document uniqueness before insertion ([#2790](https://github.com/dashpay/platform/issues/2790))
* **sdk:** expose data contract from json ([#2791](https://github.com/dashpay/platform/issues/2791))
* **drive:** document filter for state transition subscriptions part 2 ([#2781](https://github.com/dashpay/platform/issues/2781))
* **sdk:** add more SDK methods ([#2784](https://github.com/dashpay/platform/issues/2784))
* **drive:** document filter for state transition subscriptions part 1 ([#2761](https://github.com/dashpay/platform/issues/2761))
* add tests for new token transitions
* evo sdk ([#2771](https://github.com/dashpay/platform/issues/2771))
* **sdk:** epic: rs-sdk-ffi and ios support ([#2756](https://github.com/dashpay/platform/issues/2756))
* **sdk:** provide all getStatus info ([#2729](https://github.com/dashpay/platform/issues/2729))
* **wasm-sdk:** implement four missing token transitions
* **wasm-sdk:** remove unused key_id parameters from state transitions ([#2759](https://github.com/dashpay/platform/issues/2759))
* **dapi:** access logging

### Bug Fixes

* **dashmate:** dapi not removed after migration to rs-dapi ([#2817](https://github.com/dashpay/platform/issues/2817))
* **swift-sdk:** make SPV C callbacks Swift 6–safe; eliminate races and TOCTOU ([#2814](https://github.com/dashpay/platform/issues/2814))
* **dashmate:** consensus params in dashmate different than on testnet ([#2682](https://github.com/dashpay/platform/issues/2682))
* **sdk:** wasm is not initialized for some methods ([#2792](https://github.com/dashpay/platform/issues/2792))
* **dashmate:** incompatible tenderdash version ([#2786](https://github.com/dashpay/platform/issues/2786))
* **sdk:** wasm sdk is not initialized for static methods ([#2788](https://github.com/dashpay/platform/issues/2788))
* **sdk:** fix generate docs ([#2730](https://github.com/dashpay/platform/issues/2730))
* **sdk:** js sdk audit warnings by adding crypto-related dependencies to package.json ([#2757](https://github.com/dashpay/platform/issues/2757))
* **wasm-sdk:**  handle identity create transition signing for all types of keys ([#2754](https://github.com/dashpay/platform/issues/2754))
* **wasm-sdk:** address compiler warnings ([#2734](https://github.com/dashpay/platform/issues/2734))
* **wasm-sdk:** connect where and orderBy clause functionality for getDocuments ([#2753](https://github.com/dashpay/platform/issues/2753))
* **wasm-sdk:** enable proofs for getContestedResourceVotersForIdentity ([#2732](https://github.com/dashpay/platform/issues/2732))
* **wasm-sdk:** fix nft transitions ([#2751](https://github.com/dashpay/platform/issues/2751))
* **wasm-sdk:** resolve CI test failures and build issues ([#2765](https://github.com/dashpay/platform/issues/2765))
* **wasm-sdk:** resolve test failures and optimize CI workflow ([#2735](https://github.com/dashpay/platform/issues/2735))
* **wasm-sdk:** use identity contract nonce for data contract updates ([#2738](https://github.com/dashpay/platform/issues/2738))


### Miscellaneous Chores

* update to groveDB 3.1 ([#2808](https://github.com/dashpay/platform/issues/2808))
* update to rust dash core v0.40.0 ([#2810](https://github.com/dashpay/platform/issues/2810))
* script to backup and restore state
* add wasm-sdk as scope for pr linting ([#2731](https://github.com/dashpay/platform/issues/2731))
* clean dpp clippy ([#2764](https://github.com/dashpay/platform/issues/2764))
* **drive:** fix drive linting ([#2763](https://github.com/dashpay/platform/issues/2763))
* sdk clippy issues ([#2767](https://github.com/dashpay/platform/issues/2767))
* update yarn cache with new dependencies ([#2758](https://github.com/dashpay/platform/issues/2758))
* **wasm-sdk:** apply cargo fmt and cleanup ([#2766](https://github.com/dashpay/platform/issues/2766))
* fix wasm-sdk build
* getrandom downgrade continued
* getrandom downgrade, continued
* **release:** update changelog and version to 2.1.0-dev.3
* trying to build
* update some deps
* wasm-sdk deps update


### Code Refactoring

* **dapi:** rewrite dapi in Rust as rs-dapi ([#2716](https://github.com/dashpay/platform/issues/2716))
* **sdk:** wasm-sdk doc generation refactor ([#2726](https://github.com/dashpay/platform/issues/2726))
* swift sdk fixes ([#2772](https://github.com/dashpay/platform/issues/2772))
* **wasm-sdk:** improve documentation generation maintainability ([#2773](https://github.com/dashpay/platform/issues/2773))


### Performance Improvements

* **rs-sdk:** optimize wasm-sdk bundle size ([#2783](https://github.com/dashpay/platform/issues/2783))


### Build System

* fix sdk npm packaging ([#2780](https://github.com/dashpay/platform/issues/2780))
* add version param to release a specific version
* bump tenderdash-abci to v1.5.0-dev.2 ([#2770](https://github.com/dashpay/platform/issues/2770))
* update rust to 1.89 ([#2755](https://github.com/dashpay/platform/issues/2755))
* **deps:** update getrandom to v0.3


### Tests

* **sdk:** expand wasm-sdk page UI testing ([#2720](https://github.com/dashpay/platform/issues/2720))
* **wasm-sdk:** add ui tests for almost all state transitions ([#2739](https://github.com/dashpay/platform/issues/2739))


### Continuous Integration

* dont do CI when it's not needed ([#2774](https://github.com/dashpay/platform/issues/2774))
* swift CI fixes ([#2775](https://github.com/dashpay/platform/issues/2775))
* Use self hosted mac runner ([#2776](https://github.com/dashpay/platform/issues/2776))
* rs-dapi workflows


## [2.1.0-rc.1](https://github.com/dashpay/platform/compare/v2.1.0-dev.8...v2.1.0-rc.1) (2025-10-21)


### Features

* **swift-sdk:** swift SDK improvements ([#2809](https://github.com/dashpay/platform/issues/2809))
* **wasm-sdk:** add custom masternode address configuration ([#2805](https://github.com/dashpay/platform/issues/2805))


### Miscellaneous Chores

* update to groveDB 3.1 ([#2808](https://github.com/dashpay/platform/issues/2808))
* update to rust dash core v0.40.0 ([#2810](https://github.com/dashpay/platform/issues/2810))


### Code Refactoring

* **dapi:** rewrite dapi in Rust as rs-dapi ([#2716](https://github.com/dashpay/platform/issues/2716))


## [2.1.0-dev.8](https://github.com/dashpay/platform/compare/v2.1.0-dev.7...v2.1.0-dev.8) (2025-10-03)


### ⚠ BREAKING CHANGES

* **platform:** creator id and improved verification of document uniqueness before insertion (#2790)

### Features

* **platform:** creator id and improved verification of document uniqueness before insertion ([#2790](https://github.com/dashpay/platform/issues/2790))
* **sdk:** expose data contract from json ([#2791](https://github.com/dashpay/platform/issues/2791))


### Bug Fixes

* **dashmate:** consensus params in dashmate different than on testnet ([#2682](https://github.com/dashpay/platform/issues/2682))
* **sdk:** wasm is not initialized for some methods ([#2792](https://github.com/dashpay/platform/issues/2792))


### Miscellaneous Chores

* script to backup and restore state


## [2.1.0-dev.7](https://github.com/dashpay/platform/compare/v2.1.0-dev.6...v2.1.0-dev.7) (2025-09-29)


### Bug Fixes

* **sdk:** wasm sdk is not initialized for static methods ([#2788](https://github.com/dashpay/platform/issues/2788))

## [2.1.0-dev.6](https://github.com/dashpay/platform/compare/v2.1.0-dev.5...v2.1.0-dev.6) (2025-09-24)


### Features

* **drive:** document filter for state transition subscriptions part 2 ([#2781](https://github.com/dashpay/platform/issues/2781))
* **sdk:** add more SDK methods ([#2784](https://github.com/dashpay/platform/issues/2784))

### Bug Fixes

* **dashmate:** incompatible tenderdash version ([#2786](https://github.com/dashpay/platform/issues/2786))


### Performance Improvements

* **rs-sdk:** optimize wasm-sdk bundle size ([#2783](https://github.com/dashpay/platform/issues/2783))

## [2.1.0-dev.5](https://github.com/dashpay/platform/compare/v2.1.0-dev.4...v2.1.0-dev.5) (2025-09-19)

### Features

* **drive:** document filter for state transition subscriptions part 1 ([#2761](https://github.com/dashpay/platform/issues/2761))


### Build System

* fix sdk npm packaging ([#2780](https://github.com/dashpay/platform/issues/2780))

## [2.1.0-dev.4](https://github.com/dashpay/platform/compare/v2.0.0...v2.1.0-dev.4) (2025-09-18)


### ⚠ BREAKING CHANGES

* **wasm-sdk:**  handle identity create transition signing for all types of keys (#2754)
* **wasm-sdk:** remove unused key_id parameters from state transitions (#2759)
* **sdk:** provide all getStatus info (#2729)

### Features

* add tests for new token transitions
* evo sdk ([#2771](https://github.com/dashpay/platform/issues/2771))
* **sdk:** epic: rs-sdk-ffi and ios support ([#2756](https://github.com/dashpay/platform/issues/2756))
* **sdk:** provide all getStatus info ([#2729](https://github.com/dashpay/platform/issues/2729))
* **wasm-sdk:** implement four missing token transitions
* **wasm-sdk:** remove unused key_id parameters from state transitions ([#2759](https://github.com/dashpay/platform/issues/2759))


### Bug Fixes

* **sdk:** fix generate docs ([#2730](https://github.com/dashpay/platform/issues/2730))
* **sdk:** js sdk audit warnings by adding crypto-related dependencies to package.json ([#2757](https://github.com/dashpay/platform/issues/2757))
* **wasm-sdk:**  handle identity create transition signing for all types of keys ([#2754](https://github.com/dashpay/platform/issues/2754))
* **wasm-sdk:** address compiler warnings ([#2734](https://github.com/dashpay/platform/issues/2734))
* **wasm-sdk:** connect where and orderBy clause functionality for getDocuments ([#2753](https://github.com/dashpay/platform/issues/2753))
* **wasm-sdk:** enable proofs for getContestedResourceVotersForIdentity ([#2732](https://github.com/dashpay/platform/issues/2732))
* **wasm-sdk:** fix nft transitions ([#2751](https://github.com/dashpay/platform/issues/2751))
* **wasm-sdk:** resolve CI test failures and build issues ([#2765](https://github.com/dashpay/platform/issues/2765))
* **wasm-sdk:** resolve test failures and optimize CI workflow ([#2735](https://github.com/dashpay/platform/issues/2735))
* **wasm-sdk:** use identity contract nonce for data contract updates ([#2738](https://github.com/dashpay/platform/issues/2738))


### Tests

* **sdk:** expand wasm-sdk page UI testing ([#2720](https://github.com/dashpay/platform/issues/2720))
* **wasm-sdk:** add ui tests for almost all state transitions ([#2739](https://github.com/dashpay/platform/issues/2739))


### Miscellaneous Chores

* add wasm-sdk as scope for pr linting ([#2731](https://github.com/dashpay/platform/issues/2731))
* clean dpp clippy ([#2764](https://github.com/dashpay/platform/issues/2764))
* **drive:** fix drive linting ([#2763](https://github.com/dashpay/platform/issues/2763))
* sdk clippy issues ([#2767](https://github.com/dashpay/platform/issues/2767))
* update yarn cache with new dependencies ([#2758](https://github.com/dashpay/platform/issues/2758))
* **wasm-sdk:** apply cargo fmt and cleanup ([#2766](https://github.com/dashpay/platform/issues/2766))


### Code Refactoring

* **sdk:** wasm-sdk doc generation refactor ([#2726](https://github.com/dashpay/platform/issues/2726))
* swift sdk fixes ([#2772](https://github.com/dashpay/platform/issues/2772))
* **wasm-sdk:** improve documentation generation maintainability ([#2773](https://github.com/dashpay/platform/issues/2773))


### Continuous Integration

* dont do CI when it's not needed ([#2774](https://github.com/dashpay/platform/issues/2774))
* swift CI fixes ([#2775](https://github.com/dashpay/platform/issues/2775))
* Use self hosted mac runner ([#2776](https://github.com/dashpay/platform/issues/2776))


### Build System

* add version param to release a specific version
* bump tenderdash-abci to v1.5.0-dev.2 ([#2770](https://github.com/dashpay/platform/issues/2770))
* update rust to 1.89 ([#2755](https://github.com/dashpay/platform/issues/2755))

## [2.1.0-dev.3](https://github.com/dashpay/platform/compare/v2.1.0-dev.2...v2.1.0-dev.3) (2025-08-07)


### Miscellaneous Chores

* fix wasm-sdk build
* getrandom downgrade continued
* getrandom downgrade, continued
* **release:** update changelog and version to 2.1.0-dev.3
* trying to build
* update some deps
* wasm-sdk deps update

## [2.1.0-dev.2](https://github.com/dashpay/platform/compare/v2.1.0-dev.1...v2.1.0-dev.2) (2025-08-06)


### Features

* access logging


### Build System

* **deps:** update getrandom to v0.3


### Continuous Integration

* rs-dapi workflows


### Miscellaneous Chores

* at least compiles
* better logging
* cargo.lock version
* cargo.toml reorder packages
* cleanup deps
* clippy
* copy rs-dapi
* dashmate impl
* DESIGN - logging described
* disable access log (doesn't work anyway)
* example apps
* fix env var name
* identity create green
* improve logging
* minor fixes
* move old dapi to /deprecated prefix
* progress, tenderdash to do
* refactor of td client and websockets
* **release:** update changelog and version to 2.1.0-dev.2
* replace sync zmq with async zeromq
* rs-dapi verbose entrypoint
* rs-dapi, wip
* some logs
* tracing logging
* try to fix logging
* wip
* wip
* wip
* zeromq improvements
* zmq
* zmq details
* zmq reconnecting
* zmq to test

## [2.1.0-dev.1](https://github.com/dashpay/platform/compare/v2.0.1...v2.1.0-dev.1) (2025-07-11)

### Miscellaneous Chores

* **release:** update changelog and version to 2.1.0-dev.1


### [2.0.1](https://github.com/dashpay/platform/compare/v2.0.0...v2.0.1) (2025-07-10)


### ⚠ BREAKING CHANGES

* **platform:** update keyword search contract ID and owner ID bytes (#2693)

### Bug Fixes

* **platform:** update keyword search contract ID and owner ID bytes ([#2693](https://github.com/dashpay/platform/issues/2693))


### Miscellaneous Chores

* release version 2.0.1 ([#2695](https://github.com/dashpay/platform/issues/2695))


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
