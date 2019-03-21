const chai = require('chai');
const SimpleCache = require('../../../lib/services/caching/spvSimpleCache');

const { expect } = chai;


let cache;
const randomHash = 'b1044fab96f048d44e55de54506f0285a1faad3e12826f51aeee6e9f9db71234';
describe('Caching', () => {
  describe('getCorrectedHash', () => {
    before(() => {
      cache = new SimpleCache();
    });
    it('add a cached value', () => {
      cache.set(randomHash, { merkleBlock: 'someMerkleBlock' });

      expect(cache.get(randomHash).merkleblocks.length).to.be.equal(1);
      expect(cache.getAllFilterHashes().length).to.be.equal(1);
      expect(cache.getAllFilterHashes()[0]).to.be.equal(randomHash);
    });

    it('clear cached values', () => {
      cache.clearInactiveClients([]);
      expect(cache.get(randomHash)).to.be.equal(undefined);
      expect(cache.getAllFilterHashes().length).to.be.equal(0);
    });
  });
});
