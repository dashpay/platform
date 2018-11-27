const chai = require('chai');
const sinon = require('sinon');
const chaiAsPromised = require('chai-as-promised');
const getAddressTotalReceivedFactory = require('../../../lib/rpcServer/commands/getAddressTotalReceived');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getAddressTotalReceived', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getAddressTotalReceived = getAddressTotalReceivedFactory(coreAPIFixture);
      expect(getAddressTotalReceived).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getAddressTotalReceived');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return a number', async () => {
    const getAddressTotalReceived = getAddressTotalReceivedFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const totalReceived = await getAddressTotalReceived({ address: 'XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w' });
    expect(totalReceived).to.be.an('number');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw if arguments are not valid', async () => {
    const getAddressTotalReceived = getAddressTotalReceivedFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getAddressTotalReceived([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getAddressTotalReceived({})).to.be.rejectedWith('should have required property \'address\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(getAddressTotalReceived({ address: 1 })).to.be.rejectedWith('address should be string');
    expect(spy.callCount).to.be.equal(0);
  });
});
