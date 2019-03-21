const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getBlocksFactory = require('../../../../lib/rpcServer/commands/getBlocks');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getBlocks', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getBlocks = getBlocksFactory(coreAPIFixture);
      expect(getBlocks).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getBlocks');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return array of blocks', async () => {
    const getBlocks = getBlocksFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const blocks = await getBlocks({ limit: 2, blockDate: '123' });
    expect(blocks).to.be.an('array');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw an error if arguments are not valid', async () => {
    const getBlocks = getBlocksFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlocks({ limit: -1, blockDate: '123' })).to.be.rejectedWith('should be >= 1');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlocks({ limit: 0, blockDate: '123' })).to.be.rejectedWith('should be >= 1');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlocks({ limit: 0.5, blockDate: '123' })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlocks({ limit: 1, blockDate: 23 })).to.be.rejectedWith('should be string');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlocks({})).to.be.rejectedWith('should have required property');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlocks()).to.be.rejectedWith('should be object');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlocks({ limit: 'string', blockDate: '123' })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlocks([-1])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlocks([1, 2])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
  });
});
