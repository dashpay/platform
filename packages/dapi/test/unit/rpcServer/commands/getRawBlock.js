const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getRawBlockFactory = require('../../../../lib/rpcServer/commands/getRawBlock');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getRawBlock', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getRawBlock = getRawBlockFactory(coreAPIFixture);
      expect(getRawBlock).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getRawBlock');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(async () => {
    spy.restore();
  });

  it('Should return an object', async () => {
    const getRawBlock = getRawBlockFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const rawBlock = await getRawBlock({ blockHash: 'XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w' });
    expect(rawBlock).to.be.an('object');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw if arguments are not valid', async () => {
    const getRawBlock = getRawBlockFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getRawBlock([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getRawBlock({})).to.be.rejectedWith('should have required property \'blockHash\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(getRawBlock({ blockHash: 1 })).to.be.rejectedWith('blockHash should be string');
    expect(spy.callCount).to.be.equal(0);
  });
});
