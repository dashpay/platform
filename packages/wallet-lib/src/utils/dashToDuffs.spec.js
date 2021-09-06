const {expect} = require('chai');
const dashToDuffs = require('./dashToDuffs');
const {duffsToDash} = require("./index");

describe('Utils - dashToDuffs', function suite() {
  it('should correctly convert dash to duffs', () => {
    const results = [
      dashToDuffs(1),
      dashToDuffs(-1),
      dashToDuffs(0.1),
      dashToDuffs(0.01),
      dashToDuffs(0.00000001),
      dashToDuffs(0.000000001),
      dashToDuffs(-0.000000001),
      dashToDuffs(-12345678.9876543210),
    ]
    const expectedResults = [
      100000000,
      -100000000,
      10000000,
      1000000,
      1,
      0,
      -0,
      -1234567898765432
    ]
    results.forEach((result, resultIndex) => {
      expect(results[resultIndex]).to.equal(expectedResults[resultIndex]);
    })
    expect(() => dashToDuffs('deuxmille')).to.throw('Can only convert a number');
  });
});
