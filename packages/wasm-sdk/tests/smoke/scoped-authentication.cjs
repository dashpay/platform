/* Run against a real wasm-bindgen --target nodejs build:
 * node tests/smoke/scoped-authentication.cjs /absolute/path/to/wasm_sdk.js
 */
const assert = require('node:assert/strict');
const path = require('node:path');

if (!process.argv[2]) throw new Error('Pass the generated wasm_sdk.js module path');
const wasm = require(path.resolve(process.argv[2]));
const id = '11111111111111111111111111111111';
const P = wasm.AuthenticationPermission;
const bounds = wasm.ContractBounds.Scoped(
  [{ id, documentTypes: ['post', 'like'] }],
  P.DocumentCreate | P.DocumentTokenPayment,
  100n,
);
assert.deepEqual(bounds.scope.contracts[0].documentTypes, ['like', 'post']);
assert.deepEqual(wasm.ContractBounds.fromJSON(bounds.toJSON()).toJSON(), bounds.toJSON());
assert.deepEqual(wasm.ContractBounds.fromObject(bounds.toObject()).toJSON(), bounds.toJSON());
assert.equal(bounds.identifier, undefined);
assert.throws(() => { bounds.documentTypeName = 'profile'; });

for (const [contracts, mask] of [
  [[{ id, documentTypes: [] }], 65],
  [[{ id }, { id }], 65],
  [[{ id }], 0],
  [[{ id }], 1 << 30],
  [[{ id, documentTypes: ['a'.repeat(2048)] }], 65],
]) {
  assert.throws(() => wasm.ContractBounds.Scoped(contracts, mask));
}

const unrestricted = wasm.ContractBounds.Scoped([{ id }], P.DocumentCreate);
const unrestrictedObject = unrestricted.toObject();
assert.equal(unrestrictedObject.contracts[0].documentTypes, undefined);
assert.equal(unrestrictedObject.expiresAt, undefined);
assert.deepEqual(
  wasm.ContractBounds.fromObject(unrestrictedObject).toJSON(),
  unrestricted.toJSON(),
);

const key = new wasm.IdentityPublicKeyInCreation({
  keyId: 2,
  purpose: 'authentication',
  securityLevel: 'high',
  keyType: 'ecdsa_hash160',
  isReadOnly: false,
  data: new Uint8Array(20).fill(1),
  signature: new Uint8Array(),
  contractBounds: bounds,
});
assert.deepEqual(key.contractBounds.toJSON(), bounds.toJSON());
assert.deepEqual(
  wasm.IdentityPublicKeyInCreation.fromJSON(key.toJSON()).contractBounds.toJSON(),
  bounds.toJSON(),
);
console.log('Scoped authentication WASM smoke checks passed');
