const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getBestBlockHashFactory = require('../../../lib/rpcServer/commands/getBestBlockHash.js');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getBestBlockHash', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getBestBlockHash = getBestBlockHashFactory(coreAPIFixture);
      expect(getBestBlockHash).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getBestBlockHash');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return a number', async () => {
    const getBestBlockHash = getBestBlockHashFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const bestBlockHash = await getBestBlockHash();
    expect(bestBlockHash).to.be.an('string');
    expect(spy.callCount).to.be.equal(1);
  });
});
