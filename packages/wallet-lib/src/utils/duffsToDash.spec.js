const {expect} = require('chai');
const dashToDuffs = require('./dashToDuffs');
const {duffsToDash} = require("./index");

describe('Utils - duffsToDash', function suite() {
  it('should correctly convert duffs to dash', () => {
    it('should handle duff2Dash', () => {
      expect(duffsToDash(200000000000)).to.equal(2000);
      expect(duffsToDash(-200000000000)).to.equal(-2000);
      expect(() => duffsToDash('deuxmille')).to.throw('Can only convert a number');
    });
  });
});
