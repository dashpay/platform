const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getAddressUnconfirmedBalanceFactory = require('../../../lib/rpcServer/commands/getAddressUnconfirmedBalance.js');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getAddressUnconfirmedBalance', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getAddressUnconfirmedBalance = getAddressUnconfirmedBalanceFactory(coreAPIFixture);
      expect(getAddressUnconfirmedBalance).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getAddressUnconfirmedBalance');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return a number', async () => {
    const getAddressUnconfirmedBalance = getAddressUnconfirmedBalanceFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const unconfirmedBalance = await getAddressUnconfirmedBalance({ address: 'XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w' });
    expect(unconfirmedBalance).to.be.an('number');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw if arguments are not valid', async () => {
    const getAddressUnconfirmedBalance = getAddressUnconfirmedBalanceFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getAddressUnconfirmedBalance([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getAddressUnconfirmedBalance({})).to.be.rejectedWith('should have required property \'address\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(getAddressUnconfirmedBalance({ address: 1 })).to.be.rejectedWith('address should be string');
    expect(spy.callCount).to.be.equal(0);
  });
});
