# [0.13.0](https://github.com/dashevo/js-dpp/compare/v0.12.1...v0.13.0) (2020-06-08)


### Bug Fixes

* document validation after validation the contract with the same id ([#166](https://github.com/dashevo/js-dpp/pull/166))


### Features

* support documents from multiple contracts in Documents Batch Transition ([#159](https://github.com/dashevo/js-dpp/pull/159))
* add `hash` method to `IdentityPublicKey` ([#170](https://github.com/dashevo/js-dpp/pull/170), [#173](https://github.com/dashevo/js-dpp/pull/173))
* `StateRepository#fetchTransaction` responses with verbose data ([#169](https://github.com/dashevo/js-dpp/pull/169))
* check asset lock transaction is confirmed ([#168](https://github.com/dashevo/js-dpp/pull/168), [#184](https://github.com/dashevo/js-dpp/pull/184))
* identity topup state transition ([#167](https://github.com/dashevo/js-dpp/pull/167), [#178](https://github.com/dashevo/js-dpp/pull/178), [#180](https://github.com/dashevo/js-dpp/pull/180))
* validate first identity public key uniqueness ([#175](https://github.com/dashevo/js-dpp/pull/175))


### Code Refactoring

* rename `LockTransaction` to `AssetLockTransaction` ([#177](https://github.com/dashevo/js-dpp/pull/177))



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
