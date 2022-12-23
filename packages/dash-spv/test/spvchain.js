const { expect } = require('chai');
const { SpvChain } = require('../index');

const { testnet } = require('./data/rawHeaders');

describe('SPVChain', () => {
  let spvChain;

  beforeEach(() => {
    spvChain = new SpvChain('testnet', 100);
  });

  describe('#addHeaders', () => {
    it('should assemble headers chain if headers arriving out of order', () => {
      expect(true).to.equal(true);
      spvChain.reset(10000);

      spvChain.addHeaders(testnet.slice(400), 10400);
      expect(spvChain.getOrphanChunks()).to.have.length(1);
      spvChain.addHeaders(testnet.slice(200, 300), 10200);
      expect(spvChain.getOrphanChunks()).to.have.length(2);
      spvChain.addHeaders(testnet.slice(300, 400), 10300);
      expect(spvChain.getOrphanChunks()).to.have.length(3);
      spvChain.addHeaders(testnet.slice(100, 200), 10100);
      expect(spvChain.getOrphanChunks()).to.have.length(4);
      spvChain.addHeaders(testnet.slice(0, 100), 10000);

      const longestChain = spvChain.getLongestChain({ withPruned: true });
      expect(longestChain).to.have.length(testnet.length);
      expect(spvChain.getOrphanChunks()).to.have.length(0);
    });
  });
});
