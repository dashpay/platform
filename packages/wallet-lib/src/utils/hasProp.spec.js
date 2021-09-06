const { expect } = require('chai');
const { hasProp } = require("./index");

describe('Utils - hasProp', function suite() {
  it('should correctly handle property detection', function () {
      expect(hasProp({ key1: true }, 'key1')).to.equal(true);
      expect(hasProp({ key1: true }, 'key2')).to.equal(false);
      expect(hasProp(['key1'], 'key1')).to.equal(true);
      expect(hasProp(['key1'], 'key2')).to.equal(false);
      expect(hasProp(null, 'key2')).to.equal(false);
  });
});
