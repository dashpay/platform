## [0.19.2](https://github.com/dashevo/js-dpp/compare/v0.19.1...v0.19.2) (2021-05-20)


### Bug Fixes

* Cbor not decoding buffers properly in browsers ([#306](https://github.com/dashevo/js-dpp/issues/308))



## [0.19.1](https://github.com/dashevo/js-dpp/compare/v0.19.0...v0.19.1) (2021-05-04)


### Bug Fixes

* `topLevelIdentity.getId` is not a function ([#306](https://github.com/dashevo/js-dpp/issues/306))



# [0.19.0](https://github.com/dashevo/js-dpp/compare/v0.18.0...v0.19.0) (2021-04-30)


### Features

* add data triggers for feature flags documents ([#297](https://github.com/dashevo/js-dpp/issues/297), [#302](https://github.com/dashevo/js-dpp/issues/302))
* ChainLock Asset Lock proof ([#296](https://github.com/dashevo/js-dpp/issues/296))
* use `verifyInstantLock` instead of `fetchSMLStore` ([#294](https://github.com/dashevo/js-dpp/issues/294))


### BREAKING CHANGES

* `AssetLock` class was removed.
* `InstantAssetLockProof` requires `transaction` and `outputIndex` property.
* `IdentityCreateTransition` schema changed. `assetLock` property renamed to `assetLockProof` and expect `InstantAssetLockProof` or `ChainAssetLockProof`. `transaction` and `outputIndex` properties are removed.
* `IdentityTopUpTransition` schema changed. `assetLock` property renamed to `assetLockProof` and expect `InstantAssetLockProof` or `ChainAssetLockProof`. `transaction` and `outputIndex` properties are removed.



# [0.18.0](https://github.com/dashevo/js-dpp/compare/v0.17.0...v0.18.0) (2021-03-03)


### Features

* get modified data ids from state transitions ([#290](https://github.com/dashevo/js-dpp/issues/290))


### Bug Fixes

* BLS was throwing an error inside `uncaughtException` handler ([#293](https://github.com/dashevo/js-dpp/issues/293))



# [0.17.0](https://github.com/dashevo/js-dpp/compare/v0.16.0...v0.17.0) (2020-12-29)


### Features

* dashpay data triggers ([#285](https://github.com/dashevo/js-dpp/issues/285))
* update dashcore-lib ([#271](https://github.com/dashevo/js-dpp/issues/271), [#283](https://github.com/dashevo/js-dpp/issues/283), [#287](https://github.com/dashevo/js-dpp/issues/287))
* fund identity with Asset Lock Proofs ([#276](https://github.com/dashevo/js-dpp/issues/276), [#277](https://github.com/dashevo/js-dpp/issues/277), [#280](https://github.com/dashevo/js-dpp/issues/280))
* limit publicKeys items to 32 ([#278](https://github.com/dashevo/js-dpp/issues/278))


### Bug Fixes

* fs not found error on deploy ([#273](https://github.com/dashevo/js-dpp/issues/273))


### BREAKING CHANGES

* Identity Create and Topup Transitions expect Asset Lock object instead of asset lock outpoint
* `identity.create` and `identity.createIdentityTopUpTransition` expect asset lock transaction, output, and proof instead of outpoint
* renamed `skipAssetLockConfirmationValidation` DPP option to `skipAssetLockProofSignatureVerification`
* identity allows only 32 public keys



# [0.16.0](https://github.com/dashevo/js-dpp/compare/v0.15.0...v0.16.0) (2020-10-26)


### Features

* use Buffers for binary data ([#238](https://github.com/dashevo/js-dpp/issues/238), [#240](https://github.com/dashevo/js-dpp/issues/240), [#241](https://github.com/dashevo/js-dpp/issues/241), [#246](https://github.com/dashevo/js-dpp/issues/246), [#247](https://github.com/dashevo/js-dpp/issues/247), [#261](https://github.com/dashevo/js-dpp/issues/261), [#262](https://github.com/dashevo/js-dpp/issues/262), [#263](https://github.com/dashevo/js-dpp/issues/263), [#266](https://github.com/dashevo/js-dpp/issues/266))
* Identifier property type ([#252](https://github.com/dashevo/js-dpp/issues/252), [#265](https://github.com/dashevo/js-dpp/issues/265), [#267](https://github.com/dashevo/js-dpp/issues/267), [#268](https://github.com/dashevo/js-dpp/issues/268))
* `byteArray` JSON Schema keyword instead of `contentEncoding` ([#245](https://github.com/dashevo/js-dpp/issues/245), [#248](https://github.com/dashevo/js-dpp/issues/248), [#251](https://github.com/dashevo/js-dpp/issues/251), [#254](https://github.com/dashevo/js-dpp/issues/254), [#260]((https://github.com/dashevo/js-dpp/issues/260)))
* use 32 random bytes instead of blockchain address for entropy ([#250](https://github.com/dashevo/js-dpp/issues/250), [#259](https://github.com/dashevo/js-dpp/issues/259))
* validate and store all identity keys instead of the first one ([#234](https://github.com/dashevo/js-dpp/issues/234), [#237], [#242](https://github.com/dashevo/js-dpp/issues/242))
* validate document upon creation ([#255](https://github.com/dashevo/js-dpp/issues/255))
* hash methods responds with Buffer ([#249](https://github.com/dashevo/js-dpp/issues/249))
* introduce a BLS identity key type ([#239](https://github.com/dashevo/js-dpp/issues/239))
* add revision property to identity ([#235](https://github.com/dashevo/js-dpp/issues/235))
* `isEnabled` property removed from Identity Public Key [#236](https://github.com/dashevo/js-dpp/issues/236)


### BREAKING CHANGES

* Node.JS 10 and lower are not supported
* data models use Buffers instead of strings for binary fields
* `serialize` methods renamed to `toBuffer`
* `createFromSerialized` methods renamed to `createFromBuffer`
* `StateRepository` accept `Identifier` and `Buffer` instead of strings
* identifiers like document, data contract and identity IDs are instances `Identifier` (compatible with `Buffer`)
* `contentEncoding` keyword isn't supported anymore. Use `byteArray: true` with `type: array` to store binary data
* Data Contract and Document entropy is now a random 32 bytes instead of blockchain address
* identity and identity create transition can't contain duplicate public keys anymore
* `DocumentFactory#create` throws an error if specified data is not valid
* `hash` methods respond with `Buffer` instead of hex encoded string
* ECDSA Public key (type `0`) must be a 33 long byte array.
* Identity's `revision` is required
* Identity Public Key's `isEnabled` is not accepted
* Data created or serialized by previous is incompatible



# [0.15.0](https://github.com/dashevo/js-dpp/compare/v0.14.0...v0.15.0) (2020-09-04)


### Features

* protocol versioning ([#217](https://github.com/dashevo/js-dpp/issues/217))
* document binary properties ([#199](https://github.com/dashevo/js-dpp/issues/199), [#211](https://github.com/dashevo/js-dpp/issues/211), [#215](https://github.com/dashevo/js-dpp/issues/215), [#218](https://github.com/dashevo/js-dpp/issues/218), [#213](https://github.com/dashevo/js-dpp/issues/213))
* handle unique and alias identities in DPNS data triggers ([#201](https://github.com/dashevo/js-dpp/issues/213))
* add data trigger condition to check allowing subdomain rules ([#224](https://github.com/dashevo/js-dpp/issues/224), [#228](https://github.com/dashevo/js-dpp/pull/228))
* reject `replace` and `delete` actions for DPNS preorder document ([#210](https://github.com/dashevo/js-dpp/issues/224))


### Bug Fixes

* empty where conditions were sent during unique indices validation ([#222](https://github.com/dashevo/js-dpp/issues/222))
* duplicate key error in case of unique index on optional fields ([#230](https://github.com/dashevo/js-dpp/pull/230))
* invalid arguments were submitted to search for parent domain ([#226](https://github.com/dashevo/js-dpp/issues/226))
* invalid where clause was sent, invalid query error was not handled by unique index validation method ([#220](https://github.com/dashevo/js-dpp/issues/220))
* undefined in data contract schema id ([#209](https://github.com/dashevo/js-dpp/issues/209))
* data contract fixture was not isolated properly ([#207](https://github.com/dashevo/js-dpp/issues/207))
* schema with key or id already exists ([#203](https://github.com/dashevo/js-dpp/issues/203))


### BREAKING CHANGES

* `protocolVersion` property equals to `0` is required for all data structures
* `Document` now awaits `DataContract` as a second argument in constructor
* `DocumentsBatchTransition` now awaits `DataContract` as a second argument in constructor
* a document compound unique index shouldn't contain both required and optional properties
* a document with a compound unique index must contain all indexed properties or non of them
* only second-level DPNS domain owner is allowed to create its subdomains
* DPNS preorder document is immutable now. Modification and deletion of preorder are restricted.
* `getDocumentsFixture.dataContract` is not available anymore
* DPNS data trigger expect `dashUniqueIdentityId` and `dashAliasIdentityId` records, instead of oboslete `dashIdentity`



# [0.14.0](https://github.com/dashevo/js-dpp/compare/v0.13.1...v0.14.0) (2020-07-22)


### Bug Fixes

* missing indexed string property constraint validation ([#196](https://github.com/dashevo/js-dpp/issues/196))
* error when the indexed field has an undefined value ([#194](https://github.com/dashevo/js-dpp/issues/194))
* conflicting schema ids in AJV cache ([#187](https://github.com/dashevo/js-dpp/issues/187))


### Features

* add `createdAt` and `updatedAt` timestamps to Document ([#192](https://github.com/dashevo/js-dpp/issues/192))
* disable unsupported JSON Schema conditions ([#193](https://github.com/dashevo/js-dpp/issues/193))


### Documentation

* readme standard updates ([#189](https://github.com/dashevo/js-dpp/issues/189))


### BREAKING CHANGES

* Indexed strings should have `maxLength` constraint not greater than 1024 chars
* JSON Schema conditions (`allOf`, `if`, ...) are not allowed in Document JSON Schema



## [0.13.1](https://github.com/dashevo/js-dpp/compare/v0.13.0...v0.13.1) (2020-06-15)


### Bug Fixes

* conflicting schema ids in AJV cache ([#187](https://github.com/dashevo/js-dpp/issues/187))



# [0.13.0](https://github.com/dashevo/js-dpp/compare/v0.12.1...v0.13.0) (2020-06-08)


### Bug Fixes

* document validation after validation the contract with the same id ([#166](https://github.com/dashevo/js-dpp/pull/166))


### Features

* support documents from multiple contracts in Documents Batch Transition ([#159](https://github.com/dashevo/js-dpp/pull/159))
* add `hash` method to `IdentityPublicKey` ([#170](https://github.com/dashevo/js-dpp/pull/170), [#173](https://github.com/dashevo/js-dpp/pull/173))
* `StateRepository#fetchTransaction` responses with verbose data ([#169](https://github.com/dashevo/js-dpp/pull/169))
* check asset lock transaction is confirmed ([#168](https://github.com/dashevo/js-dpp/pull/168), [#184](https://github.com/dashevo/js-dpp/pull/184))
* introduce Identity Topup Transition ([#167](https://github.com/dashevo/js-dpp/pull/167), [#178](https://github.com/dashevo/js-dpp/pull/178), [#180](https://github.com/dashevo/js-dpp/pull/180))
* validate first identity public key uniqueness ([#175](https://github.com/dashevo/js-dpp/pull/175))


### Code Refactoring

* rename `LockTransaction` to `AssetLockTransaction` ([#177](https://github.com/dashevo/js-dpp/pull/177))


### BREAKING CHANGES

* the first public key in Identity should be unique
* expect `StateRepository#fetchTransaction` to respond with verbose transaction


## [0.12.1](https://github.com/dashevo/js-dpp/compare/v0.12.0...v0.12.1) (2020-04-22)


### Bug Fixes

* data trigger should accept document transition ([#164](https://github.com/dashevo/js-dpp/issues/164))


# [0.12.0](https://github.com/dashevo/js-dpp/compare/v0.11.0...v0.12.0) (2020-04-17)


### Bug Fixes

* do not allow to change `ownerId` and `entropy` ([bff5807](https://github.com/dashevo/js-dpp/commit/bff580701322e2100e484989c476d583d26af38a))
* json schema for `signaturePublicKeyId` ([#161](https://github.com/dashevo/js-dpp/issues/161))
* wrong entropy size ([#157](https://github.com/dashevo/js-dpp/issues/157))
* data contract definitions might be `null` or `undefined` ([#153](https://github.com/dashevo/js-dpp/issues/153))
* identity existence validation in data contract structure validation ([#149](https://github.com/dashevo/js-dpp/issues/149))
* state transition signature validation in data contract structure validation ([#150](https://github.com/dashevo/js-dpp/pull/150))


### Code Refactoring

* rename `$rev` to `$revision` ([#140](https://github.com/dashevo/js-dpp/issues/140))
* rename `userId` to `ownerId` ([b9a5e83](https://github.com/dashevo/js-dpp/commit/b9a5e839608f94c964ff791bcbae4cb03a46028d))
* remove `type` from Identity ([227dc4d](https://github.com/dashevo/js-dpp/commit/227dc4d96e72172fd17cc44b46dd3ca0ef3da301))
* remove `version` from Data Contract ([f856ecc](https://github.com/dashevo/js-dpp/commit/f856ecc1b00e8f0962f96a9f84d84bd2322ad374))
* rename `$ownerId` to `ownerId` in Data Contract ([#160](https://github.com/dashevo/js-dpp/pull/160))
* rename `$contractId` to `$dataContractId` in Document ([158](https://github.com/dashevo/js-dpp/pull/158))
* split document model and it's state transitions ([#126](https://github.com/dashevo/js-dpp/issues/126), [#156](https://github.com/dashevo/js-dpp/pull/156))
* store document ID as a part of the document ([3d10a01](https://github.com/dashevo/js-dpp/commit/3d10a01577ca871cbf3fb1c4ea5f39904a27ca33))
* Data Contract Create Transition now accepts raw data ([#136](https://github.com/dashevo/js-dpp/issues/136))
* start types and indices from `0` instead of `1` ([#155](https://github.com/dashevo/js-dpp/pull/155))
* put JSON Schemas into order ([#135](https://github.com/dashevo/js-dpp/pull/135))


### Features

* implement apply state transition function ([#138](https://github.com/dashevo/js-dpp/issues/138), [#139](https://github.com/dashevo/js-dpp/issues/139), [#142](https://github.com/dashevo/js-dpp/issues/142), [#141](https://github.com/dashevo/js-dpp/issues/141), [#147](https://github.com/dashevo/js-dpp/issues/147), [#143](https://github.com/dashevo/js-dpp/issues/143))
* generate Data Contract ID from `ownerId` and entropy ([4c0dae1](https://github.com/dashevo/js-dpp/commit/4c0dae1a248d5a8af92f1023cdeed58377e51aae))
* introduce balance to Identities ([#137](https://github.com/dashevo/js-dpp/issues/137), [b13a9bb](https://github.com/dashevo/js-dpp/commit/b13a9bb2dfb22ea355620e064675a600a3908018), [#146](https://github.com/dashevo/js-dpp/issues/146))
* validate ST size is less than 16 Kb ([ff7aa51](https://github.com/dashevo/js-dpp/commit/ff7aa51dd88d4047637fb69e048a896dd92f3fd0), [70c3c54](https://github.com/dashevo/js-dpp/commit/70c3c541920a5bdc73845ac1ef835d7b21dfa92b))
* validate state transition fee ([48c9fda](https://github.com/dashevo/js-dpp/commit/48c9fda5cf958eb2046c8a5a98e09e78c1e8085f), [#145](https://github.com/dashevo/js-dpp/issues/145), [0cb1d6f](https://github.com/dashevo/js-dpp/commit/0cb1d6f69650e91ed944a11f77aeb6541e5755f4))
* create Identity factory now accepts locked out point and public keys ([#151](https://github.com/dashevo/js-dpp/issues/151))
* `getDataContractFixture` accepts `ownerId` ([#148](https://github.com/dashevo/js-dpp/issues/148))
* implement create identity create transition factory ([#152](https://github.com/dashevo/js-dpp/issues/152))
* introduce `signByPrivateKey` and `verifySignatureByPublicKey` methods to ST ([4eb5cdc](https://github.com/dashevo/js-dpp/commit/4eb5cdc408df8fe95294f668743c75da17ac0083))
* verbose invalid data errors ([#134](https://github.com/dashevo/js-dpp/pull/134))


### BREAKING CHANGES

* Data Contract ID is ownerId + entropy. You don't need to create an additional identity anymore.
* Data Contract Create Transition now accepts raw data
* size of serialized state transition must be less than 16 Kb
* `type` removed from Identity due to Data Contract doesn't require it anymore
* `version` removed from Data Contract
* `userId` renamed to `ownerId`
* Documents State Transition renamed to and it's structure is changed
* `applyStateTransition` methods no longer a part of identity, data contract and document facades
* renamed `$rev` field to `$revision` in raw document model
* applyIdentityStateTransition is now asynchronous
* Documents State Transition is now happening through separate state transition classes and renamed to Documents Batch Transition. Hence document class no longer have `$action` field. `$action` is now starting from 0. `$entropy` field is now a part of document create state transition. `createStateTransition` method of a document factory now accepts a map with actions as keys (`create`, `replace`, `delete`) and document arrays as values respectively.
* create Identity factory accepts locked out point and public keys instead of ID and `IndentityPublicKey`
* types and indices now starts from `0` instead of `1`
* Data Provider renamed to State Repository and store/remove functions introduced


## [0.11.1](https://github.com/dashevo/js-dpp/compare/v0.11.0...v0.11.1) (2020-03-17)


### Bug Fixes

* documents validate against wrong Data Contract ([0db6e44](https://github.com/dashevo/js-dpp/commit/0db6e44cfa8309d46bb42b5a0174574604861b2b))


# [0.11.0](https://github.com/dashevo/js-dpp/compare/v0.10.0...v0.11.0) (2020-03-09)


### Bug Fixes

* missing public key during ST signature validation ([667402d](https://github.com/dashevo/js-dpp/commit/667402dd659d50d7c2d9da5c61c32f2964a4c8b8))
* add npmignore ([c2e5f5d](https://github.com/dashevo/js-dpp/commit/c2e5f5d5b6c891b3280d02da659fb8eda613a43c))
* prevent to update dependencies with major version `0` to minor versions ([ea7de93](https://github.com/dashevo/js-dpp/commit/ea7de9379a38b856f4a7b779786986afacd75b0d))


### Features

* catch `decode` errors and rethrow consensus error ([892be82](https://github.com/dashevo/js-dpp/commit/892be823d44ff6edab82d89fa8e54b88f6b63534))
* limit data contract schema max depth ([f78df33](https://github.com/dashevo/js-dpp/commit/f78df334cf2f3e54744bcafdbbadeae54a5c980b))
* limit serialized Data Contract size to 15Kb ([7c95197](https://github.com/dashevo/js-dpp/commit/7c9519733cd05ef2c0b8d388a5135f54371f1054))
* remove Data Contract restriction option ([0edd6ff](https://github.com/dashevo/js-dpp/commit/0edd6ff85e2fe077f3c1c05c5fb8299417e1123e))
* validate documents JSON Schemas during data contract validation ([d88817d](https://github.com/dashevo/js-dpp/commit/d88817d5b7438168d225b6cec36377dac3e30284))
* ensure `maxItems` with `uniqueItems` for large non-scalar arrays ([3364325](https://github.com/dashevo/js-dpp/commit/3364325d23aaf72f37f2fdc663b29e8332d98f0e))
* ensure `maxLength` in case of `pattern` or `format` ([297c754](https://github.com/dashevo/js-dpp/commit/297c7543bfbe6723f92d83c50facb75ac4bfa00c))
* ensure all arrays items are defined ([43d7b8f](https://github.com/dashevo/js-dpp/commit/43d7b8f20886ec2c9f1bd6d16d6760d84a18c7c9))
* ensure all object properties are defined ([d9f71df](https://github.com/dashevo/js-dpp/commit/d9f71df99618719201ebfb0a3267bda1ed5b77c4))
* limit number of allowed indices ([5adff5d](https://github.com/dashevo/js-dpp/commit/5adff5d917c6e5bc11ee337ddb9f1775e8afc7d9))
* `validateData` method accept raw data too ([e72a627](https://github.com/dashevo/js-dpp/commit/e72a6274a26002ddd88c08c15dc89b8c8f94564d))
* prevent of defining `propertyNames` ([c40663f](https://github.com/dashevo/js-dpp/commit/c40663fc9c5db35a00c33ff43b24e2719ee84ee9))
* prevent of defining remote `$ref` ([34bdb3f](https://github.com/dashevo/js-dpp/commit/34bdb3f9c78cd1f2d01264752a9fb712ca313de8))
* prevent of using `default` keyword in Data Contract ([7629878](https://github.com/dashevo/js-dpp/commit/762987887112a89d4a153167e89a7ec97429994f))
* throw error if 16Kb reached for payload in `encode` function ([c6aba8b](https://github.com/dashevo/js-dpp/commit/c6aba8bf38c4a0f8c6dd955624eab6bf07a20a9c))
* accept `JsonSchemaValidator` as an option ([ee1bb0f](https://github.com/dashevo/js-dpp/commit/ee1bb0f180c8a3550da1f63c7a0200dac19f3966))


### BREAKING CHANGES

* Data Contract schema max depth is now limited by 500
* Serialized Data Contract size is now limited to 15Kb
* `validate`, `createFromSerialized`, `createFromObject` methods of Data Contract Factory are now async
* `items` and `additionalItems` are required for arrays
* `properties` and `additionalProperties` are required for objects
* number of indices limited to 10
* number of unique indices limited to 3
* number of properties in an index limited to 10
* required `maxItems` with `uniqueItems` for large non-scalar arrays
* required `maxLength` in case of `pattern` or `format`
* `propertyNames` keyword is restricted in document schema
* `default` keyword is restricted in Data Contract
* `encode` function throws error if payload is bigger than 16Kb
