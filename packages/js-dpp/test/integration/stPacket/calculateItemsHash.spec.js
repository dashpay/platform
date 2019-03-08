const calculateItemsHash = require('../../../lib/stPacket/calculateItemsHash');

describe('calculateItemsHash', () => {
  it('should return null if contracts and objects are empty', () => {
    const result = calculateItemsHash({
      contracts: [],
      objects: [],
    });

    expect(result).to.be.null();
  });
});
