const calculateItemsHash = require('../../../lib/stPacket/calculateItemsHash');

describe('calculateItemsHash', () => {
  it('should returns null if contracts and objects are empty', () => {
    const result = calculateItemsHash({
      contracts: [],
      objects: [],
    });

    expect(result).to.be.null();
  });
});
