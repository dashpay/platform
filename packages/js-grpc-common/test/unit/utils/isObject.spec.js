const isObject = require('../../../lib/utils/isObject');

describe('isObject', () => {
  it('should return true if argument is Object', () => {
    const result = isObject({ some: 42 });
    expect(result).to.be.true();
  });

  it('should return false if argument is not Object', () => {
    const result = isObject('asdasd');
    expect(result).to.be.false();
  });
});
