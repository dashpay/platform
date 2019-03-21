const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const BloomFilter = require('bloom-filter');
const getSpvDataFactory = require('../../../../lib/rpcServer/commands/getSpvData');
const spvServiceFixture = require('../../../mocks/spvServiceFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getSpvData', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getSpvData = getSpvDataFactory(spvServiceFixture);
      expect(getSpvData).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(spvServiceFixture, 'getSpvData');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  describe('getSpvData', () => {
    it('should return a promise', async () => {
      const getSpvData = getSpvDataFactory(spvServiceFixture);
      expect(spy.callCount).to.be.equal(0);
      const res = await getSpvData({ filter: BloomFilter.create(10, 0.01) });
      expect(res).to.be.an('object');
      expect(spy.callCount).to.be.equal(1);
    });

    it('Should throw if arguments are not valid', async () => {
      const getSpvData = getSpvDataFactory(spvServiceFixture);
      expect(spy.callCount).to.be.equal(0);
      await expect(getSpvData([])).to.be.rejected;
      expect(spy.callCount).to.be.equal(0);
      await expect(getSpvData({})).to.be.rejectedWith('should have required property \'filter\'');
      expect(spy.callCount).to.be.equal(0);
    });
  });
});
