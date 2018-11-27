const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const BloomFilter = require('bloom-filter');
const addToBloomFilterFactory = require('../../../lib/rpcServer/commands/addToBloomFilter');
const spvServiceFixture = require('../../fixtures/spvServiceFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('addToBloomFilter', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const addToBloomFilter = addToBloomFilterFactory(spvServiceFixture);
      expect(addToBloomFilter).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(spvServiceFixture, 'addToBloomFilter');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  describe('addToBloomFilter', () => {
    it('promise should resolve to true', async () => {
      const addToBloomFilter = addToBloomFilterFactory(spvServiceFixture);
      expect(spy.callCount).to.be.equal(0);
      const res = await addToBloomFilter({
        originalFilter: BloomFilter.create(10, 0.01),
        element: {},
      });
      expect(res).to.be.equal(true);
      expect(spy.callCount).to.be.equal(1);
    });

    it('Should throw if arguments are not valid', async () => {
      const addToBloomFilter = addToBloomFilterFactory(spvServiceFixture);
      expect(spy.callCount).to.be.equal(0);
      await expect(addToBloomFilter([])).to.be.rejected;
      expect(spy.callCount).to.be.equal(0);
      await expect(addToBloomFilter({})).to.be.rejectedWith('should have required property \'originalFilter\'');
      expect(spy.callCount).to.be.equal(0);
      const expectedError = 'should have required property \'element\'';
      await expect(addToBloomFilter({ originalFilter: {} })).to.be.rejectedWith(expectedError);
      expect(spy.callCount).to.be.equal(0);
    });
  });
});
