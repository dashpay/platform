const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const BloomFilter = require('bloom-filter');
const loadBloomFilterFactory = require('../../../lib/rpcServer/commands/loadBloomFilter');
const spvServiceFixture = require('../../fixtures/spvServiceFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('loadBloomFilter', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const loadBloomfilter = loadBloomFilterFactory(spvServiceFixture);
      expect(loadBloomfilter).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(spvServiceFixture, 'loadBloomFilter');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  describe('loadBloomFilter', () => {
    it('should return a promise', async () => {
      const loadBloomFilter = loadBloomFilterFactory(spvServiceFixture);
      expect(spy.callCount).to.be.equal(0);
      const res = await loadBloomFilter({ filter: BloomFilter.create(10, 0.01) });
      expect(res).to.be.equal(true);
      expect(spy.callCount).to.be.equal(1);
    });

    it('Should throw if arguments are not valid', async () => {
      const loadBloomFilter = loadBloomFilterFactory(spvServiceFixture);
      expect(spy.callCount).to.be.equal(0);
      await expect(loadBloomFilter([])).to.be.rejected;
      expect(spy.callCount).to.be.equal(0);
      await expect(loadBloomFilter({})).to.be.rejectedWith('should have required property \'filter\'');
      expect(spy.callCount).to.be.equal(0);
    });
  });
});
