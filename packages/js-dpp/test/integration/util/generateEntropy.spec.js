const generateEntropy = require('../../../lib/util/generateEntropy');

describe('generateEntropy', () => {
  it('should generate a byte array of length 32', () => {
    const entropy = generateEntropy();

    expect(Buffer.isBuffer(entropy)).to.be.true();
    expect(entropy.byteLength).to.be.equal(32);
  });

  it('should generate random byte array', () => {
    const randomBuffer = generateEntropy();
    const secondRandomBuffer = generateEntropy();

    expect(randomBuffer).to.not.deep.equal(secondRandomBuffer);
  });
});
