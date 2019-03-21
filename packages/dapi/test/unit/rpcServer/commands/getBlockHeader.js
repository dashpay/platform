const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getBlockHeaderFactory = require('../../../../lib/rpcServer/commands/getBlockHeader');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getBlockHeader', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getBlockHeaders = getBlockHeaderFactory(coreAPIFixture);
      expect(getBlockHeaders).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getBlockHeader');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return a header object', async () => {
    const getBlockHeader = getBlockHeaderFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const blockHeader = await getBlockHeader({ blockHash: '000000000000003e1918a7f080c6d795f9d0486c715095cb133d6171f394f8e9' });
    expect(blockHeader).to.be.an('object');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw an error if arguments are not valid', async () => {
    const getBlockHeader = getBlockHeaderFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlockHeader({ blockHash: '000000000000003e1918a7f080c6d795f9d0486c715095cb133d6171f394f8e90' }))
      .to.be.rejectedWith('params.blockHash should NOT be longer than 64 characters');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBlockHeader({ blockHash: '000123' }))
      .to.be.rejectedWith('params.blockHash should NOT be shorter than 64 characters');
    expect(spy.callCount).to.be.equal(0);
  });
});
