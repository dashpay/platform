const entropy = require('../../../lib/util/entropy');

describe('entropy', () => {
  describe('generate', () => {
    it('should generate a string', () => {
      const randomBuffer = entropy.generate();

      expect(Buffer.isBuffer(randomBuffer)).to.be.true();
      expect(randomBuffer.length).to.be.above(1);
    });

    it('should generate random string', () => {
      const randomBuffer = entropy.generate();
      const secondRandomBuffer = entropy.generate();

      expect(randomBuffer).to.not.deep.equal(secondRandomBuffer);
    });
  });

  describe('validate', () => {
    it('should return true if entropy is valid', () => {
      const randomBuffer = entropy.generate();

      const result = entropy.validate(randomBuffer);

      expect(result).to.be.true();
    });

    it('should return false if entropy is invalid', () => {
      const result = entropy.validate(Buffer.from('wrong'));

      expect(result).to.be.false();
    });
  });
});
