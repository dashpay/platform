const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getSpvDataFactory = require('../../../lib/rpcServer/commands/findDataForBlock');
const BloomFilter = require('bloom-filter');
const spvServiceFixture = require('../../fixtures/spvServiceFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('findDataForBlock', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const findDataForBlock = getSpvDataFactory(spvServiceFixture);
      expect(findDataForBlock).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(spvServiceFixture, 'findDataForBlock');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  describe('findDataForBlock', () => {
    it('should return a promise', async () => {
      const findDataForBlock = getSpvDataFactory(spvServiceFixture);
      expect(spy.callCount).to.be.equal(0);
      const res = await findDataForBlock({ filter: BloomFilter.create(1, 0.1, 1, 1), blockHash: '' });
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
