const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sion = require('sinon');
const getTransactionsByAddressFactory = require('../../../lib/rpcServer/commands/getTransactionsByAddress');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');

const { expect } = chai;
chai.use(chaiAsPromised);
let spy;

describe('getTransactionsByAddress', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getTransactionsByAddress = getTransactionsByAddressFactory(coreAPIFixture);
      expect(getTransactionsByAddress).to.be.a('function');
    });
  });

  before(() => {
    spy = sion.spy(coreAPIFixture, 'getTransactionsByAddress');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return an array', async () => {
    const getTransactionsByAddress = getTransactionsByAddressFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const transactions = await getTransactionsByAddress({ address: 'XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w' });
    expect(transactions).to.be.an('array');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should accept an array as input', async () => {
    const getTransactionsByAddress = getTransactionsByAddressFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const addressArray = ['XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w', 'yYmrsYP3XYMZr1cGtha3QzmuNB1C7CfyhV'];
    const transactions = await getTransactionsByAddress({ address: addressArray });
    expect(transactions).to.be.an('array');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw if arguments are not valid', async () => {
    const getTransactionsByAddress = getTransactionsByAddressFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getTransactionsByAddress([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getTransactionsByAddress({})).to.be.rejectedWith('should have required property \'address\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(getTransactionsByAddress({ address: 1 })).to.be.rejectedWith('params.address should be array,string');
    expect(spy.callCount).to.be.equal(0);
  });
});
