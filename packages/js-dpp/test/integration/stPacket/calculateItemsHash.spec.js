const calculateItemsHash = require('../../../lib/stPacket/calculateItemsHash');

describe.skip('calculateItemsHash', () => {
  it('should return null if contracts and documents are empty', () => {
    const result = calculateItemsHash({
      contracts: [],
      documents: [],
    });

    expect(result).to.be.null();
  });
});
