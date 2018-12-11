const entropy = require('../../../lib/util/entropy');

describe('entropy', () => {
  describe('generate', () => {
    it('should generate a string', () => {
      const randomString = entropy.generate();

      expect(randomString).to.be.a('string');
      expect(randomString.length).to.be.above(1);
    });

    it('should generate random string', () => {
      const randomString = entropy.generate();
      const secondRandomString = entropy.generate();

      expect(secondRandomString).to.be.not.equal(randomString);
    });
  });

  describe('validate', () => {
    it('should return true if entropy is valid', () => {
      const randomString = entropy.generate();

      const result = entropy.validate(randomString);

      expect(result).to.be.true();
    });

    it('should return false if entropy is invalid', () => {
      const result = entropy.validate('wrong');

      expect(result).to.be.false();
    });
  });
});
