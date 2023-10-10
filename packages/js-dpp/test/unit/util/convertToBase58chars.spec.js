const convertToHomographSafeChars = require('../../../lib/util/convertToHomographSafeChars');

describe('convertToHomographSafeChars', () => {
  it('should replace o, l, i to 0 and 1', () => {
    const result = convertToHomographSafeChars('A0boic0Dlelfl');

    expect(result).to.equals('a0b01c0d1e1f1');
  });
});
