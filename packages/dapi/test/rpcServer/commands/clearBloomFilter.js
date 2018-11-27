const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const BloomFilter = require('bloom-filter');
const clearBloomFilterFactory = require('../../../lib/rpcServer/commands/clearBloomFilter');
const spvServiceFixture = require('../../fixtures/spvServiceFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('clearBloomFilter', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const clearBloomFilter = clearBloomFilterFactory(spvServiceFixture);
      expect(clearBloomFilter).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(spvServiceFixture, 'clearBloomFilter');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  describe('clearBloomFilter', () => {
    it('should return a promise', async () => {
      const clearBloomFilter = clearBloomFilterFactory(spvServiceFixture);
      expect(spy.callCount).to.be.equal(0);
      const res = await clearBloomFilter({ filter: BloomFilter.create(10, 0.01) });
      expect(res).to.be.equal(true);
      expect(spy.callCount).to.be.equal(1);
    });

    it('Should throw if arguments are not valid', async () => {
      const clearBloomFilter = clearBloomFilterFactory(spvServiceFixture);
      expect(spy.callCount).to.be.equal(0);
      await expect(clearBloomFilter([])).to.be.rejected;
      expect(spy.callCount).to.be.equal(0);
      await expect(clearBloomFilter({})).to.be.rejectedWith('should have required property \'filter\'');
      expect(spy.callCount).to.be.equal(0);
    });
  });
});
