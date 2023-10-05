const convertToBase58chars = require('../../../lib/util/convertToBase58chars');
describe('convertToBase58chars', () => {
  it('should replace 0 and l to o and 1', () => {
    const result = convertToBase58chars('a0b0c0dlelfl');

    expect(result).to.equals('aobocod1e1f1');
  });
});
