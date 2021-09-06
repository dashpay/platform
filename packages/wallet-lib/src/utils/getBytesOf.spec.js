const { expect } = require('chai');
const { getBytesOf } = require("./index");

describe('Utils - getBytesOf', function suite() {
  it('should have getBytesOf return false on unknown type', () => {
    expect(getBytesOf(null, 'foo')).to.be.equal(false);
  });
});
