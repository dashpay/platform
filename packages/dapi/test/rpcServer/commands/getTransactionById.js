const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getTransactionByIdFactory = require('../../../lib/rpcServer/commands/getTransactionById');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getTransactionById', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getTransactionById = getTransactionByIdFactory(coreAPIFixture);
      expect(getTransactionById).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getTransactionById');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(async () => {
    spy.restore();
  });

  it('Should return an object', async () => {
    const getTransactionById = getTransactionByIdFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const transaction = await getTransactionById({ txid: 'beef56cc3cff03a48d078fd7839c05ec16f12f1919ac366596bb5e025f78a2aa' });
    expect(transaction).to.be.an('object');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw if arguments are not valid', async () => {
    const getTransactionById = getTransactionByIdFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getTransactionById([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getTransactionById({})).to.be.rejectedWith('should have required property \'txid\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(getTransactionById({ txid: 1 })).to.be.rejectedWith('txid should be string');
    expect(spy.callCount).to.be.equal(0);
  });
});
