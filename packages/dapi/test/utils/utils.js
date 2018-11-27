const chai = require('chai');

const { expect } = chai;
const chaiAsPromised = require('chai-as-promised');

chai.use(chaiAsPromised);
const { isPortTaken } = require('../../lib/utils/utils');

describe('utils/utils', () => {
  describe('#isPortTaken', () => {
    it('should isPortTaken return promise', () => {
      const res = isPortTaken();
      expect(res).to.be.a('promise');
    });
    it('should isPortTaken reject 22 port', async () => {
      await expect(isPortTaken(22)).to.be.rejectedWith('listen EACCES 127.0.0.1:22');
    });
    it('should isPortTaken reject invalid port', async () => {
      await expect(isPortTaken(22222222)).to.be.rejectedWith('"port" argument must be >= 0 and < 65536');
    });
  });
});
