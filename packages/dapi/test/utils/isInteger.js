const chai = require('chai');

const { expect } = chai;
const isInteger = require('../../lib/utils/isInteger');

describe('utils/isInteger', () => {
  describe('#isPortTaken', () => {
    it('should isInteger return function', () => {
      const res = isInteger;
      expect(res).to.be.a('function');
    });
    it('should isInteger handle positive integer', async () => {
      const res = isInteger(22);
      expect(res).to.be.equal(true);
    });
    it('should isInteger handle negative integer', async () => {
      const res = isInteger(-22);
      expect(res).to.be.equal(true);
    });
    it('should isInteger handle positive integer as decimal', async () => {
      const res = isInteger(2.0);
      expect(res).to.be.equal(true);
    });
    it('should isInteger handle floats', async () => {
      const res = isInteger(2.01);
      expect(res).to.be.equal(false);
    });
    it('should isInteger handle boolean', async () => {
      const res = isInteger(true);
      expect(res).to.be.equal(false);
    });
  });
});
