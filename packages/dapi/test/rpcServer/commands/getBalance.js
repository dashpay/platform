const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getBalanceFactory = require('../../../lib/rpcServer/commands/getBalance.js');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');

const { expect } = chai;
chai.use(chaiAsPromised);
let spy;

describe('getBalance', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getBalance = getBalanceFactory(coreAPIFixture);
      expect(getBalance).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getBalance');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return a number', async () => {
    const getBalance = getBalanceFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const balance = await getBalance({ address: 'XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w' });
    expect(balance).to.be.an('number');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw if arguments are not valid', async () => {
    const getBalance = getBalanceFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getBalance([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getBalance({})).to.be.rejectedWith('should have required property \'address\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(getBalance({ address: 1 })).to.be.rejectedWith('address should be string');
    expect(spy.callCount).to.be.equal(0);
  });
});
