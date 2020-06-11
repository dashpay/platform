# [3.13.0](https://github.com/dashevo/DashJS/compare/v3.13.0...v3.0.2) (2020-05-11)

- **feat:**
  - feat: update wallet lib to 7.1.4 (#80)

# [3.0.2](https://github.com/dashevo/DashJS/compare/v3.0.2...v3.0.1) (2020-05-06)

- **fix**:
  - typescript support (#46)

# [3.0.1](https://github.com/dashevo/DashJS/compare/v3.0.1...v3.0.0) (2020-04-27)

- **fix**:
  - changed dpp.documents (undefined) to dpp.document (#48)

# [3.0.0](https://github.com/dashevo/DashJS/compare/v3.0.0...v2.0.0) (2020-04-24)

- **breaking:**
  - Identity registration will use HDKeys(0) instead 1 (https://github.com/dashevo/DashJS/pull/41/commits/4bbc54d265c679affbd043b03a88f8ed2f1d52fb)
  - contract.broadcast() now returns dataContract (https://github.com/dashevo/DashJS/pull/41/commits/6f7e9225f317525388fb7701619da74b5d76222b#diff-486b5234782255b516fe9c1868c7d3b0R19)
  - identities.broadcast() now return identity (https://github.com/dashevo/DashJS/pull/41/commits/4bbc54d265c679affbd043b03a88f8ed2f1d52fb#diff-27f47e1bd838b3993aed5eaa396a00e5R90)
  - document.broadcast() creation is now performed via passing documents to be created in an array of property create. `{create:[document]}` (https://github.com/dashevo/DashJS/commit/91127d774a339c4204891f5863c91a64d521ddb8#diff-0202b3d53936b94585a8c0cfa0481bccR10)

- **feat**:
  - added replacement of a document. (#41)
  - added deletion of a document (#41)

- **impr**: 
  - update to dpp 0.12 (#41)

- **fix**:
  - properly release (throw) catched error (#41)

- **Chore, Docs & Tests:**
  - bumped wallet-lib to 6.1 (#41)
 
# [2.0.0](https://github.com/dashevo/DashJS/compare/v2.0.0...v1.1.2) (2020-03-27)

- **breaking:**
  - renamed DashJS namespace to SDK namespace.
  - renamed SDK namespace to Client namespace (DashJS.SDK -> SDK.Client).
  - moved L1 primitive namespace from `SDK.*` to `SDK.Core.*`.
  - moved L2 primitive namespace from `SDK.*` to `SDK.Platform.*`.
  - exported file for web environment is now `Dash` instead of `DashJS`
- **feat**:
  - Sign and verify message (#24)
- **impr**: 
  - Typings documentation (#30)
  - Code cleanup (#31)
  - Export all Dashcore Primitives (under `SDK.Core.*`)
- **fix**:
  - fix(contracts): pass data contract without array (#32 in #31)
  - fix: remove .serialize() before broadcasting records (#e047d515a12d0d14ff69b4fe3ea5b8b10bd6f890)
  - Identity/register: updated getUTXOS usages on (#afda5bbafb940b2e15d5e773d0e8fc5fbc48ee13)
  - fix(StateTransitionBuilder): records type detection (#b49f74b4b8e03e9d1020dd789c62f4310a4fc1ad)
  - broadcasting of contracts (#f4b63e6be692841f1b138e5b058e531a0873f456, #4aa31fec0e5579d7ef8b9222576863a069b95fd3)
- **chore**: 
  - updated for new evonet (updated wallet-lib to 6.0.0)
  -  updated dapi-client to 0.11 (#fba4d55d3281bec5e65605787dd23a6ca3517476)
  - updated DPNS contractID on evonet (#d0cf11d30cf7c9aaef1ffa4a2b8a955fbf5b1184)
