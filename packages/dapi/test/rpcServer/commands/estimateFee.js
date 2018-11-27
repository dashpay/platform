const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sion = require('sinon');
const estimateFeeFactory = require('../../../lib/rpcServer/commands/estimateFee');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');

const { expect } = chai;
chai.use(chaiAsPromised);

let spy;

describe('estimateFee', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const estimateFee = estimateFeeFactory(coreAPIFixture);
      expect(estimateFee).to.be.a('function');
    });
  });

  before(() => {
    spy = sion.spy(coreAPIFixture, 'estimateFee');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return a number', async () => {
    const estimateFee = estimateFeeFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const fee = await estimateFee({ nbBlocks: 1 });
    expect(fee).to.be.a('number');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw an error if arguments are not valid', async () => {
    const estimateFee = estimateFeeFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(estimateFee({ nbBlocks: -1 })).to.be.rejectedWith('should be >= 0');
    expect(spy.callCount).to.be.equal(0);
    await expect(estimateFee({ nbBlocks: 0.5 })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(estimateFee({})).to.be.rejectedWith('should have required property');
    expect(spy.callCount).to.be.equal(0);
    await expect(estimateFee()).to.be.rejectedWith('should be object');
    expect(spy.callCount).to.be.equal(0);
    await expect(estimateFee({ nbBlocks: 'string' })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(estimateFee([-1])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
  });
});
