const { PrivateKey } = require('@dashevo/dashcore-lib');
const { default: loadWasmDpp } = require('../../..');

let { generateTemporaryEcdsaPrivateKey } = require('../../..');

describe('generateTemporaryEcdsaPrivateKey', () => {
  beforeEach(async () => {
    ({ generateTemporaryEcdsaPrivateKey } = await loadWasmDpp());
  });

  it('should generate a valid private key', () => {
    const keyBuffer = generateTemporaryEcdsaPrivateKey();

    expect(keyBuffer.length).to.be.equal(32);

    const privateKey = new PrivateKey(keyBuffer);

    expect(privateKey).to.be.instanceOf(PrivateKey);
  });
});
