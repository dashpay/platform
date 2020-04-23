const { expect } = require('chai');
const varInt = require('./varInt');

describe('Utils - varInt', () => {
  it('should get varint of size from length', () => {
    expect(varInt.varIntSizeBytesFromLength()).to.equal(1);
    expect(varInt.varIntSizeBytesFromLength(1)).to.equal(1);
    expect(varInt.varIntSizeBytesFromLength(42)).to.equal(1);
    expect(varInt.varIntSizeBytesFromLength(42000)).to.equal(3);
    expect(varInt.varIntSizeBytesFromLength(4200000000)).to.equal(5);
  });
});
