const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getBlockHashFactory = require('../../../../lib/rpcServer/commands/getBlockHash');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getBlockHash', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getBlockHash = getBlockHashFactory(coreAPIFixture);
      expect(getBlockHash).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getBlockHash');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return block hash', async () => {
    const getBlockHash = getBlockHashFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const blockHash = await getBlockHash({ height: 100 });
    expect(blockHash).to.be.a('string');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw an error if arguments are not valid', async () => {
    const getBlockHash = getBlockHashFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlockHash({ height: -1 })).to.be.rejectedWith('params/height must be >= 0');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlockHash({ height: 0.5 })).to.be.rejectedWith('params/height must be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlockHash({})).to.be.rejectedWith('must have required property');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlockHash()).to.be.rejectedWith('params must be object');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlockHash({ height: 'string' })).to.be.rejectedWith('params/height must be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlockHash([-1])).to.be.rejectedWith('params must be object');
    expect(spy.callCount).to.be.equal(0);
  });
});
