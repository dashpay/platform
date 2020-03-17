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
