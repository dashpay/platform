const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getStatusFactory = require('../../../lib/rpcServer/commands/getStatus');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

const validQueries = [
  'getInfo',
  'getDifficulty',
  'getBestBlockHash',
  'getLastBlockHash',
];

describe('getStatus', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getStatus = getStatusFactory(coreAPIFixture);
      expect(getStatus).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getStatus');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(async () => {
    spy.restore();
  });

  it('Should return result if valid query passed', async () => {
    const getStatus = getStatusFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const results = await Promise.all(validQueries.map(query => getStatus({ query })));
    results.forEach((result) => {
      expect(result).to.be.an('object');
    });
    expect(spy.callCount).to.be.equal(4);
  });

  it('Should throw if arguments are not valid', async () => {
    const getStatus = getStatusFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getStatus([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getStatus(['invalidQuery'])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getStatus({ query: 'invalidQuery' })).to.be.rejectedWith('query should be equal to one of the allowed values');
    expect(spy.callCount).to.be.equal(0);
    await expect(getStatus({})).to.be.rejectedWith('should have required property \'query\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(getStatus({ query: 1 })).to.be.rejectedWith('query should be string');
    expect(spy.callCount).to.be.equal(0);
  });
});
