const { PrivateKey } = require('@dashevo/dashcore-lib');
const { default: loadWasmDpp } = require('../../..');

let { generateTemporaryEcdsaPrivateKey } = require('../../..');

describe.skip('generateTemporaryEcdsaPrivateKey', () => {
  beforeEach(async () => {
    ({ generateTemporaryEcdsaPrivateKey } = await loadWasmDpp());
  });

  it('should generate a valid private key', () => {
    const keyBase64 = generateTemporaryEcdsaPrivateKey();

    // eslint-disable-next-line
    const _key = new PrivateKey(keyBase64);
  });
});
