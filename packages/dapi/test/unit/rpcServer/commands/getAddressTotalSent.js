const chai = require('chai');
const sinon = require('sinon');
const chaiAsPromised = require('chai-as-promised');
const getAddressTotalSentFactory = require('../../../../lib/rpcServer/commands/getAddressTotalSent');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');


chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getAddressTotalSent', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getAddressTotalSent = getAddressTotalSentFactory(coreAPIFixture);
      expect(getAddressTotalSent).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getAddressTotalSent');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(async () => {
    spy.restore();
  });

  it('Should return a number', async () => {
    const getAddressTotalSent = getAddressTotalSentFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const totalSent = await getAddressTotalSent({ address: 'XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w' });
    expect(totalSent).to.be.an('number');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw if arguments are not valid', async () => {
    const getAddressTotalSent = getAddressTotalSentFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getAddressTotalSent([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getAddressTotalSent({})).to.be.rejectedWith('should have required property \'address\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(getAddressTotalSent({ address: 1 })).to.be.rejectedWith('address should be string');
    expect(spy.callCount).to.be.equal(0);
  });
});
